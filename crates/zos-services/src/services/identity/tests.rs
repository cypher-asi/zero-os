//! Unit tests for IdentityService
//!
//! These tests verify the internal logic of the identity service
//! without requiring actual IPC or VFS communication.

#[cfg(test)]
mod tests {
    use crate::services::identity::pending::{PendingStorageOp, RequestContext};
    use crate::services::identity::IdentityService;
    use alloc::vec;

    // =========================================================================
    // RequestContext tests
    // =========================================================================

    #[test]
    fn test_request_context_new() {
        let ctx = RequestContext::new(42, vec![1, 2, 3]);

        assert_eq!(ctx.client_pid, 42);
        assert_eq!(ctx.cap_slots, vec![1, 2, 3]);
    }

    #[test]
    fn test_request_context_clone() {
        let ctx1 = RequestContext::new(100, vec![5, 10]);
        let ctx2 = ctx1.clone();

        assert_eq!(ctx2.client_pid, 100);
        assert_eq!(ctx2.cap_slots, vec![5, 10]);
    }

    #[test]
    fn test_request_context_empty_cap_slots() {
        let ctx = RequestContext::new(1, vec![]);

        assert_eq!(ctx.client_pid, 1);
        assert!(ctx.cap_slots.is_empty());
    }

    // =========================================================================
    // IdentityService initialization tests
    // =========================================================================

    #[test]
    fn test_identity_service_default() {
        let service = IdentityService::default();

        assert!(!service.registered);
        assert!(service.pending_vfs_ops.is_empty());
        assert_eq!(service.next_vfs_op_id, 0);
        assert!(service.pending_net_ops.is_empty());
    }

    // =========================================================================
    // Pending VFS operations FIFO order tests
    // =========================================================================

    #[test]
    fn test_pending_vfs_ops_insertion_order() {
        let mut service = IdentityService::default();

        // Insert operations with IDs 5, 3, 7 (out of order)
        service.pending_vfs_ops.insert(
            5,
            PendingStorageOp::GetIdentityKey {
                ctx: RequestContext::new(1, vec![]),
            },
        );
        service.pending_vfs_ops.insert(
            3,
            PendingStorageOp::GetIdentityKey {
                ctx: RequestContext::new(2, vec![]),
            },
        );
        service.pending_vfs_ops.insert(
            7,
            PendingStorageOp::GetIdentityKey {
                ctx: RequestContext::new(3, vec![]),
            },
        );

        // BTreeMap keys are ordered, so iteration should be 3, 5, 7
        let keys: Vec<_> = service.pending_vfs_ops.keys().copied().collect();
        assert_eq!(keys, vec![3, 5, 7]);
    }

    #[test]
    fn test_pending_vfs_ops_fifo_removal() {
        let mut service = IdentityService::default();

        // Insert operations in order
        service.pending_vfs_ops.insert(
            1,
            PendingStorageOp::GetIdentityKey {
                ctx: RequestContext::new(10, vec![]),
            },
        );
        service.pending_vfs_ops.insert(
            2,
            PendingStorageOp::GetIdentityKey {
                ctx: RequestContext::new(20, vec![]),
            },
        );
        service.pending_vfs_ops.insert(
            3,
            PendingStorageOp::GetIdentityKey {
                ctx: RequestContext::new(30, vec![]),
            },
        );

        // Remove first (smallest key)
        let first_key = *service.pending_vfs_ops.keys().next().unwrap();
        let first_op = service.pending_vfs_ops.remove(&first_key).unwrap();

        // Verify we got the first one (ID 1, client_pid 10)
        if let PendingStorageOp::GetIdentityKey { ctx } = first_op {
            assert_eq!(ctx.client_pid, 10);
        } else {
            panic!("Expected GetIdentityKey");
        }

        // Remaining should be 2 and 3
        let remaining_keys: Vec<_> = service.pending_vfs_ops.keys().copied().collect();
        assert_eq!(remaining_keys, vec![2, 3]);
    }

    #[test]
    fn test_next_vfs_op_id_increments() {
        let mut service = IdentityService::default();

        assert_eq!(service.next_vfs_op_id, 0);
        service.next_vfs_op_id += 1;
        assert_eq!(service.next_vfs_op_id, 1);
        service.next_vfs_op_id += 1;
        assert_eq!(service.next_vfs_op_id, 2);
    }

    // =========================================================================
    // Pending network operations tests
    // =========================================================================

    #[test]
    fn test_pending_net_ops_empty_initially() {
        let service = IdentityService::default();
        assert!(service.pending_net_ops.is_empty());
    }

    #[test]
    fn test_pending_net_ops_keyed_by_request_id() {
        use crate::services::identity::pending::PendingNetworkOp;

        let mut service = IdentityService::default();

        // Network ops are keyed by request_id from syscall
        service.pending_net_ops.insert(
            42,
            PendingNetworkOp::SubmitZidLogin {
                ctx: RequestContext::new(1, vec![]),
                user_id: 12345,
                zid_endpoint: "https://example.com".into(),
            },
        );

        assert!(service.pending_net_ops.contains_key(&42));
        assert!(!service.pending_net_ops.contains_key(&99));
    }
}
