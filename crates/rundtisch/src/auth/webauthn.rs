use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde_cbor_2::Value as CborValue;
use serde_json::{Map, Value};
use uuid::Uuid;
use webauthn_rs::prelude::{
    AuthenticationResult, CredentialID, DiscoverableAuthentication, DiscoverableKey, Passkey,
    PasskeyAuthentication, PasskeyRegistration, PublicKeyCredential, RegisterPublicKeyCredential,
    Url, Webauthn, WebauthnBuilder,
};

use crate::AppState;
use crate::auth::config::{
    AUTH_WEBAUTHN_RP_ID, AUTH_WEBAUTHN_RP_NAME, AUTH_WEBAUTHN_RP_ORIGIN, DEFAULT_WEBAUTHN_RP_ID,
    DEFAULT_WEBAUTHN_RP_NAME, DEFAULT_WEBAUTHN_RP_ORIGIN,
};
use crate::auth::error::AuthError;
use crate::auth::session::credential_id_key;

pub struct PasskeyCeremony {
    webauthn: Webauthn,
}

impl PasskeyCeremony {
    pub fn from_app(state: &AppState) -> Result<Self, AuthError> {
        let rp_id = env_or(state, AUTH_WEBAUTHN_RP_ID, DEFAULT_WEBAUTHN_RP_ID);
        let rp_origin = env_or(state, AUTH_WEBAUTHN_RP_ORIGIN, DEFAULT_WEBAUTHN_RP_ORIGIN);
        let rp_name = env_or(state, AUTH_WEBAUTHN_RP_NAME, DEFAULT_WEBAUTHN_RP_NAME);
        Self::new(&rp_id, &rp_origin, &rp_name)
    }

    pub fn new(rp_id: &str, rp_origin: &str, rp_name: &str) -> Result<Self, AuthError> {
        let origin = Url::parse(rp_origin).map_err(|_| AuthError::Secrets)?;
        let webauthn = WebauthnBuilder::new(rp_id, &origin)
            .map_err(|err| {
                eprintln!("webauthn rp configuration rejected: {err}");
                AuthError::Secrets
            })?
            .rp_name(rp_name)
            .allow_any_port(true)
            .timeout(Duration::from_secs(300))
            .build()
            .map_err(|err| {
                eprintln!("webauthn builder failed: {err}");
                AuthError::Secrets
            })?;
        Ok(Self { webauthn })
    }

    pub fn start_registration(
        &self,
        user_id: Uuid,
        user_name: &str,
        display_name: &str,
        exclude_credential_ids: &[Vec<u8>],
    ) -> Result<(Value, String), AuthError> {
        let exclude: Vec<CredentialID> = exclude_credential_ids
            .iter()
            .cloned()
            .map(CredentialID::from)
            .collect();
        let exclude = if exclude.is_empty() {
            None
        } else {
            Some(exclude)
        };
        let (options, state) = self
            .webauthn
            .start_passkey_registration(user_id, user_name, display_name, exclude)
            .map_err(ceremony_err)?;
        let mut options_json = serde_json::to_value(&options)
            .map_err(|err| AuthError::Backend(format!("webauthn options: {err}")))?;
        // webauthn-rs passkey registration hardcodes residentKey=discouraged.
        // Discoverable login needs a resident credential, so require one here.
        require_discoverable_credential(&mut options_json);
        // webauthn-rs also hardcodes attestation=none. Request direct so authenticators
        // that support it expose a non-nil AAGUID in attestationObject authData.
        require_direct_attestation(&mut options_json);
        let state_json = serde_json::to_string(&state)
            .map_err(|err| AuthError::Backend(format!("webauthn state: {err}")))?;
        Ok((options_json, state_json))
    }

    pub fn finish_registration(
        &self,
        credential: &Value,
        state_json: &str,
    ) -> Result<(Passkey, Option<Uuid>), AuthError> {
        let reg: RegisterPublicKeyCredential = serde_json::from_value(credential.clone())
            .map_err(|_| AuthError::InvalidCredentials)?;
        let state: PasskeyRegistration =
            serde_json::from_str(state_json).map_err(|_| AuthError::InvalidToken)?;
        let passkey = self
            .webauthn
            .finish_passkey_registration(&reg, &state)
            .map_err(ceremony_err)?;
        // Read AAGUID from raw authData so fmt=none (e.g. Android Password Manager) still works.
        let aaguid = aaguid_from_attestation_object(credential);
        Ok((passkey, aaguid))
    }

    pub fn start_authentication(&self, passkeys: &[Passkey]) -> Result<(Value, String), AuthError> {
        let (options, state) = self
            .webauthn
            .start_passkey_authentication(passkeys)
            .map_err(ceremony_err)?;
        let options_json = serde_json::to_value(&options)
            .map_err(|err| AuthError::Backend(format!("webauthn options: {err}")))?;
        let state_json = serde_json::to_string(&state)
            .map_err(|err| AuthError::Backend(format!("webauthn state: {err}")))?;
        Ok((options_json, state_json))
    }

    pub fn finish_authentication(
        &self,
        credential: &Value,
        state_json: &str,
    ) -> Result<AuthenticationResult, AuthError> {
        let assertion: PublicKeyCredential = serde_json::from_value(credential.clone())
            .map_err(|_| AuthError::InvalidCredentials)?;
        let state: PasskeyAuthentication =
            serde_json::from_str(state_json).map_err(|_| AuthError::InvalidToken)?;
        self.webauthn
            .finish_passkey_authentication(&assertion, &state)
            .map_err(ceremony_err)
    }

    /// Usernameless login. Options carry an empty allow list so the response
    /// does not reveal which accounts have passkeys. `mediation` is cleared so
    /// the client chooses conditional autofill or a modal prompt.
    pub fn start_discoverable_authentication(&self) -> Result<(Value, String), AuthError> {
        let (mut options, state) = self
            .webauthn
            .start_discoverable_authentication()
            .map_err(ceremony_err)?;
        options.mediation = None;
        let options_json = serde_json::to_value(&options)
            .map_err(|err| AuthError::Backend(format!("webauthn options: {err}")))?;
        let state_json = serde_json::to_string(&state)
            .map_err(|err| AuthError::Backend(format!("webauthn state: {err}")))?;
        Ok((options_json, state_json))
    }

    pub fn identify_discoverable(
        &self,
        credential: &Value,
    ) -> Result<IdentifiedPasskey, AuthError> {
        let assertion: PublicKeyCredential = serde_json::from_value(credential.clone())
            .map_err(|_| AuthError::InvalidCredentials)?;
        let (public_id, credential_id) = self
            .webauthn
            .identify_discoverable_authentication(&assertion)
            .map_err(ceremony_err)?;
        Ok(IdentifiedPasskey {
            public_id,
            credential_id: credential_id.to_vec(),
            assertion,
        })
    }

    pub fn finish_discoverable(
        &self,
        identified: &IdentifiedPasskey,
        state_json: &str,
        passkey: &Passkey,
    ) -> Result<AuthenticationResult, AuthError> {
        if passkey.cred_id().as_slice() != identified.credential_id.as_slice() {
            return Err(AuthError::InvalidCredentials);
        }
        let state: DiscoverableAuthentication =
            serde_json::from_str(state_json).map_err(|_| AuthError::InvalidToken)?;
        let key = DiscoverableKey::from(passkey);
        self.webauthn
            .finish_discoverable_authentication(&identified.assertion, state, &[key])
            .map_err(ceremony_err)
    }
}

pub struct IdentifiedPasskey {
    pub public_id: Uuid,
    pub credential_id: Vec<u8>,
    assertion: PublicKeyCredential,
}

fn require_discoverable_credential(options: &mut Value) {
    let Some(public_key) = options.get_mut("publicKey").and_then(Value::as_object_mut) else {
        return;
    };
    let selection = public_key
        .entry("authenticatorSelection")
        .or_insert_with(|| Value::Object(Map::new()));
    let Some(selection) = selection.as_object_mut() else {
        return;
    };
    selection.insert(
        "residentKey".to_string(),
        Value::String("required".to_string()),
    );
    selection.insert("requireResidentKey".to_string(), Value::Bool(true));
}

fn require_direct_attestation(options: &mut Value) {
    let Some(public_key) = options.get_mut("publicKey").and_then(Value::as_object_mut) else {
        return;
    };
    public_key.insert(
        "attestation".to_string(),
        Value::String("direct".to_string()),
    );
}

pub fn passkey_json(passkey: &Passkey) -> Result<String, AuthError> {
    serde_json::to_string(passkey).map_err(|err| AuthError::Backend(format!("passkey json: {err}")))
}

pub fn passkey_from_json(json: &str) -> Result<Passkey, AuthError> {
    serde_json::from_str(json).map_err(|_| AuthError::Backend("stored passkey is invalid".into()))
}

/// AAGUID from `response.attestationObject` authData (attestedCredentialData).
/// Works for `fmt: "none"` as well as packed/TPM. Nil AAGUIDs are treated as absent.
pub fn aaguid_from_attestation_object(credential: &Value) -> Option<Uuid> {
    const AT_FLAG: u8 = 0x40;
    const AUTH_DATA_FIXED: usize = 32 + 1 + 4;
    const AAGUID_LEN: usize = 16;

    let b64 = credential
        .get("response")?
        .get("attestationObject")?
        .as_str()?;
    let bytes = URL_SAFE_NO_PAD.decode(b64).ok()?;
    let cbor: CborValue = serde_cbor_2::from_slice(&bytes).ok()?;
    let CborValue::Map(map) = cbor else {
        return None;
    };
    let auth_data = map.iter().find_map(|(key, value)| match (key, value) {
        (CborValue::Text(name), CborValue::Bytes(data)) if name == "authData" => {
            Some(data.as_slice())
        }
        _ => None,
    })?;
    if auth_data.len() < AUTH_DATA_FIXED + AAGUID_LEN {
        return None;
    }
    if auth_data[32] & AT_FLAG == 0 {
        return None;
    }
    let aaguid = Uuid::from_slice(&auth_data[AUTH_DATA_FIXED..AUTH_DATA_FIXED + AAGUID_LEN]).ok()?;
    (aaguid != Uuid::nil()).then_some(aaguid)
}

pub fn passkey_credential_id(passkey: &Passkey) -> Vec<u8> {
    passkey.cred_id().to_vec()
}

pub fn stored_credential_id(passkey: &Passkey) -> String {
    credential_id_key(passkey.cred_id())
}

pub fn authentication_credential_id(result: &AuthenticationResult) -> String {
    credential_id_key(result.cred_id())
}

fn ceremony_err(err: impl std::fmt::Display) -> AuthError {
    eprintln!("webauthn ceremony failed");
    let _ = err;
    AuthError::InvalidCredentials
}

fn env_or(state: &AppState, name: &str, default: &str) -> String {
    match state.secret(name) {
        Ok(value) if !value.trim().is_empty() => value,
        _ => default.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::aaguid_from_attestation_object;
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use serde_cbor_2::Value as CborValue;
    use serde_json::json;
    use std::collections::BTreeMap;
    use uuid::Uuid;

    fn credential_with_auth_data(auth_data: Vec<u8>) -> serde_json::Value {
        let mut map = BTreeMap::new();
        map.insert(
            CborValue::Text("fmt".into()),
            CborValue::Text("none".into()),
        );
        map.insert(
            CborValue::Text("authData".into()),
            CborValue::Bytes(auth_data),
        );
        map.insert(
            CborValue::Text("attStmt".into()),
            CborValue::Map(BTreeMap::new()),
        );
        let encoded = URL_SAFE_NO_PAD.encode(serde_cbor_2::to_vec(&CborValue::Map(map)).unwrap());
        json!({
            "response": {
                "attestationObject": encoded,
            }
        })
    }

    fn auth_data_with_aaguid(aaguid: [u8; 16], attested: bool) -> Vec<u8> {
        let mut auth_data = vec![0u8; 37];
        if attested {
            auth_data[32] = 0x45; // UP | UV | AT
            auth_data.extend_from_slice(&aaguid);
            auth_data.extend_from_slice(&0u16.to_be_bytes());
        }
        auth_data
    }

    #[test]
    fn reads_aaguid_from_none_attestation_auth_data() {
        let id = Uuid::parse_str("fbfc3007-154e-4ecc-8c0b-6e020557d7bd").unwrap();
        let credential = credential_with_auth_data(auth_data_with_aaguid(*id.as_bytes(), true));
        assert_eq!(aaguid_from_attestation_object(&credential), Some(id));
    }

    #[test]
    fn ignores_nil_aaguid_and_missing_at_flag() {
        let nil = credential_with_auth_data(auth_data_with_aaguid([0; 16], true));
        assert_eq!(aaguid_from_attestation_object(&nil), None);

        let no_at = credential_with_auth_data(auth_data_with_aaguid(
            *Uuid::parse_str("fbfc3007-154e-4ecc-8c0b-6e020557d7bd")
                .unwrap()
                .as_bytes(),
            false,
        ));
        assert_eq!(aaguid_from_attestation_object(&no_at), None);

        assert_eq!(aaguid_from_attestation_object(&json!({})), None);
    }
}
