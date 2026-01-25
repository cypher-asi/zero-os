//! Unit tests for VfsService
//!
//! These tests cover internal state behavior without invoking syscalls.

#[cfg(test)]
mod tests {
    use crate::services::vfs::{ClientContext, InodeOpType, PendingOp, VfsService, validate_path};
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
}
