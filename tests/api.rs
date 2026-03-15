//! API integration tests for the Vibe MCP Server.

use vibe_mcp_server::models::TaskStatus;
use vibe_mcp_server::services::{create_jwt_token, decode_jwt_token, generate_verification_code};
use vibe_mcp_server::utils::helpers::validate_email;
use vibe_mcp_server::{hash_password, Config, TaskService, User, UserService};

const TEST_EMAIL: &str = "testuser@vibe.app";
const TEST_PASSWORD: &str = "testpass123";
const TEST_JWT_SECRET: &str = "test-secret";

fn create_test_config() -> Config {
    Config {
        host: "0.0.0.0".to_string(),
        port: 3010,
        api_key: "test-api-key".to_string(),
        jwt_secret: TEST_JWT_SECRET.to_string(),
        jwt_expiry_hours: 24,
    }
}

fn create_user_service() -> UserService {
    UserService::new(String::new())
}

fn create_task_service() -> TaskService {
    TaskService::new(String::new())
}

#[cfg(test)]
mod auth_flow_tests {
    use super::*;

    #[test]
    fn test_email_validation_valid() {
        assert!(validate_email("user@example.com"));
        assert!(validate_email("user.name@example.co.uk"));
        assert!(validate_email("user+tag@example.org"));
        assert!(validate_email("testuser@vibe.app"));
    }

    #[test]
    fn test_email_validation_invalid() {
        assert!(!validate_email("userexample.com")); // No @
        assert!(!validate_email("user@")); // No domain
        assert!(!validate_email("@example.com")); // No local part
        assert!(!validate_email("user@.com")); // No domain name
        assert!(!validate_email("a@b")); // Too short
        assert!(!validate_email("")); // Empty
    }

    #[test]
    fn test_full_auth_flow_register_verify_login() {
        let user_service = create_user_service();
        let config = create_test_config();

        // 1. Register user
        let password_hash = hash_password(TEST_PASSWORD).unwrap();
        let user = user_service.create_user(TEST_EMAIL, &password_hash);

        assert!(!user.is_verified);

        // 2. Send verification code
        let code = generate_verification_code();
        user_service.save_verification_code(TEST_EMAIL, &code, 10);

        // 3. Verify code
        assert!(user_service.verify_code(TEST_EMAIL, &code));

        // 4. Mark user as verified
        user_service.set_user_verified(TEST_EMAIL);
        let verified_user = user_service.find_user_by_email(TEST_EMAIL).unwrap();
        assert!(verified_user.is_verified);

        // 5. Login
        let token = create_jwt_token(&verified_user, &config).unwrap();
        assert!(!token.is_empty());

        // 6. Token should be valid
        let claims = decode_jwt_token(&token, &config).unwrap();
        assert_eq!(claims.email, TEST_EMAIL);
    }

    #[test]
    fn test_auth_flow_registration_duplicate_email() {
        let user_service = create_user_service();

        let password_hash = hash_password(TEST_PASSWORD).unwrap();
        user_service.create_user(TEST_EMAIL, &password_hash);

        // Try to register same email again
        let existing_user = user_service.find_user_by_email(TEST_EMAIL);
        assert!(existing_user.is_some());
    }

    #[test]
    fn test_auth_flow_invalid_verification_code() {
        let user_service = create_user_service();

        let password_hash = hash_password(TEST_PASSWORD).unwrap();
        user_service.create_user(TEST_EMAIL, &password_hash);

        let code = generate_verification_code();
        user_service.save_verification_code(TEST_EMAIL, &code, 10);

        // Try wrong code
        assert!(!user_service.verify_code(TEST_EMAIL, "WRONG1"));

        // Original code should still work
        assert!(user_service.verify_code(TEST_EMAIL, &code));
    }
}

#[cfg(test)]
mod task_crud_tests {
    use super::*;

    fn create_test_setup() -> (UserService, TaskService, Config) {
        let user_service = create_user_service();
        let task_service = create_task_service();
        let config = create_test_config();

        // Create a test user
        let password_hash = hash_password(TEST_PASSWORD).unwrap();
        user_service.create_user(TEST_EMAIL, &password_hash);

        (user_service, task_service, config)
    }

    #[test]
    fn test_task_creation_workflow() {
        let (_, task_service, _) = create_test_setup();

        // Create initial task
        let result = task_service.update_task_status(
            "test-task-001",
            Some("armed"),
            Some("mcp"),
            None,
            None,
            TEST_EMAIL,
        );
        assert!(result.is_ok());

        let tasks = task_service.get_tasks(Some(TEST_EMAIL));
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "test-task-001");
        assert_eq!(tasks[0].status, TaskStatus::Armed);
    }

    #[test]
    fn test_task_status_workflow() {
        let (_, task_service, _) = create_test_setup();

        // Task lifecycle: Armed -> Running -> Completed
        let result1 = task_service.update_task_status(
            "test-task-002",
            Some("armed"),
            Some("mcp"),
            None,
            None,
            TEST_EMAIL,
        );
        assert!(result1.is_ok());

        // Start task
        let result2 = task_service.update_task_status(
            "test-task-002",
            Some("running"),
            Some("mcp"),
            None,
            None,
            TEST_EMAIL,
        );
        assert!(result2.is_ok());

        // Update progress
        let result3 = task_service.update_task_progress(
            "test-task-002",
            Some(300000),
            Some("Implementing feature"),
            TEST_EMAIL,
        );
        assert!(result3.is_ok());

        // Complete task
        let result4 = task_service.update_task_status(
            "test-task-002",
            Some("completed"),
            Some("mcp"),
            None,
            None,
            TEST_EMAIL,
        );
        assert!(result4.is_ok());

        let tasks = task_service.get_tasks(Some(TEST_EMAIL));
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].status, TaskStatus::Completed);
        assert_eq!(tasks[0].current_stage, Some("__completed__".to_string()));
    }

    #[test]
    fn test_task_error_handling() {
        let (_, task_service, _) = create_test_setup();

        let result1 = task_service.update_task_status(
            "test-task-003",
            Some("running"),
            Some("mcp"),
            None,
            None,
            TEST_EMAIL,
        );
        assert!(result1.is_ok());

        // Simulate error
        let result2 = task_service.update_task_status(
            "test-task-003",
            Some("error"),
            Some("mcp"),
            None,
            None,
            TEST_EMAIL,
        );
        assert!(result2.is_ok());

        let tasks = task_service.get_tasks(Some(TEST_EMAIL));
        assert_eq!(tasks[0].status, TaskStatus::Error);
    }

    #[test]
    fn test_task_cancellation() {
        let (_, task_service, _) = create_test_setup();

        let result1 = task_service.update_task_status(
            "test-task-004",
            Some("armed"),
            Some("mcp"),
            None,
            None,
            TEST_EMAIL,
        );
        assert!(result1.is_ok());

        // Cancel task
        let result2 = task_service.update_task_status(
            "test-task-004",
            Some("cancelled"),
            Some("mcp"),
            None,
            None,
            TEST_EMAIL,
        );
        assert!(result2.is_ok());

        let tasks = task_service.get_tasks(Some(TEST_EMAIL));
        assert_eq!(tasks[0].status, TaskStatus::Cancelled);
    }

    #[test]
    fn test_multiple_tasks_isolation() {
        let user1_service = create_user_service();
        let user2_service = create_user_service();
        let task_service = create_task_service();

        let password_hash = hash_password(TEST_PASSWORD).unwrap();
        user1_service.create_user("user1@test.com", &password_hash);
        user2_service.create_user("user2@test.com", &password_hash);

        // Create tasks for different users
        let _ = task_service.update_task_status(
            "task-1",
            Some("running"),
            Some("mcp"),
            None,
            None,
            "user1@test.com",
        );

        let _ = task_service.update_task_status(
            "task-2",
            Some("running"),
            Some("mcp"),
            None,
            None,
            "user2@test.com",
        );

        // Each user sees only their tasks
        let user1_tasks = task_service.get_tasks(Some("user1@test.com"));
        let user2_tasks = task_service.get_tasks(Some("user2@test.com"));

        assert_eq!(user1_tasks.len(), 1);
        assert_eq!(user1_tasks[0].id, "task-1");
        assert_eq!(user2_tasks.len(), 1);
        assert_eq!(user2_tasks[0].id, "task-2");
    }

    #[test]
    fn test_task_deletion() {
        let (_, task_service, _) = create_test_setup();

        let _ = task_service.update_task_status(
            "task-to-delete",
            Some("running"),
            Some("mcp"),
            None,
            None,
            TEST_EMAIL,
        );

        assert_eq!(task_service.get_tasks(Some(TEST_EMAIL)).len(), 1);

        let deleted = task_service.delete_task("task-to-delete", TEST_EMAIL);
        assert!(deleted);
        assert!(task_service.get_tasks(Some(TEST_EMAIL)).is_empty());
    }

    #[test]
    fn test_task_reset() {
        let (_, task_service, _) = create_test_setup();

        // Create multiple tasks
        let _ = task_service.update_task_status(
            "task-1",
            Some("running"),
            Some("mcp"),
            None,
            None,
            TEST_EMAIL,
        );
        let _ = task_service.update_task_status(
            "task-2",
            Some("completed"),
            Some("mcp"),
            None,
            None,
            TEST_EMAIL,
        );
        let _ = task_service.update_task_status(
            "task-3",
            Some("running"),
            Some("mcp"),
            None,
            None,
            TEST_EMAIL,
        );

        // Reset all tasks
        task_service.reset_tasks(None, TEST_EMAIL);

        assert!(task_service.get_tasks(Some(TEST_EMAIL)).is_empty());
    }

    #[test]
    fn test_multiple_task_creation() {
        let (_, task_service, _) = create_test_setup();

        for i in 1..=3 {
            let result = task_service.update_task_status(
                &format!("test-task-00{}", i),
                Some("armed"),
                Some("mcp"),
                None,
                None,
                TEST_EMAIL,
            );
            assert!(result.is_ok());
        }

        let tasks = task_service.get_tasks(Some(TEST_EMAIL));
        assert_eq!(tasks.len(), 3);
    }
}

#[cfg(test)]
mod error_handling_tests {
    use super::*;

    fn create_test_setup() -> (UserService, TaskService, Config) {
        let user_service = create_user_service();
        let task_service = create_task_service();
        let config = create_test_config();

        let password_hash = hash_password(TEST_PASSWORD).unwrap();
        user_service.create_user(TEST_EMAIL, &password_hash);

        (user_service, task_service, config)
    }

    #[test]
    fn test_invalid_status_error() {
        let (_, task_service, _) = create_test_setup();

        let result = task_service.update_task_status(
            "invalid-task",
            Some("invalid_status_value"),
            Some("mcp"),
            None,
            None,
            TEST_EMAIL,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_nonexistent_task_progress_error() {
        let (_, task_service, _) = create_test_setup();

        let result = task_service.update_task_progress(
            "nonexistent-task-id",
            Some(1000),
            Some("stage"),
            TEST_EMAIL,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_user_task_access() {
        let user1_service = create_user_service();
        let user2_service = create_user_service();
        let task_service = create_task_service();

        let password_hash = hash_password(TEST_PASSWORD).unwrap();
        user1_service.create_user("user1@test.com", &password_hash);
        user2_service.create_user("user2@test.com", &password_hash);

        // User 1 creates a task
        let _ = task_service.update_task_status(
            "shared-task-id",
            Some("running"),
            Some("mcp"),
            None,
            None,
            "user1@test.com",
        );

        // User 2 tries to access User 1's task
        let tasks = task_service.get_tasks(Some("user2@test.com"));
        assert!(tasks.is_empty());

        // User 2 tries to update User 1's task
        let result = task_service.update_task_progress(
            "shared-task-id",
            Some(1000),
            Some("stage"),
            "user2@test.com",
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_nonexistent_task() {
        let (_, task_service, _) = create_test_setup();

        let result = task_service.delete_task("nonexistent-id", TEST_EMAIL);
        assert!(!result);
    }
}

#[cfg(test)]
mod data_consistency_tests {
    use super::*;

    fn create_test_setup() -> (UserService, TaskService, Config) {
        let user_service = create_user_service();
        let task_service = create_task_service();
        let config = create_test_config();

        let password_hash = hash_password(TEST_PASSWORD).unwrap();
        user_service.create_user(TEST_EMAIL, &password_hash);

        (user_service, task_service, config)
    }

    #[test]
    fn test_user_data_persistence_during_operations() {
        let user_service = create_user_service();
        let task_service = create_task_service();
        let config = create_test_config();

        // Create user
        let password_hash = hash_password(TEST_PASSWORD).unwrap();
        let user = user_service.create_user(TEST_EMAIL, &password_hash);
        let user_id = user.id.clone();

        // Create tasks
        for i in 1..=5 {
            let _ = task_service.update_task_status(
                &format!("task-{}", i),
                Some("running"),
                Some("mcp"),
                None,
                None,
                TEST_EMAIL,
            );
        }

        // User should still exist
        let found_user = user_service.find_user_by_id(&user_id);
        assert!(found_user.is_some());
        assert_eq!(found_user.unwrap().email, TEST_EMAIL);

        // Tasks should still exist
        let tasks = task_service.get_tasks(Some(TEST_EMAIL));
        assert_eq!(tasks.len(), 5);
    }

    #[test]
    fn test_multiple_status_updates_data_integrity() {
        let (_, task_service, _) = create_test_setup();

        let _ = task_service.update_task_status(
            "persistent-task",
            Some("armed"),
            Some("mcp"),
            Some(60000),
            Some("Initializing"),
            TEST_EMAIL,
        );

        for _ in 0..10 {
            let _ = task_service.update_task_status(
                "persistent-task",
                Some("running"),
                Some("mcp"),
                Some(60000),
                Some("Processing"),
                TEST_EMAIL,
            );
        }

        let tasks = task_service.get_tasks(Some(TEST_EMAIL));
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].status, TaskStatus::Running);
        assert_eq!(tasks[0].current_stage, Some("Processing".to_string()));
    }

    #[test]
    fn test_service_clone_isolation() {
        let service1 = create_task_service();
        let service2 = service1.clone();

        let _ = service1.update_task_status(
            "task-1",
            Some("running"),
            Some("mcp"),
            None,
            None,
            TEST_EMAIL,
        );

        // service2 should see the task (Arc clone shares data)
        let tasks = service2.get_tasks(Some(TEST_EMAIL));
        assert_eq!(tasks.len(), 1);
    }
}
