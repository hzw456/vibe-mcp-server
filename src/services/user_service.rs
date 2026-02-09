use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::utils::helpers::{now_millis, generate_id};
use crate::services::auth_service::{hash_password, generate_verification_code};
use chrono::{Duration, Utc};

#[derive(Clone)]
pub struct UserService {
    pub users: Arc<Mutex<HashMap<String, crate::models::User>>>,
    pub verification_codes: Arc<Mutex<HashMap<String, crate::models::VerificationCode>>>,
}

impl UserService {
    pub fn new() -> Self {
        Self {
            users: Arc::new(Mutex::new(HashMap::new())),
            verification_codes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn find_user_by_email(&self, email: &str) -> Option<crate::models::User> {
        let users = self.users.lock().unwrap();
        users.values().find(|u| u.email == email).cloned()
    }

    pub fn find_user_by_id(&self, id: &str) -> Option<crate::models::User> {
        let users = self.users.lock().unwrap();
        users.get(id).cloned()
    }

    pub fn create_user(&self, email: &str, password_hash: &str) -> crate::models::User {
        let mut users = self.users.lock().unwrap();
        let user = crate::models::User {
            id: generate_id(),
            email: email.to_string(),
            password_hash: password_hash.to_string(),
            created_at: now_millis(),
            is_verified: false,
        };
        users.insert(user.id.clone(), user.clone());
        user
    }

    pub fn save_verification_code(&self, email: &str, code: &str, expires_minutes: i64) {
        let mut codes = self.verification_codes.lock().unwrap();
        let expires_at = Utc::now()
            .checked_add_signed(Duration::minutes(expires_minutes))
            .unwrap()
            .timestamp_millis();
        codes.insert(email.to_string(), crate::models::VerificationCode {
            code: code.to_string(),
            expires_at,
        });
    }

    pub fn verify_code(&self, email: &str, code: &str) -> bool {
        let mut codes = self.verification_codes.lock().unwrap();
        if let Some(stored) = codes.get(email) {
            if stored.code == code && stored.expires_at > now_millis() {
                codes.remove(email);
                return true;
            }
        }
        false
    }

    pub fn set_user_verified(&self, email: &str) {
        let mut users = self.users.lock().unwrap();
        for user in users.values_mut() {
            if user.email == email {
                user.is_verified = true;
                break;
            }
        }
    }
}
