pub mod approvals;
pub mod events;
pub mod server;

pub use approvals::{ApprovalDecision, ApprovalError, ApprovalManager, PendingApproval};
pub use events::ServerEvent;
pub use server::{DaemonServer, ServerConfig, ServerError};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_token_auth() {
        let config = ServerConfig {
            auth_token: "test-token-123".to_string(),
            ..Default::default()
        };
        let server = DaemonServer::new(config);

        // Valid tokens
        assert!(server.verify_token(Some("test-token-123")).is_ok());
        assert!(server.verify_token(Some("Bearer test-token-123")).is_ok());

        // Invalid tokens
        assert!(matches!(
            server.verify_token(Some("wrong")),
            Err(ServerError::Unauthorized)
        ));
        assert!(matches!(
            server.verify_token(None),
            Err(ServerError::Unauthorized)
        ));
    }

    #[test]
    fn test_approval_request_bound_and_remote_merge_protection() {
        // By default, allow_remote_merge is false
        let config = ServerConfig {
            allow_remote_merge: false,
            ..Default::default()
        };
        let server = DaemonServer::new(config);

        // Non-merge approval passes
        let (req1, _) = server.request_approval("T-0001", "run db migration", false);
        let res = server.submit_approval(
            Some("secret-a4l-token"),
            &req1.request_id,
            ApprovalDecision::Approve,
        );
        assert!(res.is_ok());

        // Merge approval rejected because allow_remote_merge is false
        let (req2, _) = server.request_approval("T-0001", "merge to main", true);
        let res2 = server.submit_approval(
            Some("secret-a4l-token"),
            &req2.request_id,
            ApprovalDecision::Approve,
        );
        assert!(matches!(
            res2,
            Err(ServerError::Approval(ApprovalError::RemoteMergeDisabled))
        ));
    }

    #[test]
    fn test_approval_timeout_denies() {
        // Set timeout to 0 seconds so it immediately expires
        let mgr = ApprovalManager::new(-1, true);
        let req = mgr.create_request("T-0001", "reboot container", false);

        let res = mgr.resolve(&req.request_id, ApprovalDecision::Approve);
        assert!(matches!(res, Err(ApprovalError::Expired(_))));
    }
}
