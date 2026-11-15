//! A real subscriber decrypts the payload, and an independent verifier
//! checks the RFC 8292 signature and complete claims.
use crate::push::crypto::{Crypto, PAYLOAD};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::Deserialize;
use vessel_proto::push::{PushAuth, PushPublicKey, PushSubscription};
use web_push_native::{
    Auth,
    p256::{
        SecretKey,
        ecdsa::{Signature, VerifyingKey, signature::Verifier as _},
        elliptic_curve::sec1::ToEncodedPoint as _,
    },
};

pub(super) fn subscriber() -> (SecretKey, PushSubscription) {
    let key = SecretKey::from_slice(&[7_u8; 32]).unwrap();
    let subscription = PushSubscription {
        endpoint: "https://push.example/subscription-capability"
            .parse()
            .unwrap(),
        p256dh: PushPublicKey::from_bytes(key.public_key().to_encoded_point(false).as_bytes())
            .unwrap(),
        auth: PushAuth::from_bytes([9_u8; 16]),
    };
    (key, subscription)
}
#[derive(Debug, Deserialize, PartialEq, Eq)]
struct Header {
    typ: String,
    alg: String,
}
#[derive(Debug, Deserialize, PartialEq, Eq)]
struct Claims {
    aud: String,
    exp: i64,
    sub: String,
}
#[test]
fn encrypted_payload_and_signed_vapid_envelope_are_standard_and_content_free() {
    let temp = tempfile::tempdir().unwrap();
    let dir = vessel_store::DataDir::new(temp.path());
    dir.prepare().unwrap();
    let crypto = Crypto::load(&dir).unwrap();
    let (secret, subscription) = subscriber();
    let now = chrono::DateTime::from_timestamp(1_700_000_000, 0).unwrap();
    let request = crypto.encrypt_at(&subscription, now).unwrap();
    assert_eq!(
        request.headers().get("Content-Encoding").unwrap(),
        "aes128gcm"
    );
    assert_eq!(request.headers().get("TTL").unwrap(), "60");
    assert_ne!(request.body(), PAYLOAD);
    assert!(
        !request
            .body()
            .windows(PAYLOAD.len())
            .any(|window| window == PAYLOAD)
    );
    let decrypted = web_push_native::decrypt(
        request.body().clone(),
        &secret,
        &Auth::from(*subscription.auth.expose()),
    )
    .unwrap();
    assert_eq!(decrypted, PAYLOAD);
    let auth = request.headers().get("Authorization").unwrap();
    assert!(auth.is_sensitive());
    let auth = auth.to_str().unwrap();
    let (token, public) = auth
        .strip_prefix("vapid t=")
        .unwrap()
        .split_once(", k=")
        .unwrap();
    assert_eq!(
        URL_SAFE_NO_PAD.decode(public).unwrap(),
        crypto.public().key.as_bytes()
    );
    let parts: Vec<_> = token.split('.').collect();
    assert_eq!(parts.len(), 3);
    let header: Header =
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[0]).unwrap()).unwrap();
    assert_eq!(
        header,
        Header {
            typ: "JWT".to_owned(),
            alg: "ES256".to_owned()
        }
    );
    let claims: Claims =
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[1]).unwrap()).unwrap();
    assert_eq!(claims.aud, "https://push.example");
    assert_eq!(claims.sub, "https://github.com/Team-WICKED/vessel");
    assert_eq!(claims.exp, 1_700_000_300);
    let signature = Signature::from_slice(&URL_SAFE_NO_PAD.decode(parts[2]).unwrap()).unwrap();
    let public = VerifyingKey::from_sec1_bytes(crypto.public().key.as_bytes()).unwrap();
    public
        .verify(format!("{}.{}", parts[0], parts[1]).as_bytes(), &signature)
        .unwrap();
    assert!(public.verify(b"tampered", &signature).is_err());
    let wrong = Auth::from([8_u8; 16]);
    assert!(web_push_native::decrypt(request.body().clone(), &secret, &wrong).is_err());
}
#[test]
fn key_is_persistent_private_and_debug_output_never_contains_capability_material() {
    let temp = tempfile::tempdir().unwrap();
    let dir = vessel_store::DataDir::new(temp.path());
    dir.prepare().unwrap();
    let first = Crypto::load(&dir).unwrap();
    let second = Crypto::load(&dir).unwrap();
    assert_eq!(first.public(), second.public());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        assert_eq!(
            std::fs::metadata(temp.path().join("push-vapid.key"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        assert_eq!(
            std::fs::metadata(temp.path()).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }
    let (_, subscription) = subscriber();
    assert_eq!(
        format!("{:?}", subscription.endpoint),
        "PushEndpoint(<redacted>)"
    );
    assert_eq!(format!("{:?}", subscription.auth), "PushAuth(<redacted>)");
    assert_eq!(format!("{first:?}"), "PushCrypto(<redacted>)");
}
