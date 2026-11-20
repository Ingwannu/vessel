//! Opaque cache association for the credential actually cached by this
//! Credentials instance. Pointer identity avoids storing token-derived
//! hashes and cannot be reused while a page retains its marker.

use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use vessel_proto::forge::ForgeHost;

/// A credential-cache generation, for isolating cached forge facts. It
/// contains no token and exposes no pointer, serializable id or text.
#[derive(Clone)]
pub struct CredentialIdentity {
    host: ForgeHost,
    loaded: Arc<()>,
    invalidated: Arc<()>,
}

impl CredentialIdentity {
    pub(super) fn new(host: ForgeHost, loaded: Arc<()>, invalidated: Arc<()>) -> Self {
        Self {
            host,
            loaded,
            invalidated,
        }
    }
}

impl PartialEq for CredentialIdentity {
    fn eq(&self, other: &Self) -> bool {
        self.host == other.host
            && Arc::ptr_eq(&self.loaded, &other.loaded)
            && Arc::ptr_eq(&self.invalidated, &other.invalidated)
    }
}

impl Eq for CredentialIdentity {}

impl Hash for CredentialIdentity {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.host.hash(state);
        std::ptr::hash(Arc::as_ptr(&self.loaded), state);
        std::ptr::hash(Arc::as_ptr(&self.invalidated), state);
    }
}

impl fmt::Debug for CredentialIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CredentialIdentity(..)")
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::super::{Credentials, EnvTokens, Token};
    use super::*;

    #[test]
    fn generations_compare_allocations_instead_of_equal_unit_values() {
        let host: ForgeHost = "forge.example.invalid".parse().unwrap();
        let first = CredentialIdentity::new(host.clone(), Arc::new(()), Arc::new(()));
        let same = first.clone();
        let loaded =
            CredentialIdentity::new(host.clone(), Arc::new(()), Arc::clone(&first.invalidated));
        let refused = CredentialIdentity::new(host, Arc::clone(&first.loaded), Arc::new(()));
        let keys = HashSet::from([first.clone(), same, loaded, refused]);
        assert_eq!(keys.len(), 3);
        assert_eq!(format!("{first:?}"), "CredentialIdentity(..)");
    }

    #[tokio::test]
    async fn configured_identity_is_host_bound_and_refusals_always_invalidate_it() {
        let github: ForgeHost = "github.com".parse().unwrap();
        let enterprise: ForgeHost = "forge.example.invalid".parse().unwrap();
        let credentials = Credentials::new(
            EnvTokens {
                github: Token::new("fixture-configured"),
                enterprise: Some((
                    enterprise.clone(),
                    Token::new("fixture-enterprise").unwrap(),
                )),
            },
            None,
        );
        let initial = credentials.identity(&github).await.unwrap();
        assert_eq!(credentials.identity(&github).await.as_ref(), Some(&initial));
        assert_ne!(credentials.identity(&enterprise).await.unwrap(), initial);
        assert!(
            credentials
                .identity(&"other.example.invalid".parse().unwrap())
                .await
                .is_none()
        );
        credentials.refused(&github);
        let refused = credentials.identity(&github).await.unwrap();
        assert_ne!(refused, initial);
        // gh's retry cooldown must not let a known second refusal reuse a
        // page admitted before that refusal. No extra gh retry is added.
        credentials.refused(&github);
        assert_ne!(credentials.identity(&github).await.unwrap(), refused);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn gh_identity_changes_when_the_existing_token_cache_actually_refreshes() {
        let temp = tempfile::tempdir().unwrap();
        let program = temp.path().join("gh");
        vessel_testkit::write_executable(&program, "#!/bin/sh\ncat credential-fixture\n").unwrap();
        let data = temp.path().join("credential-fixture");
        std::fs::write(&data, "fixture-first").unwrap();
        let credentials = Credentials::new(
            EnvTokens::default(),
            Some(crate::Gh::new(program, temp.path().to_path_buf())),
        );
        let host: ForgeHost = "github.com".parse().unwrap();
        let first = credentials.identity(&host).await.unwrap();
        credentials.cache().get_mut(&host).unwrap().until = tokio::time::Instant::now();
        assert_eq!(
            credentials.identity(&host).await.as_ref(),
            Some(&first),
            "an unchanged credential refresh keeps its identity"
        );
        std::fs::write(&data, "fixture-second").unwrap();
        assert_eq!(
            credentials.identity(&host).await.as_ref(),
            Some(&first),
            "the existing cache lifetime still applies"
        );
        credentials.cache().get_mut(&host).unwrap().until = tokio::time::Instant::now();
        let second = credentials.identity(&host).await.unwrap();
        assert_ne!(second, first);
        let super::super::Found::Token(token, _) = credentials.token(&host).await else {
            panic!("fake gh has a token")
        };
        assert!(token.same_as(&Token::new("fixture-second").unwrap()));
        credentials.refused(&host);
        assert_ne!(credentials.identity(&host).await.unwrap(), second);
    }
}
