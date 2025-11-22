use jsonwebtoken::{decode, DecodingKey, Validation, errors::Error, Algorithm};
use serde::{Deserialize, Serialize};
use std::env;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Claims {
    pub sub: Uuid,
    pub name: String,
    pub email: String,
    pub exp: usize
}

pub fn validate_token(token: &str) -> Result<Claims, Error> {
    let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let key = DecodingKey::from_secret(secret.as_ref());
    let validation = Validation::new(Algorithm::HS256);

    return decode::<Claims>(token, &key, &validation).map(|data| data.claims);
}