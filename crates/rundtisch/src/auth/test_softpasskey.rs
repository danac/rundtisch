//! In-process P-256 passkey used by handler tests.
//!
//! `webauthn-authenticator-rs` needs `tokio ^1.47.2`, which the Wasmer crate
//! index does not publish. `rundtisch` is a workspace member, so that
//! dev-dependency is part of the shared lockfile even when the selected
//! package is the demo binary. This helper uses `p256` and `serde_cbor_2`,
//! which `webauthn-rs` already requires.

use std::collections::{BTreeMap, HashMap};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use p256::ecdsa::Signature;
use p256::ecdsa::SigningKey;
use p256::ecdsa::signature::Signer;
use serde::Serialize;
use serde_cbor_2::Value;
use sha2::Digest;
use sha2::Sha256;
use webauthn_rs::prelude::{
    CreationChallengeResponse, PublicKeyCredential, RegisterPublicKeyCredential,
    RequestChallengeResponse, Url,
};
use webauthn_rs_proto::{
    AllowCredentials, AuthenticationExtensionsClientOutputs, AuthenticatorAssertionResponseRaw,
    AuthenticatorAttestationResponseRaw, RegistrationExtensionsClientOutputs,
};

pub struct SoftPasskey {
    keys: HashMap<Vec<u8>, SigningKey>,
    counter: u32,
}

impl SoftPasskey {
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
            counter: 0,
        }
    }

    pub fn do_registration(
        &mut self,
        origin: Url,
        options: CreationChallengeResponse,
    ) -> Result<RegisterPublicKeyCredential, String> {
        let options = options.public_key;
        let client_data = client_data_json("webauthn.create", &options.challenge, &origin)?;
        let client_hash = Sha256::digest(&client_data).to_vec();
        let rp_hash = Sha256::digest(options.rp.id.as_bytes()).to_vec();
        let credential_id = random_bytes(32)?;
        let signing_key = random_signing_key()?;
        let (x, y) = public_coords(&signing_key);
        let public_key = cose_public_key(&x, &y)?;
        let auth_data = authenticator_data(&rp_hash, 0x45, 0, Some((&credential_id, &public_key)));
        let signature = sign(&signing_key, &auth_data, &client_hash);
        let attestation = attestation_object(&auth_data, &signature)?;
        self.keys.insert(credential_id.clone(), signing_key);
        Ok(RegisterPublicKeyCredential {
            id: URL_SAFE_NO_PAD.encode(&credential_id),
            raw_id: credential_id,
            response: AuthenticatorAttestationResponseRaw {
                attestation_object: attestation,
                client_data_json: client_data,
                transports: None,
            },
            type_: "public-key".to_string(),
            extensions: RegistrationExtensionsClientOutputs::default(),
        })
    }

    pub fn do_authentication(
        &mut self,
        origin: Url,
        options: RequestChallengeResponse,
    ) -> Result<PublicKeyCredential, String> {
        let options = options.public_key;
        let client_data = client_data_json("webauthn.get", &options.challenge, &origin)?;
        let client_hash = Sha256::digest(&client_data).to_vec();
        let rp_hash = Sha256::digest(options.rp_id.as_bytes()).to_vec();
        let credential_id = self.lookup_id(&options.allow_credentials)?;
        self.counter = self.counter.wrapping_add(1);
        let counter = self.counter;
        let auth_data = authenticator_data(&rp_hash, 0x05, counter, None);
        let signing_key = self.keys.get(&credential_id).expect("key stored");
        let signature = sign(signing_key, &auth_data, &client_hash);
        Ok(PublicKeyCredential {
            id: URL_SAFE_NO_PAD.encode(&credential_id),
            raw_id: credential_id,
            response: AuthenticatorAssertionResponseRaw {
                authenticator_data: auth_data,
                client_data_json: client_data,
                signature,
                user_handle: None,
            },
            type_: "public-key".to_string(),
            extensions: AuthenticationExtensionsClientOutputs::default(),
        })
    }

    fn lookup_id(&self, allow: &[AllowCredentials]) -> Result<Vec<u8>, String> {
        if allow.is_empty() {
            return self.keys.keys().next().cloned().ok_or("no passkey".into());
        }
        allow
            .iter()
            .find(|credential| self.keys.contains_key(&credential.id))
            .map(|credential| credential.id.clone())
            .ok_or_else(|| "credential not found".into())
    }
}

#[derive(Serialize)]
struct ClientData<'a> {
    #[serde(rename = "type")]
    type_: &'a str,
    challenge: &'a str,
    origin: &'a str,
}

fn client_data_json(type_: &str, challenge: &[u8], origin: &Url) -> Result<Vec<u8>, String> {
    let challenge = URL_SAFE_NO_PAD.encode(challenge);
    serde_json::to_vec(&ClientData {
        type_,
        challenge: &challenge,
        origin: origin.as_str(),
    })
    .map_err(|err| err.to_string())
}

fn random_bytes(len: usize) -> Result<Vec<u8>, String> {
    let mut bytes = vec![0u8; len];
    getrandom::fill(&mut bytes).map_err(|err| err.to_string())?;
    Ok(bytes)
}

fn random_signing_key() -> Result<SigningKey, String> {
    loop {
        let bytes = random_bytes(32)?;
        if let Ok(key) = SigningKey::from_slice(&bytes) {
            return Ok(key);
        }
    }
}

fn public_coords(key: &SigningKey) -> (Vec<u8>, Vec<u8>) {
    let point = key.verifying_key().to_encoded_point(false);
    (
        point.x().expect("x").to_vec(),
        point.y().expect("y").to_vec(),
    )
}

fn cose_public_key(x: &[u8], y: &[u8]) -> Result<Vec<u8>, String> {
    let mut map = BTreeMap::new();
    map.insert(Value::Integer(1), Value::Integer(2));
    map.insert(Value::Integer(3), Value::Integer(-7));
    map.insert(Value::Integer(-1), Value::Integer(1));
    map.insert(Value::Integer(-2), Value::Bytes(x.to_vec()));
    map.insert(Value::Integer(-3), Value::Bytes(y.to_vec()));
    serde_cbor_2::to_vec(&Value::Map(map)).map_err(|err| err.to_string())
}

fn authenticator_data(
    rp_hash: &[u8],
    flags: u8,
    counter: u32,
    attested: Option<(&[u8], &[u8])>,
) -> Vec<u8> {
    let mut data = Vec::with_capacity(
        37 + attested
            .map(|(id, pk)| 18 + id.len() + pk.len())
            .unwrap_or(0),
    );
    data.extend_from_slice(rp_hash);
    data.push(flags);
    data.extend_from_slice(&counter.to_be_bytes());
    if let Some((credential_id, public_key)) = attested {
        data.extend_from_slice(&[0u8; 16]);
        let len = u16::try_from(credential_id.len()).expect("credential id fits u16");
        data.extend_from_slice(&len.to_be_bytes());
        data.extend_from_slice(credential_id);
        data.extend_from_slice(public_key);
    }
    data
}

fn sign(key: &SigningKey, auth_data: &[u8], client_hash: &[u8]) -> Vec<u8> {
    let mut message = Vec::with_capacity(auth_data.len() + client_hash.len());
    message.extend_from_slice(auth_data);
    message.extend_from_slice(client_hash);
    let signature: Signature = key.sign(&message);
    signature.to_der().as_bytes().to_vec()
}

fn attestation_object(auth_data: &[u8], signature: &[u8]) -> Result<Vec<u8>, String> {
    let mut statement = BTreeMap::new();
    statement.insert(Value::Text("alg".to_string()), Value::Integer(-7));
    statement.insert(
        Value::Text("sig".to_string()),
        Value::Bytes(signature.to_vec()),
    );
    let mut object = BTreeMap::new();
    object.insert(
        Value::Text("fmt".to_string()),
        Value::Text("packed".to_string()),
    );
    object.insert(Value::Text("attStmt".to_string()), Value::Map(statement));
    object.insert(
        Value::Text("authData".to_string()),
        Value::Bytes(auth_data.to_vec()),
    );
    serde_cbor_2::to_vec(&Value::Map(object)).map_err(|err| err.to_string())
}
