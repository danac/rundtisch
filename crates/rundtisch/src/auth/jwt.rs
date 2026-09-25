use crate::auth::config::{ACCESS_TTL, ACTIVATION_TTL, JWT_LEEWAY};
use crate::auth::models::Role;
use crate::traits::clock::Clock;
use chrono::{DateTime, Utc};
use jwt_compact::alg::{Hs256, Hs256Key};
use jwt_compact::prelude::*;
use jwt_compact::{Claims, Header, TimeOptions, UntrustedToken};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const TYP_ACCESS: &str = "access";
const TYP_EMAIL_VERIFY: &str = "email_verify";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenError {
    Expired,
    Invalid,
    Backend(String),
}

impl std::fmt::Display for TokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenError::Expired => write!(f, "token_expired"),
            TokenError::Invalid => write!(f, "invalid_token"),
            TokenError::Backend(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for TokenError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct AccessPrivate {
    sub: String,
    role: Role,
    typ: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ActivationPrivate {
    sub: String,
    email: String,
    typ: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessClaims {
    pub sub: Uuid,
    pub role: Role,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivationClaims {
    pub sub: Uuid,
    pub email: String,
}

fn chrono_utc(now: time::OffsetDateTime) -> DateTime<Utc> {
    DateTime::from_timestamp(now.unix_timestamp(), now.nanosecond())
        .expect("timestamp in chrono range")
}

fn time_options(clock: &dyn Clock) -> TimeOptions<impl Fn() -> DateTime<Utc> + '_> {
    TimeOptions::new(
        chrono::Duration::seconds(JWT_LEEWAY.whole_seconds()),
        || chrono_utc(clock.now_utc()),
    )
}

fn key(secret: &[u8]) -> Result<Hs256Key, TokenError> {
    if secret.len() < 32 {
        return Err(TokenError::Backend(
            "JWT secret must be at least 32 bytes".into(),
        ));
    }
    Ok(Hs256Key::new(secret))
}

pub fn issue_access_token(
    sub: Uuid,
    role: Role,
    secret: &[u8],
    clock: &dyn Clock,
) -> Result<String, TokenError> {
    let key = key(secret)?;
    let options = time_options(clock);
    let claims = Claims::new(AccessPrivate {
        sub: sub.to_string(),
        role,
        typ: TYP_ACCESS.into(),
    })
    .set_duration(
        &options,
        chrono::Duration::seconds(ACCESS_TTL.whole_seconds()),
    );
    Hs256
        .token(&Header::empty(), &claims, &key)
        .map_err(|err| TokenError::Backend(err.to_string()))
}

pub fn verify_access_token(
    token: &str,
    secret: &[u8],
    clock: &dyn Clock,
) -> Result<AccessClaims, TokenError> {
    let key = key(secret)?;
    let untrusted = UntrustedToken::new(token).map_err(|_| TokenError::Invalid)?;
    let token: jwt_compact::Token<AccessPrivate> = Hs256
        .validator(&key)
        .validate(&untrusted)
        .map_err(|_| TokenError::Invalid)?;
    let claims = token.claims();
    match claims.validate_expiration(&time_options(clock)) {
        Ok(_) => {}
        Err(jwt_compact::ValidationError::Expired) => return Err(TokenError::Expired),
        Err(_) => return Err(TokenError::Invalid),
    }
    if claims.custom.typ != TYP_ACCESS {
        return Err(TokenError::Invalid);
    }
    let sub = Uuid::parse_str(&claims.custom.sub).map_err(|_| TokenError::Invalid)?;
    Ok(AccessClaims {
        sub,
        role: claims.custom.role,
    })
}

pub fn issue_activation_token(
    sub: Uuid,
    email: &str,
    secret: &[u8],
    clock: &dyn Clock,
) -> Result<String, TokenError> {
    let key = key(secret)?;
    let options = time_options(clock);
    let claims = Claims::new(ActivationPrivate {
        sub: sub.to_string(),
        email: email.to_string(),
        typ: TYP_EMAIL_VERIFY.into(),
    })
    .set_duration(
        &options,
        chrono::Duration::seconds(ACTIVATION_TTL.whole_seconds()),
    );
    Hs256
        .token(&Header::empty(), &claims, &key)
        .map_err(|err| TokenError::Backend(err.to_string()))
}

pub fn verify_activation_token(
    token: &str,
    secret: &[u8],
    clock: &dyn Clock,
) -> Result<ActivationClaims, TokenError> {
    let key = key(secret)?;
    let untrusted = UntrustedToken::new(token).map_err(|_| TokenError::Invalid)?;
    let token: jwt_compact::Token<ActivationPrivate> = Hs256
        .validator(&key)
        .validate(&untrusted)
        .map_err(|_| TokenError::Invalid)?;
    let claims = token.claims();
    match claims.validate_expiration(&time_options(clock)) {
        Ok(_) => {}
        Err(jwt_compact::ValidationError::Expired) => return Err(TokenError::Expired),
        Err(_) => return Err(TokenError::Invalid),
    }
    if claims.custom.typ != TYP_EMAIL_VERIFY {
        return Err(TokenError::Invalid);
    }
    let sub = Uuid::parse_str(&claims.custom.sub).map_err(|_| TokenError::Invalid)?;
    Ok(ActivationClaims {
        sub,
        email: claims.custom.email.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::clock::FrozenClock;

    const ACCESS: &[u8] = b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const VERIFY: &[u8] = b"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn clock() -> FrozenClock {
        FrozenClock(time::OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap())
    }

    #[test]
    fn access_token_round_trip() {
        let sub = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let token = issue_access_token(sub, Role::Admin, ACCESS, &clock()).unwrap();
        let claims = verify_access_token(&token, ACCESS, &clock()).unwrap();
        assert_eq!(claims.sub, sub);
        assert_eq!(claims.role, Role::Admin);
        assert!(verify_access_token(&token, VERIFY, &clock()).is_err());
    }

    #[test]
    fn access_token_expires() {
        let sub = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let issued = clock();
        let token = issue_access_token(sub, Role::User, ACCESS, &issued).unwrap();
        let later = FrozenClock(issued.0 + ACCESS_TTL + time::Duration::minutes(1));
        assert_eq!(
            verify_access_token(&token, ACCESS, &later).unwrap_err(),
            TokenError::Expired
        );
    }

    #[test]
    fn activation_is_not_access() {
        let sub = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let token = issue_activation_token(sub, "carol@example.com", VERIFY, &clock()).unwrap();
        assert!(verify_access_token(&token, VERIFY, &clock()).is_err());
        assert!(verify_access_token(&token, ACCESS, &clock()).is_err());
        let claims = verify_activation_token(&token, VERIFY, &clock()).unwrap();
        assert_eq!(claims.sub, sub);
        assert_eq!(claims.email, "carol@example.com");
    }
}
