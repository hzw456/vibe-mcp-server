//! Unit tests for the Vibe MCP Server services.

use vibe_mcp_server::models::TaskStatus;
use vibe_mcp_server::services::{
    create_jwt_token, generate_verification_code, hash_password, verify_password,
};
use vibe_mcp_server::{Config, TaskService, User, UserService};

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

fn create_test_user() -> User {
    User {
        id: "test-user-id-001".to_string(),
        email: TEST_EMAIL.to_string(),
        password_hash: "$2a$06$testhash".to_string(),
        created_at: 1700000000000,
        is_verified: false,
    }
}

fn create_verified_user() -> User {
    User {
        id: "test-user-id-001".to_string(),
        email: TEST_EMAIL.to_string(),
        password_hash: "$2a$06$testhash".to_string(),
        created_at: 1700000000000,
        is_verified: true,
    }
}

#[cfg(test)]
mod password_service_tests {
    use super::*;

    #[test]
    fn test_hash_password() {
        let password = TEST_PASSWORD;
        let hash = hash_password(password).unwrap();

        // BCrypt hashes start with $2a$ or $2b$
        assert!(hash.starts_with("$2a$") || hash.starts_with("$2b$"));
        assert_ne!(hash, password);
    }

    #[test]
    fn test_hash_password_different_for_same_input() {
        let password = TEST_PASSWORD;
        let hash1 = hash_password(password).unwrap();
        let hash2 = hash_password(password).unwrap();

        // Due to random salt, hashes should be different
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_verify_password_correct() {
        let password = TEST_PASSWORD;
        let hash = hash_password(password).unwrap();

        let result = verify_password(password, &hash).unwrap();
        assert!(result);
    }

    #[test]
    fn test_verify_password_incorrect() {
        let password = TEST_PASSWORD;
        let wrong_password = "wrongpassword123";
        let hash = hash_password(password).unwrap();

        let result = verify_password(wrong_password, &hash).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_verify_password_empty() {
        let password = TEST_PASSWORD;
        let hash = hash_password(password).unwrap();

        let result = verify_password("", &hash).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_hash_empty_password() {
        let hash = hash_password("").unwrap();
        assert!(hash.starts_with("$2a$") || hash.starts_with("$2b$"));
        assert_ne!(hash, "");
    }

    #[test]
    fn test_hash_long_password() {
        let long_password = "a".repeat(100);
        let hash = hash_password(&long_password).unwrap();
        assert!(hash.starts_with("$2a$") || hash.starts_with("$2b$"));
    }
}

#[cfg(test)]
mod verification_code_tests {
    use super::*;

    #[test]
    fn test_generate_verification_code_length() {
        let code = generate_verification_code();
        assert_eq!(code.len(), 6);
    }

    #[test]
    fn test_generate_verification_code_alphanumeric() {
        let code = generate_verification_code();
        for c in code.chars() {
            assert!(c.is_ascii_alphanumeric());
        }
    }

    #[test]
    fn test_generate_verification_code_different() {
        let code1 = generate_verification_code();
        let code2 = generate_verification_code();

        assert_ne!(code1, code2);
    }

    #[test]
    fn test_generate_verification_code_uppercase() {
        let code = generate_verification_code();
        assert_eq!(code, code.to_uppercase());
    }

    #[test]
    fn test_generate_verification_code_contains_digits() {
        let has_digit = (0..100)
            .map(|_| generate_verification_code())
            .any(|code| code.chars().any(|c| c.is_ascii_digit()));
        assert!(has_digit);
    }

    #[test]
    fn test_generate_verification_code_contains_letters() {
        let has_letter = (0..100)
            .map(|_| generate_verification_code())
            .any(|code| code.chars().any(|c| c.is_ascii_alphabetic()));
        assert!(has_letter);
    }
}

#[cfg(test)]
mod jwt_service_tests {
    use super::*;
    use chrono::Utc;
    use vibe_mcp_server::services::decode_jwt_token;

    #[test]
    fn test_create_jwt_token() {
        let user = create_test_user();
        let config = create_test_config();

        let token = create_jwt_token(&user, &config).unwrap();

        assert!(!token.is_empty());
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn test_decode_jwt_token() {
        let user = create_verified_user();
        let config = create_test_config();

        let token = create_jwt_token(&user, &config).unwrap();
        let claims = decode_jwt_token(&token, &config).unwrap();

        assert_eq!(claims.sub, user.id);
        assert_eq!(claims.email, user.email);
        assert!(claims.exp > claims.iat);
    }

    #[test]
    fn test_jwt_token_contains_user_id() {
        let user = create_test_user();
        let config = create_test_config();

        let token = create_jwt_token(&user, &config).unwrap();
        let claims = decode_jwt_token(&token, &config).unwrap();

        assert_eq!(claims.sub, "test-user-id-001");
    }

    #[test]
    fn test_jwt_token_contains_email() {
        let user = create_test_user();
        let config = create_test_config();

        let token = create_jwt_token(&user, &config).unwrap();
        let claims = decode_jwt_token(&token, &config).unwrap();

        assert_eq!(claims.email, "testuser@vibe.app");
    }

    #[test]
    fn test_jwt_token_expiry_future() {
        let user = create_test_user();
        let config = create_test_config();

        let token = create_jwt_token(&user, &config).unwrap();
        let claims = decode_jwt_token(&token, &config).unwrap();

        let now = Utc::now().timestamp();
        assert!(claims.exp > now);
    }

    #[test]
    fn test_jwt_invalid_token() {
        let config = create_test_config();

        let result = decode_jwt_token("invalid-token", &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_jwt_wrong_secret() {
        let user = create_test_user();
        let config1 = create_test_config();

        let token = create_jwt_token(&user, &config1).unwrap();

        let mut config2 = create_test_config();
        config2.jwt_secret = "wrong-secret".to_string();

        let result = decode_jwt_token(&token, &config2);
        assert!(result.is_err());
    }

    #[test]
    fn test_jwt_empty_token() {
        let config = create_test_config();

        let result = decode_jwt_token("", &config);
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod user_service_tests {
    use super::*;
    use vibe_mcp_server::utils::helpers::now_millis;

    fn create_user_service() -> UserService {
        UserService::new(String::new())
    }

    #[test]
    fn test_user_service_new() {
        let service = create_user_service();
        let users = service.users.lock().unwrap();
        assert!(users.is_empty());
    }

    #[test]
    fn test_create_user() {
        let service = create_user_service();

        let user = service.create_user("test@example.com", "hash123");

        assert!(!user.id.is_empty());
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.password_hash, "hash123");
        assert!(!user.is_verified);
        assert!(user.created_at > 0);
    }

    #[test]
    fn test_find_user_by_email() {
        let service = create_user_service();

        let created_user = service.create_user("findme@test.com", "hash");
        let found_user = service.find_user_by_email("findme@test.com");

        assert!(found_user.is_some());
        assert_eq!(found_user.unwrap().id, created_user.id);
    }

    #[test]
    fn test_find_user_by_email_not_found() {
        let service = create_user_service();

        let found = service.find_user_by_email("nonexistent@test.com");
        assert!(found.is_none());
    }

    #[test]
    fn test_find_user_by_id() {
        let service = create_user_service();

        let created_user = service.create_user("idtest@test.com", "hash");
        let found_user = service.find_user_by_id(&created_user.id);

        assert!(found_user.is_some());
        assert_eq!(found_user.unwrap().email, "idtest@test.com");
    }

    #[test]
    fn test_find_user_by_id_not_found() {
        let service = create_user_service();

        let found = service.find_user_by_id("non-existent-id");
        assert!(found.is_none());
    }

    #[test]
    fn test_multiple_users() {
        let service = create_user_service();

        service.create_user("user1@test.com", "hash1");
        service.create_user("user2@test.com", "hash2");
        service.create_user("user3@test.com", "hash3");

        let users = service.users.lock().unwrap();
        assert_eq!(users.len(), 3);
    }

    #[test]
    fn test_save_and_verify_code() {
        let service = create_user_service();

        service.save_verification_code("test@test.com", "ABC123", 10);

        assert!(service.verify_code("test@test.com", "ABC123"));
    }

    #[test]
    fn test_verify_code_wrong_code() {
        let service = create_user_service();

        service.save_verification_code("test@test.com", "ABC123", 10);

        assert!(!service.verify_code("test@test.com", "WRONG"));
    }

    #[test]
    fn test_verify_code_expired() {
        let service = create_user_service();

        service.save_verification_code("test@test.com", "EXPIRE", 0);

        assert!(!service.verify_code("test@test.com", "EXPIRE"));
    }

    #[test]
    fn test_verify_code_removed_after_use() {
        let service = create_user_service();

        service.save_verification_code("test@test.com", "ONETIME", 10);
        assert!(service.verify_code("test@test.com", "ONETIME"));
        assert!(!service.verify_code("test@test.com", "ONETIME"));
    }

    #[test]
    fn test_set_user_verified() {
        let service = create_user_service();

        let _user = service.create_user("verify@test.com", "hash");

        service.set_user_verified("verify@test.com");

        let verified_user = service.find_user_by_email("verify@test.com").unwrap();
        assert!(verified_user.is_verified);
    }
}

#[cfg(test)]
mod task_service_tests {
    use super::*;
    use vibe_mcp_server::models::TaskStatus;

    fn create_task_service() -> TaskService {
        TaskService::new(String::new())
    }

    #[test]
    fn test_task_service_new() {
        let service = create_task_service();
        let tasks = service.tasks.lock().unwrap();
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_update_task_status_new_task() {
        let service = create_task_service();

        let result = service.update_task_status(
            "new-task-001",
            Some("running"),
            None,
            None,
            None,
            "user-001",
        );

        assert!(result.is_ok());

        let tasks = service.get_tasks(Some("user-001"));
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].status, TaskStatus::Running);
    }

    #[test]
    fn test_update_task_status_existing_task() {
        let service = create_task_service();

        service
            .update_task_status("task-001", Some("running"), None, None, None, "user-001")
            .unwrap();
        service
            .update_task_status("task-001", Some("completed"), None, None, None, "user-001")
            .unwrap();

        let tasks = service.get_tasks(Some("user-001"));
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].status, TaskStatus::Completed);
    }

    #[test]
    fn test_update_task_status_invalid_status() {
        let service = create_task_service();

        let result = service.update_task_status(
            "task-001",
            Some("invalid_status"),
            None,
            None,
            None,
            "user-001",
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_get_tasks_all_users() {
        let service = create_task_service();

        service
            .update_task_status("task-1", Some("running"), None, None, None, "user-1")
            .unwrap();
        service
            .update_task_status("task-2", Some("completed"), None, None, None, "user-2")
            .unwrap();

        let tasks = service.get_tasks(None);
        assert_eq!(tasks.len(), 2);
    }

    #[test]
    fn test_get_tasks_specific_user() {
        let service = create_task_service();

        service
            .update_task_status("task-1", Some("running"), None, None, None, "user-1")
            .unwrap();
        service
            .update_task_status("task-2", Some("completed"), None, None, None, "user-2")
            .unwrap();

        let tasks = service.get_tasks(Some("user-1"));
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "task-1");
    }

    #[test]
    fn test_update_task_progress() {
        let service = create_task_service();

        service
            .update_task_status("task-001", Some("running"), None, None, None, "user-001")
            .unwrap();
        let result = service.update_task_progress(
            "task-001",
            Some(3600000),
            Some("Writing code"),
            "user-001",
        );

        assert!(result.is_ok());

        let tasks = service.get_tasks(Some("user-001"));
        assert_eq!(tasks[0].estimated_duration, Some(3600000));
        assert_eq!(tasks[0].current_stage, Some("Writing code".to_string()));
    }

    #[test]
    fn test_update_task_progress_nonexistent_task() {
        let service = create_task_service();

        let result = service.update_task_progress(
            "nonexistent-task",
            Some(3600000),
            Some("Stage"),
            "user-001",
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_delete_task() {
        let service = create_task_service();

        service
            .update_task_status("task-1", Some("running"), None, None, None, "user-001")
            .unwrap();
        assert_eq!(service.get_tasks(Some("user-001")).len(), 1);

        let deleted = service.delete_task("task-1", "user-001");
        assert!(deleted);
        assert_eq!(service.get_tasks(Some("user-001")).len(), 0);
    }

    #[test]
    fn test_delete_task_not_found() {
        let service = create_task_service();

        let deleted = service.delete_task("nonexistent-task", "user-001");
        assert!(!deleted);
    }

    #[test]
    fn test_delete_task_wrong_user() {
        let service = create_task_service();

        service
            .update_task_status("task-1", Some("running"), None, None, None, "user-1")
            .unwrap();
        let deleted = service.delete_task("task-1", "user-2");

        assert!(!deleted);
        assert_eq!(service.get_tasks(None).len(), 1);
    }

    #[test]
    fn test_reset_tasks_specific() {
        let service = create_task_service();

        service
            .update_task_status("task-1", Some("running"), None, None, None, "user-001")
            .unwrap();
        service
            .update_task_status("task-2", Some("completed"), None, None, None, "user-001")
            .unwrap();
        service
            .update_task_status("task-3", Some("running"), None, None, None, "user-001")
            .unwrap();

        service.reset_tasks(Some("task-2".to_string()), "user-001");

        let tasks = service.get_tasks(Some("user-001"));
        assert_eq!(tasks.len(), 2);
        assert!(tasks.iter().all(|t| t.id != "task-2"));
    }

    #[test]
    fn test_reset_tasks_all_for_user() {
        let service = create_task_service();

        service
            .update_task_status("task-1", Some("running"), None, None, None, "user-001")
            .unwrap();
        service
            .update_task_status("task-2", Some("completed"), None, None, None, "user-001")
            .unwrap();
        service
            .update_task_status("task-3", Some("running"), None, None, None, "user-001")
            .unwrap();

        service.reset_tasks(None, "user-001");

        let tasks = service.get_tasks(Some("user-001"));
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_reset_tasks_preserves_other_users() {
        let service = create_task_service();

        service
            .update_task_status("task-1", Some("running"), None, None, None, "user-1")
            .unwrap();
        service
            .update_task_status("task-2", Some("completed"), None, None, None, "user-2")
            .unwrap();

        service.reset_tasks(None, "user-1");

        let all_tasks = service.get_tasks(None);
        assert_eq!(all_tasks.len(), 1);
        assert_eq!(all_tasks[0].id, "task-2");
    }

    #[test]
    fn test_calculate_progress_completed() {
        let service = create_task_service();

        let task = vibe_mcp_server::Task::new(
            "test".to_string(),
            "user".to_string(),
            "test".to_string(),
            "IDE".to_string(),
            "title".to_string(),
        );

        let progress = service.calculate_progress(&task);
        assert_eq!(progress, 100);
    }

    #[test]
    fn test_calculate_progress_no_estimate() {
        let mut task = vibe_mcp_server::Task::new(
            "test".to_string(),
            "user".to_string(),
            "test".to_string(),
            "IDE".to_string(),
            "title".to_string(),
        );
        task.status = TaskStatus::Running;
        let service = create_task_service();
        let progress = service.calculate_progress(&task);
        assert_eq!(progress, 0);
    }

    #[test]
    fn test_task_status_transitions() {
        let service = create_task_service();

        // Armed -> Running
        service
            .update_task_status("task-1", Some("armed"), None, None, None, "user")
            .unwrap();
        assert_eq!(service.get_tasks(Some("user"))[0].status, TaskStatus::Armed);

        service
            .update_task_status("task-1", Some("running"), None, None, None, "user")
            .unwrap();
        assert_eq!(
            service.get_tasks(Some("user"))[0].status,
            TaskStatus::Running
        );

        // Running -> Completed
        service
            .update_task_status("task-1", Some("completed"), None, None, None, "user")
            .unwrap();
        assert_eq!(
            service.get_tasks(Some("user"))[0].status,
            TaskStatus::Completed
        );

        // Completed -> Armed (tasks can be reused)
        service
            .update_task_status("task-1", Some("armed"), None, None, None, "user")
            .unwrap();
        assert_eq!(service.get_tasks(Some("user"))[0].status, TaskStatus::Armed);
    }
}
