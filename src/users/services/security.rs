use argon2rs::argon2i_simple;
use async_trait::async_trait;
use chrono::{Duration, Utc};

use crate::users::entity::{TokenPayload, User};
use crate::core::CommonError;

#[async_trait]
pub trait UserSecurityService: Send + Sync {
    async fn hash(&self, input: &str) -> Result<String, CommonError>;

    async fn verify_hash(&self, hashed: &str, raw: &str) -> Result<bool, CommonError>;

    async fn token_generator(&self, user: &User) -> Result<String, CommonError>;

    async fn decode_token(&self, token: &str) -> Result<TokenPayload, CommonError>;
}

pub struct UserSecurityServiceImpl {
    pub salt: String,
    pub jwt_key: String,
}

#[async_trait]
impl UserSecurityService for UserSecurityServiceImpl {
    async fn hash(&self, input: &str) -> Result<String, CommonError> {
        let result = argon2i_simple(&input, &self.salt)
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect();
        Ok(result)
    }

    async fn verify_hash(&self, hashed: &str, raw: &str) -> Result<bool, CommonError> {
        let hashed_curr = self.hash(raw).await?;
        Ok(hashed_curr == hashed)
    }

    async fn token_generator(&self, user: &User) -> Result<String, CommonError> {
        let claim = TokenPayload {
            email: user.email.clone(),
            exp: (Utc::now() + Duration::days(30)).timestamp(),
        };
        let encoding_key = jsonwebtoken::EncodingKey::from_secret(self.jwt_key.as_ref());
        let token = jsonwebtoken::encode(&jsonwebtoken::Header::default(), &claim, &encoding_key)
            .map_err(|e| CommonError {
            message: e.to_string(),
            code: 500,
        })?;
        Ok(token)
    }

    async fn decode_token(&self, token: &str) -> Result<TokenPayload, CommonError> {
        let decoding_key = jsonwebtoken::DecodingKey::from_secret(self.jwt_key.as_ref());
        jsonwebtoken::decode::<TokenPayload>(
            token,
            &decoding_key,
            &jsonwebtoken::Validation::default(),
        )
        .map(|data| data.claims)
        .map_err(|e| CommonError {
            message: e.to_string(),
            code: 403,
        })
    }
}
