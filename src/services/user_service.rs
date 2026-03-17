use crate::models::{User, VerificationCode};
use crate::utils::helpers::{generate_id, now_millis};
use chrono::{Duration, Utc};
use serde::Deserialize;
use sqlx::{mysql::MySqlPoolOptions, MySql, Pool, Row};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct SendVerificationRequest {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyCodeRequest {
    pub email: String,
    pub code: String,
}

pub struct UserService {
    pub users: Arc<Mutex<HashMap<String, User>>>,
    pub verification_codes: Arc<Mutex<HashMap<String, VerificationCode>>>,
    pub db: Option<Pool<MySql>>,
    pub db_url: String,
}

impl Clone for UserService {
    fn clone(&self) -> Self {
        Self {
            users: Arc::clone(&self.users),
            verification_codes: Arc::clone(&self.verification_codes),
            db: None,
            db_url: self.db_url.clone(),
        }
    }
}

impl UserService {
    pub fn new(db_url: String) -> Self {
        let users = if db_url.is_empty() {
            HashMap::new()
        } else {
            Self::load_users_from_db(&db_url)
        };

        Self {
            users: Arc::new(Mutex::new(users)),
            verification_codes: Arc::new(Mutex::new(HashMap::new())),
            db: None,
            db_url,
        }
    }

    fn load_users_from_db(db_url: &str) -> HashMap<String, User> {
        let db_url = db_url.to_string();
        std::thread::spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    tracing::warn!("Failed to create startup DB runtime: {}", error);
                    return HashMap::new();
                }
            };

            runtime.block_on(async move {
                let pool = match MySqlPoolOptions::new()
                    .max_connections(1)
                    .connect(&db_url)
                    .await
                {
                    Ok(pool) => pool,
                    Err(error) => {
                        tracing::warn!("Failed to connect to database during user load: {}", error);
                        return HashMap::new();
                    }
                };

                let rows = match sqlx::query(
                    "SELECT id, email, password_hash, is_verified, created_at, api_key FROM vibe_users"
                )
                .fetch_all(&pool)
                .await
                {
                    Ok(rows) => rows,
                    Err(error) => {
                        tracing::warn!("Failed to load users from database: {}", error);
                        return HashMap::new();
                    }
                };

                let mut users = HashMap::with_capacity(rows.len());
                for row in rows {
                    let id: String = match row.try_get("id") {
                        Ok(id) => id,
                        Err(error) => {
                            tracing::warn!("Skipping user row without valid id: {}", error);
                            continue;
                        }
                    };

                    let user = User {
                        id: id.clone(),
                        email: row.try_get("email").unwrap_or_default(),
                        password_hash: row.try_get("password_hash").unwrap_or_default(),
                        created_at: row
                            .try_get::<Option<i64>, _>("created_at")
                            .ok()
                            .flatten()
                            .unwrap_or(0),
                        is_verified: row.try_get("is_verified").unwrap_or(false),
                        api_key: row.try_get("api_key").ok(),
                    };

                    users.insert(id, user);
                }

                tracing::info!("Loaded {} users from database", users.len());
                users
            })
        })
        .join()
        .unwrap_or_else(|_| {
            tracing::warn!("User loading thread panicked during startup");
            HashMap::new()
        })
    }

    pub async fn init_db(&self) -> Option<Pool<MySql>> {
        if self.db_url.is_empty() {
            return None;
        }
        match MySqlPoolOptions::new()
            .max_connections(5)
            .connect(&self.db_url)
            .await
        {
            Ok(pool) => {
                tracing::info!("Connected to database");
                Some(pool)
            }
            Err(e) => {
                tracing::warn!("Failed to connect to database: {}", e);
                None
            }
        }
    }

    pub fn find_user_by_email(&self, email: &str) -> Option<User> {
        let users = self.users.lock().unwrap();
        users.values().find(|u| u.email == email).cloned()
    }

    pub fn find_user_by_api_key(&self, api_key: &str) -> Option<User> {
        let users = self.users.lock().unwrap();
        users
            .values()
            .find(|u| u.api_key.as_deref() == Some(api_key))
            .cloned()
    }

    pub fn find_user_by_id(&self, id: &str) -> Option<User> {
        let users = self.users.lock().unwrap();
        users.get(id).cloned()
    }

    pub fn regenerate_api_key(&self, user_id: &str) -> Result<String, ()> {
        let mut users = self.users.lock().unwrap();
        if let Some(user) = users.get_mut(user_id) {
            let new_key = generate_id();
            user.api_key = Some(new_key.clone());

            // Update in database
            if !self.db_url.is_empty() {
                let db_url = self.db_url.clone();
                let uid = user_id.to_string();
                let key = new_key.clone();
                tokio::spawn(async move {
                    let pool = MySqlPoolOptions::new()
                        .max_connections(1)
                        .connect(&db_url)
                        .await;
                    if let Ok(pool) = pool {
                        let _ = sqlx::query("UPDATE vibe_users SET api_key = ? WHERE id = ?")
                            .bind(&key)
                            .bind(&uid)
                            .execute(&pool)
                            .await;
                    }
                });
            }

            return Ok(new_key);
        }
        Err(())
    }

    pub fn create_user(&self, email: &str, password_hash: &str) -> User {
        let mut users = self.users.lock().unwrap();
        let user = User {
            id: generate_id(),
            email: email.to_string(),
            password_hash: password_hash.to_string(),
            created_at: now_millis(),
            is_verified: false,
            api_key: Some(generate_id()),
        };
        users.insert(user.id.clone(), user.clone());

        if !self.db_url.is_empty() {
            let db_url = self.db_url.clone();
            let user_id = user.id.clone();
            let user_email = user.email.clone();
            let user_hash = user.password_hash.clone();
            let user_verified = user.is_verified;
            let user_created = user.created_at;
            let user_api_key = user.api_key.clone();

            tokio::spawn(async move {
                let pool = MySqlPoolOptions::new()
                    .max_connections(1)
                    .connect(&db_url)
                    .await;
                if let Ok(pool) = pool {
                    let _ = sqlx::query(
                        "INSERT INTO vibe_users (id, email, password_hash, is_verified, created_at, api_key) VALUES (?, ?, ?, ?, ?, ?)"
                    )
                    .bind(&user_id)
                    .bind(&user_email)
                    .bind(&user_hash)
                    .bind(user_verified)
                    .bind(user_created)
                    .bind(&user_api_key)
                    .execute(&pool).await;
                }
            });
        }

        user
    }

    pub fn save_verification_code(&self, email: &str, code: &str, expires_minutes: i64) {
        let mut codes = self.verification_codes.lock().unwrap();
        let expires_at = Utc::now()
            .checked_add_signed(Duration::minutes(expires_minutes))
            .unwrap()
            .timestamp_millis();
        codes.insert(
            email.to_string(),
            VerificationCode {
                code: code.to_string(),
                expires_at,
            },
        );
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

                // Also update database
                if !self.db_url.is_empty() {
                    let db_url = self.db_url.clone();
                    let user_email = email.to_string();
                    tokio::spawn(async move {
                        let pool = MySqlPoolOptions::new()
                            .max_connections(1)
                            .connect(&db_url)
                            .await;
                        if let Ok(pool) = pool {
                            let _ = sqlx::query(
                                "UPDATE vibe_users SET is_verified = TRUE WHERE email = ?",
                            )
                            .bind(&user_email)
                            .execute(&pool)
                            .await;
                        }
                    });
                }
                break;
            }
        }
    }
}
