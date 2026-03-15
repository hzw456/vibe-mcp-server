//! Unit tests for the Vibe MCP Server models.

use vibe_mcp_server::{Claims, Task, TaskStatus, User, VerificationCode};

#[cfg(test)]
mod user_model_tests {
    use super::*;
    use serde_json;

    /// Test user email for testing
    const TEST_EMAIL: &str = "testuser@vibe.app";

    fn create_test_user() -> User {
        User {
            id: "test-user-id-001".to_string(),
            email: TEST_EMAIL.to_string(),
            password_hash: "$2a$06$testhash".to_string(),
            created_at: 1700000000000,
            is_verified: false,
        }
    }

    #[test]
    fn test_user_serialization() {
        let user = create_test_user();
        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("testuser@vibe.app"));
        assert!(json.contains("test-user-id-001"));
    }

    #[test]
    fn test_user_deserialization() {
        let json = r#"{
            "id": "user-123",
            "email": "user@example.com",
            "password_hash": "$2a$06$hash",
            "created_at": 1700000000000,
            "is_verified": false
        }"#;

        let user: User = serde_json::from_str(json).unwrap();
        assert_eq!(user.id, "user-123");
        assert_eq!(user.email, "user@example.com");
        assert_eq!(user.is_verified, false);
    }

    #[test]
    fn test_user_clone() {
        let user1 = create_test_user();
        let user2 = user1.clone();
        assert_eq!(user1.id, user2.id);
        assert_eq!(user1.email, user2.email);
    }

    #[test]
    fn test_verified_user() {
        let user = User {
            id: "test-user-id-001".to_string(),
            email: TEST_EMAIL.to_string(),
            password_hash: "$2a$06$testhash".to_string(),
            created_at: 1700000000000,
            is_verified: true,
        };
        assert!(user.is_verified);
    }

    #[test]
    fn test_user_default_fields() {
        let user = User {
            id: "id".to_string(),
            email: "email@test.com".to_string(),
            password_hash: "hash".to_string(),
            created_at: 0,
            is_verified: false,
        };
        assert!(!user.is_verified);
        assert!(!user.id.is_empty());
    }
}

#[cfg(test)]
mod task_model_tests {
    use super::*;

    fn create_test_task() -> Task {
        Task::new(
            "test-task-001".to_string(),
            "test-user-id-001".to_string(),
            "test-task-001".to_string(),
            "VSCode".to_string(),
            "test.py - VSCode".to_string(),
        )
    }

    #[test]
    fn test_task_creation() {
        let task = Task::new(
            "task-001".to_string(),
            "user-001".to_string(),
            "My Task".to_string(),
            "VSCode".to_string(),
            "main.rs - VSCode".to_string(),
        );

        assert_eq!(task.id, "task-001");
        assert_eq!(task.user_id, "user-001");
        assert_eq!(task.name, "My Task");
        assert_eq!(task.status, TaskStatus::Armed);
        assert!(!task.is_focused);
        assert_eq!(task.ide, "VSCode");
    }

    #[test]
    fn test_task_default_values() {
        let task = Task::new(
            "task-002".to_string(),
            "user-001".to_string(),
            "Test Task".to_string(),
            "Cursor".to_string(),
            "test.txt - Cursor".to_string(),
        );

        assert_eq!(task.project_path, None);
        assert_eq!(task.active_file, None);
        assert_eq!(task.end_time, None);
        assert_eq!(task.estimated_duration, None);
        assert_eq!(task.current_stage, None);
        assert_eq!(task.source, "mcp");
        assert_eq!(task.start_time, 0);
    }

    #[test]
    fn test_task_status_transition_armed_to_running() {
        let mut task = create_test_task();
        assert_eq!(task.status, TaskStatus::Armed);

        task.status = TaskStatus::Running;
        assert_eq!(task.status, TaskStatus::Running);
    }

    #[test]
    fn test_task_status_transition_running_to_completed() {
        let mut task = create_test_task();
        task.status = TaskStatus::Running;
        assert_eq!(task.status, TaskStatus::Running);

        task.status = TaskStatus::Completed;
        assert_eq!(task.status, TaskStatus::Completed);
    }

    #[test]
    fn test_task_status_transition_running_to_error() {
        let mut task = create_test_task();
        task.status = TaskStatus::Running;

        task.status = TaskStatus::Error;
        assert_eq!(task.status, TaskStatus::Error);
    }

    #[test]
    fn test_task_status_transition_any_to_cancelled() {
        let statuses = [
            TaskStatus::Armed,
            TaskStatus::Running,
            TaskStatus::Completed,
            TaskStatus::Error,
        ];

        for original_status in statuses {
            let mut task = create_test_task();
            task.status = original_status;
            task.status = TaskStatus::Cancelled;
            assert_eq!(task.status, TaskStatus::Cancelled);
        }
    }

    #[test]
    fn test_task_serialization() {
        let task = create_test_task();
        let json = serde_json::to_string(&task).unwrap();

        assert!(json.contains("test-task-001"));
        assert!(json.contains("test-user-id-001"));
        assert!(json.contains("armed"));
    }

    #[test]
    fn test_task_deserialization() {
        let json = r#"{
            "id": "task-123",
            "user_id": "user-456",
            "name": "Test Task",
            "is_focused": true,
            "ide": "VSCode",
            "window_title": "test.rs",
            "project_path": "/home/user/project",
            "active_file": "test.rs",
            "status": "running",
            "source": "mcp",
            "start_time": 1700000000000,
            "end_time": null,
            "last_heartbeat": 1700000100000,
            "estimated_duration": 3600000,
            "current_stage": "Writing code"
        }"#;

        let task: Task = serde_json::from_str(json).unwrap();
        assert_eq!(task.id, "task-123");
        assert_eq!(task.status, TaskStatus::Running);
        assert_eq!(task.is_focused, true);
        assert!(task.project_path.is_some());
    }
}

#[cfg(test)]
mod verification_code_tests {
    use super::*;

    #[test]
    fn test_verification_code_serialization() {
        let code = VerificationCode {
            code: "ABC123".to_string(),
            expires_at: 1700000600000,
        };
        let json = serde_json::to_string(&code).unwrap();

        assert!(json.contains("ABC123"));
        assert!(json.contains("expires_at"));
    }

    #[test]
    fn test_verification_code_deserialization() {
        let json = r#"{
            "code": "XYZ789",
            "expires_at": 1700000600000
        }"#;

        let code: VerificationCode = serde_json::from_str(json).unwrap();
        assert_eq!(code.code, "XYZ789");
        assert_eq!(code.expires_at, 1700000600000);
    }
}

#[cfg(test)]
mod claims_tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_claims_creation() {
        let claims = Claims {
            sub: "test-user-id-001".to_string(),
            email: "testuser@vibe.app".to_string(),
            exp: 1700086400,
            iat: 1700000000,
        };

        assert_eq!(claims.sub, "test-user-id-001");
        assert_eq!(claims.email, "testuser@vibe.app");
        assert!(claims.exp > 0);
        assert!(claims.iat > 0);
    }

    #[test]
    fn test_claims_serialization() {
        let claims = Claims {
            sub: "test-user-id-001".to_string(),
            email: "testuser@vibe.app".to_string(),
            exp: 1700086400,
            iat: 1700000000,
        };
        let json = serde_json::to_string(&claims).unwrap();

        assert!(json.contains("testuser@vibe.app"));
        assert!(json.contains("sub"));
        assert!(json.contains("exp"));
        assert!(json.contains("iat"));
    }

    #[test]
    fn test_claims_deserialization() {
        let json = r#"{
            "sub": "user-123",
            "email": "test@example.com",
            "exp": 1700086400,
            "iat": 1700000000
        }"#;

        let claims: Claims = serde_json::from_str(json).unwrap();
        assert_eq!(claims.sub, "user-123");
        assert_eq!(claims.email, "test@example.com");
        assert_eq!(claims.exp, 1700086400);
        assert_eq!(claims.iat, 1700000000);
    }

    #[test]
    fn test_claims_ordering() {
        let claims = Claims {
            sub: "user".to_string(),
            email: "user@test.com".to_string(),
            exp: 1700086400,
            iat: 1700000000,
        };

        // iat should be less than exp (token not expired at creation)
        assert!(claims.iat < claims.exp);
    }
}
