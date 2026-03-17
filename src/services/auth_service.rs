use crate::config::Config;
use crate::models::{Claims, User};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};

pub struct AuthService;

impl AuthService {
    pub fn create_jwt_token(
        user: &User,
        config: &Config,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        let expiry = Utc::now()
            .checked_add_signed(Duration::hours(config.jwt_expiry_hours))
            .unwrap()
            .timestamp();

        let claims = Claims {
            sub: user.id.clone(),
            email: user.email.clone(),
            exp: expiry,
            iat: Utc::now().timestamp(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
        )
    }

    pub fn decode_jwt_token(
        token: &str,
        config: &Config,
    ) -> Result<Claims, jsonwebtoken::errors::Error> {
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(config.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map(|data| data.claims)
    }

    pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
        bcrypt::hash(password, 6)
    }

    pub fn verify_password(password: &str, hash: &str) -> Result<bool, bcrypt::BcryptError> {
        bcrypt::verify(password, hash)
    }

    pub fn generate_verification_code() -> String {
        const CHARSET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        const CODE_LENGTH: usize = 6;

        let mut rng = rand::thread_rng();
        (0..CODE_LENGTH)
            .map(|_| {
                let idx = rand::Rng::gen_range(&mut rng, 0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }
}
