//! The release key's public half, compiled into the app, and the check of
//! a signature with it.
//!
//! The format is minisign's (Ed25519 over a BLAKE2b-512 prehash, with a
//! signed "trusted comment"): ADR 0008 chose it for `vesseld` releases,
//! Tauri's updater uses it, and `minisign -V` lets anyone check a file by
//! hand. Only prehashed signatures are accepted; minisign's legacy mode
//! (Ed25519 over the raw file) is refused, as nothing of Vessel's was
//! ever signed with it.

use std::fmt;

/// A minisign public key: the base64 line of a `.pub` file (`RW...`).
#[derive(Clone)]
pub struct PublicKey {
    key: minisign_verify::PublicKey,
    text: String,
}

/// Why a key or a signature was not accepted.
#[derive(Debug, thiserror::Error)]
pub enum KeyError {
    /// The text is not a minisign public key.
    #[error("not a minisign public key")]
    Key(#[source] minisign_verify::Error),
    /// The text is not a minisign signature.
    #[error("not a minisign signature")]
    Signature(#[source] minisign_verify::Error),
    /// A signature, but not by this key over these bytes.
    #[error("the signature does not verify with this key")]
    Verify(#[source] minisign_verify::Error),
}

impl PublicKey {
    /// Parse the key from its base64 line.
    ///
    /// # Errors
    /// [`KeyError::Key`] when it is not one.
    pub fn from_base64(text: &str) -> Result<Self, KeyError> {
        let text = text.trim();
        let key = minisign_verify::PublicKey::from_base64(text).map_err(KeyError::Key)?;
        Ok(Self {
            key,
            text: text.to_owned(),
        })
    }

    /// Verify that `signature` (a `.minisig` file's text) signs `bytes`.
    ///
    /// # Errors
    /// [`KeyError::Signature`] when it is not a signature,
    /// [`KeyError::Verify`] when it is not this key's over `bytes`.
    pub fn verify(&self, bytes: &[u8], signature: &str) -> Result<(), KeyError> {
        let signature =
            minisign_verify::Signature::decode(signature).map_err(KeyError::Signature)?;
        self.key
            .verify(bytes, &signature, false)
            .map_err(KeyError::Verify)
    }

    /// The base64 line, as given.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("PublicKey").field(&self.text).finish()
    }
}

impl PartialEq for PublicKey {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
    }
}

impl Eq for PublicKey {}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::manifest::{Envelope, Manifest};

    /// A key pair made for one test, never written anywhere.
    pub(crate) struct TestKey {
        pair: minisign::KeyPair,
    }

    impl TestKey {
        pub(crate) fn new() -> Self {
            Self {
                pair: minisign::KeyPair::generate_unencrypted_keypair().unwrap(),
            }
        }

        pub(crate) fn public(&self) -> PublicKey {
            PublicKey::from_base64(&self.pair.pk.to_base64()).unwrap()
        }

        pub(crate) fn sign(&self, bytes: &[u8]) -> String {
            minisign::sign(Some(&self.pair.pk), &self.pair.sk, bytes, None, None)
                .unwrap()
                .into_string()
        }

        /// A `<channel>.json` holding `manifest`, signed with this key.
        pub(crate) fn envelope(&self, manifest: &Manifest) -> Vec<u8> {
            let manifest = serde_json::to_string(manifest).unwrap();
            let signature = self.sign(manifest.as_bytes());
            serde_json::to_vec(&Envelope {
                manifest,
                signature,
            })
            .unwrap()
        }
    }

    #[test]
    fn a_signature_verifies_with_its_key_only() {
        let key = TestKey::new();
        let signature = key.sign(b"vessel");
        key.public().verify(b"vessel", &signature).unwrap();
        assert!(matches!(
            key.public().verify(b"vessel!", &signature),
            Err(KeyError::Verify(_))
        ));
        assert!(matches!(
            TestKey::new().public().verify(b"vessel", &signature),
            Err(KeyError::Verify(_))
        ));
        assert!(matches!(
            key.public().verify(b"vessel", "not a signature"),
            Err(KeyError::Signature(_))
        ));
        assert!(matches!(
            PublicKey::from_base64("RWQnot-a-key"),
            Err(KeyError::Key(_))
        ));
    }
}
