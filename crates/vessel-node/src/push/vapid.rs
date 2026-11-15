//! The small RFC 8292 JWT envelope. JSON and base64url are serialization;
//! ES256 and SHA-256 are `RustCrypto`'s p256 signer, never code of our own.
//!
//! [Decision Log] The optional all-algorithms JWT layer of web-push-native
//! added RSA, ML-DSA and unrelated ciphers to a node needing only P-256.
//! Keep encryption in that library and build this typed, three-claim JWT
//! envelope with the same p256 key; tests verify the signature and all claims.
use axum::http::HeaderValue;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, TimeDelta, Utc};
use serde::Serialize;
use vessel_proto::push::{PushEndpoint, PushPublicKey};
use web_push_native::p256::ecdsa::{Signature, SigningKey, signature::Signer as _};

#[derive(Debug, thiserror::Error)]
pub(crate) enum VapidError {
    #[error("cannot encode the push identity claims")]
    Encode(#[source] serde_json::Error),
    #[error("cannot sign the push identity")]
    Sign(#[source] web_push_native::p256::ecdsa::Error),
    #[error("cannot encode the push identity header")]
    Header(#[source] axum::http::header::InvalidHeaderValue),
    #[error("cannot read the push endpoint origin")]
    Endpoint(#[source] url::ParseError),
    #[error("the node's clock cannot represent the push identity expiry")]
    Clock,
}
#[derive(Serialize)]
struct Header {
    typ: &'static str,
    alg: &'static str,
}
#[derive(Serialize)]
struct Claims<'a> {
    aud: String,
    exp: i64,
    sub: &'a str,
}

pub(super) fn authorization(
    key: &SigningKey,
    public: &PushPublicKey,
    endpoint: &PushEndpoint,
    now: DateTime<Utc>,
) -> Result<HeaderValue, VapidError> {
    let expires = now
        .checked_add_signed(TimeDelta::minutes(5))
        .ok_or(VapidError::Clock)?;
    let origin = url::Url::parse(endpoint.expose())
        .map_err(VapidError::Endpoint)?
        .origin()
        .ascii_serialization();
    let header = URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&Header {
            typ: "JWT",
            alg: "ES256",
        })
        .map_err(VapidError::Encode)?,
    );
    let claims = URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&Claims {
            aud: origin,
            exp: expires.timestamp(),
            sub: "https://github.com/Team-WICKED/vessel",
        })
        .map_err(VapidError::Encode)?,
    );
    let input = format!("{header}.{claims}");
    let signature: Signature = key.try_sign(input.as_bytes()).map_err(VapidError::Sign)?;
    let token = format!("{input}.{}", URL_SAFE_NO_PAD.encode(signature.to_bytes()));
    let mut header = HeaderValue::from_str(&format!(
        "vapid t={token}, k={}",
        URL_SAFE_NO_PAD.encode(public.as_bytes())
    ))
    .map_err(VapidError::Header)?;
    header.set_sensitive(true);
    Ok(header)
}
