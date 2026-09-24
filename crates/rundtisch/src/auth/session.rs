use crate::auth::config::{SESSION_COOKIE, SESSION_TTL};
use crate::traits::random::{RandomError, RandomSource};
use axum::http::{HeaderMap, HeaderValue, header};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

pub fn generate_session_token(random: &dyn RandomSource) -> Result<String, RandomError> {
    let mut bytes = [0u8; 32];
    random.fill_bytes(&mut bytes)?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

pub fn token_hash(pepper: &[u8], raw_token: &str) -> Result<String, String> {
    let mut mac = HmacSha256::new_from_slice(pepper).map_err(|err| err.to_string())?;
    mac.update(raw_token.as_bytes());
    Ok(to_hex(&mac.finalize().into_bytes()))
}

pub fn token_hash_eq(left: &str, right: &str) -> bool {
    bool::from(left.as_bytes().ct_eq(right.as_bytes()))
}

fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

pub fn session_cookie_header(raw_token: &str) -> Result<HeaderValue, String> {
    let value = format!(
        "{SESSION_COOKIE}={raw_token}; Path=/; HttpOnly; Secure; SameSite=Strict; Max-Age={}",
        SESSION_TTL.whole_seconds()
    );
    HeaderValue::from_str(&value).map_err(|err| err.to_string())
}

pub fn clear_session_cookie_header() -> HeaderValue {
    HeaderValue::from_static("session=; Path=/; HttpOnly; Secure; SameSite=Strict; Max-Age=0")
}

pub fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    let header = headers.get(header::COOKIE)?.to_str().ok()?;
    header.split(';').find_map(|part| {
        let (key, value) = part.trim().split_once('=')?;
        (key == name).then(|| value.to_string())
    })
}

pub fn cap_user_agent(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(header::USER_AGENT)?.to_str().ok()?;
    let mut s = value.to_string();
    if s.len() > 512 {
        s.truncate(512);
    }
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::random::ReplayRandom;

    #[test]
    fn hash_is_hex_hmac_and_eq() {
        let pepper = b"cccccccccccccccccccccccccccccccc";
        let hash = token_hash(pepper, "token-a").unwrap();
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(hash, token_hash(pepper, "token-b").unwrap());
        assert!(token_hash_eq(&hash, &hash));
        assert!(!token_hash_eq(&hash, "abcd"));
    }

    #[test]
    fn session_token_is_url_safe() {
        let rng = ReplayRandom::new(vec![1u8; 32]);
        let token = generate_session_token(&rng).unwrap();
        assert!(!token.contains('='));
        assert!(URL_SAFE_NO_PAD.decode(&token).is_ok());
    }
}
