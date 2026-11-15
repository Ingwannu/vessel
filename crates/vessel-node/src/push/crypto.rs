//! RFC 8291 encryption and RFC 8292 signing are delegated to `RustCrypto`
//! through web-push-native; this module never implements cryptography.
use super::delivery::Delivery;
use axum::http::Request;
use std::fmt;
use vessel_proto::push::{PushKey, PushPublicKey, PushSubscription};
use vessel_store::{DataDir, PushKeyError};
use web_push_native::p256::ecdsa::SigningKey;
use web_push_native::{Auth, WebPushBuilder, p256};

/// Constant payload: even the recipient learns content only after reconnecting.
pub(crate) const PAYLOAD: &[u8] = br#"{"v":1,"type":"attention"}"#;

#[derive(Debug, thiserror::Error)]
pub(crate) enum CryptoError {
    #[error(transparent)]
    KeyFile(#[from] PushKeyError),
    #[error("the stored private push key is not valid; restore it or subscribe devices again")]
    PrivateKey(#[source] p256::ecdsa::Error),
    #[error("the push public key is not a valid P-256 point; subscribe again")]
    PublicKey(#[source] p256::elliptic_curve::Error),
    #[error("the push key could not be encoded")]
    KeyShape(#[source] vessel_proto::push::InvalidPushMaterial),
    #[error("the push endpoint could not be encoded")]
    Endpoint(#[source] axum::http::uri::InvalidUri),
    #[error("the push TLS configuration could not be prepared")]
    Tls(#[source] rustls::Error),
    #[error(transparent)]
    Vapid(#[from] super::vapid::VapidError),
    #[error("the push payload could not be encrypted")]
    Encrypt(#[source] web_push_native::Error),
}
pub(crate) struct Crypto {
    key: SigningKey,
    public: PushPublicKey,
    pub(crate) delivery: Delivery,
}
impl fmt::Debug for Crypto {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PushCrypto(<redacted>)")
    }
}
impl Crypto {
    pub(crate) fn load(dir: &DataDir) -> Result<Self, CryptoError> {
        let private = dir.load_or_create_push_key()?;
        let key = SigningKey::from_slice(private.expose()).map_err(CryptoError::PrivateKey)?;
        let public =
            PushPublicKey::from_bytes(key.verifying_key().to_encoded_point(false).as_bytes())
                .map_err(CryptoError::KeyShape)?;
        Ok(Self {
            key,
            public,
            delivery: Delivery::new().map_err(CryptoError::Tls)?,
        })
    }
    #[cfg(test)]
    pub(crate) fn test_at(mut self, origin: url::Url) -> Self {
        self.delivery = self.delivery.test_at(origin);
        self
    }
    pub(crate) fn public(&self) -> PushKey {
        PushKey {
            key: self.public.clone(),
        }
    }
    pub(crate) fn validate(subscription: &PushSubscription) -> Result<(), CryptoError> {
        let _public = p256::PublicKey::from_sec1_bytes(subscription.p256dh.as_bytes())
            .map_err(CryptoError::PublicKey)?;
        Ok(())
    }
    pub(crate) fn encrypt(
        &self,
        subscription: &PushSubscription,
    ) -> Result<Request<Vec<u8>>, CryptoError> {
        self.encrypt_at(subscription, chrono::Utc::now())
    }
    pub(super) fn encrypt_at(
        &self,
        subscription: &PushSubscription,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Request<Vec<u8>>, CryptoError> {
        let public = p256::PublicKey::from_sec1_bytes(subscription.p256dh.as_bytes())
            .map_err(CryptoError::PublicKey)?;
        let auth = Auth::from(*subscription.auth.expose());
        let endpoint = subscription
            .endpoint
            .expose()
            .parse()
            .map_err(CryptoError::Endpoint)?;
        let mut request = WebPushBuilder::new(endpoint, public, auth)
            .with_valid_duration(std::time::Duration::from_secs(60))
            .build(PAYLOAD)
            .map_err(CryptoError::Encrypt)?;
        let authorization =
            super::vapid::authorization(&self.key, &self.public, &subscription.endpoint, now)?;
        let _old = request
            .headers_mut()
            .insert(axum::http::header::AUTHORIZATION, authorization);
        Ok(request)
    }
}
