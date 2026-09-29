use std::time::Duration;

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
        let state_json = serde_json::to_string(&state)
            .map_err(|err| AuthError::Backend(format!("webauthn state: {err}")))?;
        Ok((options_json, state_json))
    }

    pub fn finish_registration(
        &self,
        credential: &Value,
        state_json: &str,
    ) -> Result<Passkey, AuthError> {
        let reg: RegisterPublicKeyCredential = serde_json::from_value(credential.clone())
            .map_err(|_| AuthError::InvalidCredentials)?;
        let state: PasskeyRegistration =
            serde_json::from_str(state_json).map_err(|_| AuthError::InvalidToken)?;
        self.webauthn
            .finish_passkey_registration(&reg, &state)
            .map_err(ceremony_err)
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

pub fn passkey_json(passkey: &Passkey) -> Result<String, AuthError> {
    serde_json::to_string(passkey).map_err(|err| AuthError::Backend(format!("passkey json: {err}")))
}

pub fn passkey_from_json(json: &str) -> Result<Passkey, AuthError> {
    serde_json::from_str(json).map_err(|_| AuthError::Backend("stored passkey is invalid".into()))
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
