//! Unit tests for the Vibe MCP Server configuration.

use vibe_mcp_server::Config;

#[cfg(test)]
mod config_tests {
    use super::*;
    use std::env;

    /// Test JWT secret constant
    const TEST_JWT_SECRET: &str = "test-secret";

    #[test]
    fn test_config_default_values() {
        // Clear any environment variables that might interfere
        env::remove_var("API_KEY");
        env::remove_var("JWT_SECRET");
        env::remove_var("JWT_EXPIRY_HOURS");
        env::remove_var("HOST");
        env::remove_var("PORT");

        let config = Config::default();

        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 3010);
        assert_eq!(config.jwt_expiry_hours, 24);
        assert_eq!(config.api_key, "vibe-mcp-secret-key");
        assert!(!config.jwt_secret.is_empty());
    }

    #[test]
    fn test_config_from_env_api_key() {
        env::set_var("API_KEY", "custom-api-key-123");

        let config = Config::default();
        assert_eq!(config.api_key, "custom-api-key-123");

        env::remove_var("API_KEY");
    }

    #[test]
    fn test_config_from_env_jwt_secret() {
        env::set_var("JWT_SECRET", TEST_JWT_SECRET);

        let config = Config::default();
        assert_eq!(config.jwt_secret, TEST_JWT_SECRET);

        env::remove_var("JWT_SECRET");
    }

    #[test]
    fn test_config_from_env_jwt_expiry() {
        env::set_var("JWT_EXPIRY_HOURS", "48");

        let config = Config::default();
        assert_eq!(config.jwt_expiry_hours, 48);

        env::remove_var("JWT_EXPIRY_HOURS");
    }

    #[test]
    fn test_config_from_env_host() {
        env::set_var("HOST", "127.0.0.1");

        let config = Config::default();
        assert_eq!(config.host, "127.0.0.1");

        env::remove_var("HOST");
    }

    #[test]
    fn test_config_from_env_port() {
        env::set_var("PORT", "8080");

        let config = Config::default();
        assert_eq!(config.port, 8080);

        env::remove_var("PORT");
    }

    #[test]
    fn test_config_invalid_jwt_expiry_fallback() {
        env::set_var("JWT_EXPIRY_HOURS", "invalid");

        let config = Config::default();
        // Should fall back to default value of 24
        assert_eq!(config.jwt_expiry_hours, 24);

        env::remove_var("JWT_EXPIRY_HOURS");
    }

    #[test]
    fn test_config_zero_jwt_expiry() {
        env::set_var("JWT_EXPIRY_HOURS", "0");

        let config = Config::default();
        // Should accept 0
        assert_eq!(config.jwt_expiry_hours, 0);

        env::remove_var("JWT_EXPIRY_HOURS");
    }

    #[test]
    fn test_config_negative_jwt_expiry() {
        env::set_var("JWT_EXPIRY_HOURS", "-1");

        let config = Config::default();
        // Should parse negative value
        assert_eq!(config.jwt_expiry_hours, -1);

        env::remove_var("JWT_EXPIRY_HOURS");
    }

    #[test]
    fn test_config_clone() {
        let config1 = Config::default();
        let config2 = config1.clone();

        assert_eq!(config1.host, config2.host);
        assert_eq!(config1.port, config2.port);
        assert_eq!(config1.api_key, config2.api_key);
        assert_eq!(config1.jwt_secret, config2.jwt_secret);
        assert_eq!(config1.jwt_expiry_hours, config2.jwt_expiry_hours);
    }

    #[test]
    fn test_config_jwt_expiry_hours_range() {
        // Test various valid expiry values
        let test_cases = vec![1, 12, 24, 48, 72, 168]; // 1 hour to 1 week

        for expiry in test_cases {
            env::set_var("JWT_EXPIRY_HOURS", expiry.to_string());
            let config = Config::default();
            assert_eq!(config.jwt_expiry_hours, expiry);
        }

        env::remove_var("JWT_EXPIRY_HOURS");
    }

    #[test]
    fn test_config_all_env_vars_together() {
        env::set_var("HOST", "192.168.1.100");
        env::set_var("PORT", "9000");
        env::set_var("API_KEY", "my-super-secret-key");
        env::set_var("JWT_SECRET", "my-jwt-secret");
        env::set_var("JWT_EXPIRY_HOURS", "72");

        let config = Config::default();

        assert_eq!(config.host, "192.168.1.100");
        assert_eq!(config.port, 9000);
        assert_eq!(config.api_key, "my-super-secret-key");
        assert_eq!(config.jwt_secret, "my-jwt-secret");
        assert_eq!(config.jwt_expiry_hours, 72);

        // Clean up
        env::remove_var("HOST");
        env::remove_var("PORT");
        env::remove_var("API_KEY");
        env::remove_var("JWT_SECRET");
        env::remove_var("JWT_EXPIRY_HOURS");
    }

    #[test]
    fn test_config_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Config>();
    }
}
