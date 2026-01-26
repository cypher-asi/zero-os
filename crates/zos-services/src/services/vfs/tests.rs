//! Unit tests for VfsService
//!
//! These tests cover internal state behavior without invoking syscalls.
//!
//! # Test Categories (Rule 13)
//!
//! - Path validation edge cases
//! - Permission denial scenarios
//! - Corrupt/invalid data handling
//! - Unexpected storage result types
//! - Resource limits

#[cfg(test)]
mod tests {
    use crate::services::vfs::{ClientContext, InodeOpType, PendingOp, VfsService, validate_path, MAX_PENDING_OPS};
    use alloc::string::String;
    use alloc::vec::Vec;
    use zos_vfs::service::{PermissionContext, ProcessClass};

    fn make_test_client_ctx(pid: u32) -> ClientContext {
        ClientContext {
            pid,
            reply_caps: Vec::new(),
        }
    }

    fn make_test_perm_ctx() -> PermissionContext {
        PermissionContext {
            user_id: None,
            process_class: ProcessClass::System,
        }
    }

    #[test]
    fn test_vfs_service_default() {
        let service = VfsService::default();
        assert!(!service.registered);
        assert!(service.pending_ops.is_empty());
    }

    #[test]
    fn test_pending_ops_ordering() {
        let mut service = VfsService::default();

        service.pending_ops.insert(
            5,
            PendingOp::GetInode {
                ctx: make_test_client_ctx(1),
                path: String::from("/tmp/a"),
                op_type: InodeOpType::Stat,
                perm_ctx: make_test_perm_ctx(),
            },
        );
        service.pending_ops.insert(
            2,
            PendingOp::ExistsCheck {
                ctx: make_test_client_ctx(2),
                path: String::from("/tmp/b"),
            },
        );
        service.pending_ops.insert(
            9,
            PendingOp::GetContent {
                ctx: make_test_client_ctx(3),
                path: String::from("/tmp/c"),
                perm_ctx: make_test_perm_ctx(),
            },
        );

        let keys: Vec<u32> = service.pending_ops.keys().copied().collect();
        assert_eq!(keys, vec![2, 5, 9]);
    }

    #[test]
    fn test_pending_op_insert_and_remove() {
        let mut service = VfsService::default();

        service.pending_ops.insert(
            1,
            PendingOp::PutContent {
                ctx: make_test_client_ctx(10),
                path: String::from("/tmp/file"),
            },
        );

        let op = service.pending_ops.remove(&1).expect("pending op should exist");
        match op {
            PendingOp::PutContent { ctx, path } => {
                assert_eq!(ctx.pid, 10);
                assert_eq!(path, "/tmp/file");
            }
            _ => panic!("expected PutContent"),
        }

        assert!(service.pending_ops.is_empty());
    }

    #[test]
    fn test_validate_path_valid() {
        assert!(validate_path("/").is_ok());
        assert!(validate_path("/tmp").is_ok());
        assert!(validate_path("/tmp/file.txt").is_ok());
        assert!(validate_path("/users/123/home").is_ok());
        assert!(validate_path("/a/b/c/d/e").is_ok());
    }

    #[test]
    fn test_validate_path_empty() {
        assert!(validate_path("").is_err());
    }

    #[test]
    fn test_validate_path_not_absolute() {
        assert!(validate_path("tmp").is_err());
        assert!(validate_path("tmp/file").is_err());
        assert!(validate_path("./file").is_err());
    }

    #[test]
    fn test_validate_path_traversal() {
        assert!(validate_path("/tmp/../etc").is_err());
        assert!(validate_path("/..").is_err());
        assert!(validate_path("/tmp/..").is_err());
    }

    #[test]
    fn test_validate_path_null_byte() {
        assert!(validate_path("/tmp/\0file").is_err());
    }

    #[test]
    fn test_pending_op_put_inode_with_context() {
        let mut service = VfsService::default();

        // With context (should send response)
        service.pending_ops.insert(
            1,
            PendingOp::PutInode {
                ctx: Some(make_test_client_ctx(5)),
                response_tag: 0x8001,
            },
        );

        let op = service.pending_ops.remove(&1).expect("pending op should exist");
        match op {
            PendingOp::PutInode { ctx, response_tag } => {
                assert!(ctx.is_some());
                assert_eq!(ctx.unwrap().pid, 5);
                assert_eq!(response_tag, 0x8001);
            }
            _ => panic!("expected PutInode"),
        }
    }

    #[test]
    fn test_pending_op_put_inode_without_context() {
        let mut service = VfsService::default();

        // Without context (intermediate step, no response)
        service.pending_ops.insert(
            2,
            PendingOp::PutInode {
                ctx: None,
                response_tag: 0,
            },
        );

        let op = service.pending_ops.remove(&2).expect("pending op should exist");
        match op {
            PendingOp::PutInode { ctx, response_tag } => {
                assert!(ctx.is_none());
                assert_eq!(response_tag, 0);
            }
            _ => panic!("expected PutInode"),
        }
    }

    // =========================================================================
    // Path Edge Cases (Rule 13)
    // =========================================================================

    #[test]
    fn test_validate_path_trailing_slash_rejected() {
        // Trailing slash should be rejected (except root)
        assert!(validate_path("/tmp/").is_err());
        assert!(validate_path("/a/b/c/").is_err());
        assert!(validate_path("/users/123/").is_err());
        
        // Root is allowed
        assert!(validate_path("/").is_ok());
    }

    #[test]
    fn test_validate_path_double_slash_rejected() {
        assert!(validate_path("/tmp//file").is_err());
        assert!(validate_path("//tmp").is_err());
        assert!(validate_path("/a//b//c").is_err());
    }

    #[test]
    fn test_validate_path_dot_components_rejected() {
        // Single dot
        assert!(validate_path("/./file").is_err());
        assert!(validate_path("/tmp/./file").is_err());
        
        // Double dot (traversal)
        assert!(validate_path("/../file").is_err());
        assert!(validate_path("/tmp/../etc").is_err());
    }

    #[test]
    fn test_validate_path_special_characters_allowed() {
        // These should all be valid
        assert!(validate_path("/tmp/file-name").is_ok());
        assert!(validate_path("/tmp/file_name").is_ok());
        assert!(validate_path("/tmp/file.txt").is_ok());
        assert!(validate_path("/tmp/file name").is_ok()); // spaces allowed
        assert!(validate_path("/tmp/file@host").is_ok());
    }

    // =========================================================================
    // Permission Context Tests (Rule 13)
    // =========================================================================

    #[test]
    fn test_permission_context_system_process() {
        let perm_ctx = PermissionContext {
            user_id: None,
            process_class: ProcessClass::System,
        };
        assert!(matches!(perm_ctx.process_class, ProcessClass::System));
        assert!(perm_ctx.user_id.is_none());
    }

    #[test]
    fn test_permission_context_application_with_user() {
        let perm_ctx = PermissionContext {
            user_id: Some(12345),
            process_class: ProcessClass::Application,
        };
        assert!(matches!(perm_ctx.process_class, ProcessClass::Application));
        assert_eq!(perm_ctx.user_id, Some(12345));
    }

    // =========================================================================
    // Resource Limit Tests (Rule 11)
    // =========================================================================

    #[test]
    fn test_max_pending_ops_constant() {
        // Ensure the constant is reasonable
        assert!(MAX_PENDING_OPS > 0);
        assert!(MAX_PENDING_OPS <= 10000); // sanity check
    }

    #[test]
    fn test_max_content_size_constant() {
        use crate::services::vfs::MAX_CONTENT_SIZE;
        
        // Ensure the constant is reasonable (16 MB)
        assert!(MAX_CONTENT_SIZE > 0);
        assert_eq!(MAX_CONTENT_SIZE, 16 * 1024 * 1024); // exactly 16 MB
    }

    #[test]
    fn test_pending_ops_map_can_hold_multiple() {
        let mut service = VfsService::default();
        
        // Insert multiple operations
        for i in 0..10 {
            service.pending_ops.insert(
                i,
                PendingOp::ExistsCheck {
                    ctx: make_test_client_ctx(i),
                    path: format!("/tmp/file{}", i),
                },
            );
        }
        
        assert_eq!(service.pending_ops.len(), 10);
    }

    // =========================================================================
    // State Machine Stage Tests (Rule 13)
    // =========================================================================

    #[test]
    fn test_write_file_stage_variants() {
        use crate::services::vfs::WriteFileStage;
        
        let stage1 = WriteFileStage::CheckingParent {
            content: vec![1, 2, 3],
        };
        let stage2 = WriteFileStage::WritingContent { content_len: 100 };
        let stage3 = WriteFileStage::WritingInode;
        
        // Verify we can clone stages
        let _cloned = stage1.clone();
        let _cloned = stage2.clone();
        let _cloned = stage3.clone();
    }

    #[test]
    fn test_mkdir_stage_variants() {
        use crate::services::vfs::MkdirStage;
        
        let stage1 = MkdirStage::CheckingExists;
        let stage2 = MkdirStage::CheckingParent;
        let stage3 = MkdirStage::WritingInode;
        
        // Verify we can clone stages
        let _cloned = stage1.clone();
        let _cloned = stage2.clone();
        let _cloned = stage3.clone();
    }

    #[test]
    fn test_readdir_stage_variants() {
        use crate::services::vfs::ReaddirStage;
        
        let stage1 = ReaddirStage::ReadingInode;
        let stage2 = ReaddirStage::ListingChildren;
        
        // Verify we can clone stages
        let _cloned = stage1.clone();
        let _cloned = stage2.clone();
    }

    #[test]
    fn test_unlink_stage_variants() {
        use crate::services::vfs::UnlinkStage;
        
        let stage1 = UnlinkStage::ReadingInode;
        let stage2 = UnlinkStage::DeletingContent;
        let stage3 = UnlinkStage::DeletingInode;
        
        // Verify we can clone stages
        let _cloned = stage1.clone();
        let _cloned = stage2.clone();
        let _cloned = stage3.clone();
    }

    // =========================================================================
    // Pending Operation Variants (Rule 13)
    // =========================================================================

    #[test]
    fn test_pending_op_mkdir_op() {
        use crate::services::vfs::MkdirStage;
        
        let mut service = VfsService::default();
        
        service.pending_ops.insert(
            1,
            PendingOp::MkdirOp {
                ctx: make_test_client_ctx(10),
                path: String::from("/tmp/newdir"),
                perm_ctx: make_test_perm_ctx(),
                stage: MkdirStage::CheckingExists,
            },
        );
        
        let op = service.pending_ops.remove(&1).expect("pending op should exist");
        match op {
            PendingOp::MkdirOp { ctx, path, stage, .. } => {
                assert_eq!(ctx.pid, 10);
                assert_eq!(path, "/tmp/newdir");
                assert!(matches!(stage, MkdirStage::CheckingExists));
            }
            _ => panic!("expected MkdirOp"),
        }
    }

    #[test]
    fn test_pending_op_readdir_op() {
        use crate::services::vfs::ReaddirStage;
        
        let mut service = VfsService::default();
        
        service.pending_ops.insert(
            1,
            PendingOp::ReaddirOp {
                ctx: make_test_client_ctx(10),
                path: String::from("/tmp"),
                perm_ctx: make_test_perm_ctx(),
                stage: ReaddirStage::ReadingInode,
            },
        );
        
        let op = service.pending_ops.remove(&1).expect("pending op should exist");
        match op {
            PendingOp::ReaddirOp { ctx, path, stage, .. } => {
                assert_eq!(ctx.pid, 10);
                assert_eq!(path, "/tmp");
                assert!(matches!(stage, ReaddirStage::ReadingInode));
            }
            _ => panic!("expected ReaddirOp"),
        }
    }

    #[test]
    fn test_pending_op_unlink_op() {
        use crate::services::vfs::UnlinkStage;
        
        let mut service = VfsService::default();
        
        service.pending_ops.insert(
            1,
            PendingOp::UnlinkOp {
                ctx: make_test_client_ctx(10),
                path: String::from("/tmp/file"),
                perm_ctx: make_test_perm_ctx(),
                stage: UnlinkStage::ReadingInode,
            },
        );
        
        let op = service.pending_ops.remove(&1).expect("pending op should exist");
        match op {
            PendingOp::UnlinkOp { ctx, path, stage, .. } => {
                assert_eq!(ctx.pid, 10);
                assert_eq!(path, "/tmp/file");
                assert!(matches!(stage, UnlinkStage::ReadingInode));
            }
            _ => panic!("expected UnlinkOp"),
        }
    }

    #[test]
    fn test_pending_op_write_file_op() {
        use crate::services::vfs::WriteFileStage;
        
        let mut service = VfsService::default();
        
        service.pending_ops.insert(
            1,
            PendingOp::WriteFileOp {
                ctx: make_test_client_ctx(10),
                path: String::from("/tmp/file"),
                perm_ctx: make_test_perm_ctx(),
                stage: WriteFileStage::CheckingParent {
                    content: vec![1, 2, 3, 4],
                },
            },
        );
        
        let op = service.pending_ops.remove(&1).expect("pending op should exist");
        match op {
            PendingOp::WriteFileOp { ctx, path, stage, .. } => {
                assert_eq!(ctx.pid, 10);
                assert_eq!(path, "/tmp/file");
                match stage {
                    WriteFileStage::CheckingParent { content } => {
                        assert_eq!(content, vec![1, 2, 3, 4]);
                    }
                    _ => panic!("expected CheckingParent stage"),
                }
            }
            _ => panic!("expected WriteFileOp"),
        }
    }
}
