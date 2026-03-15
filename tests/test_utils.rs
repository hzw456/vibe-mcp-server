//! Test utilities and fixtures for Vibe MCP Server tests.

use vibe_mcp_server::models::TaskStatus;
use vibe_mcp_server::services::{create_jwt_token, generate_verification_code, hash_password};
use vibe_mcp_server::{Config, Task, TaskService, User, UserService};

/// Test constants
pub const TEST_EMAIL: &str = "testuser@vibe.app";
pub const TEST_PASSWORD: &str = "testpass123";
pub const TEST_JWT_SECRET: &str = "test-secret";

/// Test user fixture
pub fn test_user() -> User {
    User {
        id: "test-user-id-001".to_string(),
        email: TEST_EMAIL.to_string(),
        password_hash: "$2a$06$testhash123456".to_string(),
        created_at: 1700000000000,
        is_verified: false,
    }
}

/// Verified test user fixture
pub fn verified_test_user() -> User {
    User {
        id: "test-user-id-001".to_string(),
        email: TEST_EMAIL.to_string(),
        password_hash: "$2a$06$testhash123456".to_string(),
        created_at: 1700000000000,
        is_verified: true,
    }
}

/// Create a test config
pub fn test_config() -> Config {
    Config {
        host: "0.0.0.0".to_string(),
        port: 3010,
        api_key: "test-api-key".to_string(),
        jwt_secret: TEST_JWT_SECRET.to_string(),
        jwt_expiry_hours: 24,
    }
}

/// Create a fresh test user service
pub fn create_test_user_service() -> UserService {
    UserService::new(String::new())
}

/// Create a fresh test task service
pub fn create_test_task_service() -> TaskService {
    TaskService::new(String::new())
}

/// Create a complete test setup with user and task services
pub fn create_test_setup() -> (UserService, TaskService, Config) {
    let user_service = create_test_user_service();
    let task_service = create_test_task_service();
    let config = test_config();

    // Create a test user
    let password_hash = hash_password(TEST_PASSWORD).unwrap();
    user_service.create_user(TEST_EMAIL, &password_hash);

    (user_service, task_service, config)
}

/// Generate a JWT token for testing
pub fn generate_test_token(user: &User) -> String {
    let config = test_config();
    create_jwt_token(user, &config).unwrap()
}

/// Generate a verification code for testing
pub fn generate_test_verification_code() -> String {
    generate_verification_code()
}

/// Task fixtures
pub fn test_task(id: &str) -> Task {
    Task::new(
        id.to_string(),
        "test-user-id-001".to_string(),
        id.to_string(),
        "VSCode".to_string(),
        format!("{} - VSCode", id),
    )
}

/// Create multiple test tasks
pub fn create_test_tasks(task_service: &TaskService, count: usize) {
    for i in 1..=count {
        let task_id = format!("test-task-{:03}", i);
        let _ = task_service.update_task_status(
            &task_id,
            Some("running"),
            Some("mcp"),
            None,
            None,
            TEST_EMAIL,
        );
    }
}

/// Clean up task service
pub fn cleanup_task_service(task_service: &TaskService) {
    task_service.reset_tasks(None, TEST_EMAIL);
}

/// Clean up user service
pub fn cleanup_user_service(user_service: &UserService) {
    let mut users = user_service.users.lock().unwrap();
    users.clear();

    let mut codes = user_service.verification_codes.lock().unwrap();
    codes.clear();
}

/// Full cleanup for test
pub fn cleanup_test_setup(user_service: &UserService, task_service: &TaskService) {
    cleanup_task_service(task_service);
    cleanup_user_service(user_service);
}

/// Async test client helper
#[cfg(test)]
pub struct TestClient {
    pub user_service: std::sync::Arc<UserService>,
    pub task_service: std::sync::Arc<TaskService>,
    pub config: Config,
}

impl TestClient {
    /// Create a new test client
    pub fn new() -> Self {
        Self {
            user_service: std::sync::Arc::new(create_test_user_service()),
            task_service: std::sync::Arc::new(create_test_task_service()),
            config: test_config(),
        }
    }

    /// Register a test user
    pub fn register_test_user(&self) -> User {
        let password_hash = hash_password(TEST_PASSWORD).unwrap();
        self.user_service.create_user(TEST_EMAIL, &password_hash)
    }

    /// Verify test user
    pub fn verify_test_user(&self) {
        let code = generate_test_verification_code();
        self.user_service
            .save_verification_code(TEST_EMAIL, &code, 10);
        self.user_service.verify_code(TEST_EMAIL, &code);
        self.user_service.set_user_verified(TEST_EMAIL);
    }

    /// Login and get token
    pub fn login(&self) -> String {
        let user = self.user_service.find_user_by_email(TEST_EMAIL).unwrap();
        generate_test_token(&user)
    }

    /// Create test tasks
    pub fn create_test_tasks(&self, count: usize) {
        create_test_tasks(&self.task_service, count);
    }

    /// Get all tasks for test user
    pub fn get_tasks(&self) -> Vec<Task> {
        self.task_service.get_tasks(Some(TEST_EMAIL))
    }
}

impl Default for TestClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to create test data with various states
#[cfg(test)]
pub struct TestDataBuilder {
    user_service: UserService,
    task_service: TaskService,
}

impl TestDataBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            user_service: create_test_user_service(),
            task_service: create_test_task_service(),
        }
    }

    /// Add a registered but unverified user
    pub fn with_unverified_user(self) -> Self {
        let password_hash = hash_password(TEST_PASSWORD).unwrap();
        self.user_service.create_user(TEST_EMAIL, &password_hash);
        self
    }

    /// Add a verified user
    pub fn with_verified_user(self) -> Self {
        let password_hash = hash_password(TEST_PASSWORD).unwrap();
        self.user_service.create_user(TEST_EMAIL, &password_hash);
        self
    }

    /// Add tasks in various states
    pub fn with_varied_tasks(self) -> Self {
        self.task_service
            .update_task_status(
                "test-task-001",
                Some("armed"),
                Some("mcp"),
                None,
                None,
                TEST_EMAIL,
            )
            .unwrap();
        self.task_service
            .update_task_status(
                "test-task-002",
                Some("running"),
                Some("mcp"),
                None,
                Some("Stage 1"),
                TEST_EMAIL,
            )
            .unwrap();
        self.task_service
            .update_task_status(
                "test-task-003",
                Some("completed"),
                Some("mcp"),
                Some(60000),
                Some("Done"),
                TEST_EMAIL,
            )
            .unwrap();
        self
    }

    /// Build and return services
    pub fn build(self) -> (UserService, TaskService) {
        (self.user_service, self.task_service)
    }

    /// Build with config
    pub fn build_with_config(self) -> (UserService, TaskService, Config) {
        let config = test_config();
        (self.user_service, self.task_service, config)
    }
}

impl Default for TestDataBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Random test data generators
#[cfg(test)]
pub mod generators {
    use super::*;
    use rand::Rng;

    /// Generate a random email
    pub fn random_email() -> String {
        let mut rng = rand::thread_rng();
        let local: u32 = rng.gen();
        format!("user{}@test.example.com", local)
    }

    /// Generate a random task ID
    pub fn random_task_id() -> String {
        let mut rng = rand::thread_rng();
        let id: u32 = rng.gen();
        format!("task-{}", id)
    }

    /// Generate random password
    pub fn random_password() -> String {
        let mut rng = rand::thread_rng();
        (0..12)
            .map(|_| {
                const CHARSET: &[u8] =
                    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
                CHARSET[rng.gen_range(0..CHARSET.len())] as char
            })
            .collect()
    }
}
