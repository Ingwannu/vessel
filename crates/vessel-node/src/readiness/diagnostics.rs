//! Diagnostic source used by the bounded node collector (ADR 0074).
//! Source owners omit private paths, arbitrary text and credentials.
use super::{Handshake, Key, Readiness};
use chrono::{DateTime, Utc};
use std::sync::PoisonError;
use vessel_proto::{
    agents::{SignedIn, UnknownReason},
    diagnostics::{
        AcpProtocolVersion, DiagnosticAgentRefusal, DiagnosticSignIn, DiagnosticVersion,
    },
};

/// Safe cache projection; absence is not a request to run a refresh.
#[derive(Debug)]
pub(crate) struct Cached {
    pub(crate) sign_in: DiagnosticSignIn,
    pub(crate) version: Option<DiagnosticVersion>,
    pub(crate) refusal: Option<DiagnosticAgentRefusal>,
    pub(crate) at: DateTime<Utc>,
    pub(crate) stale: bool,
}
impl Readiness {
    /// Historical facts stay explicitly stale without evicting/refreshing
    /// the ordinary readiness cache or cloning its private strings.
    pub(crate) fn diagnostic(&self, key: &Key, observed_at: DateTime<Utc>) -> Option<Cached> {
        let kept = self.kept.lock().unwrap_or_else(PoisonError::into_inner);
        let checked = kept.get(key)?;
        let stale = observed_at > checked.until;
        let sign_in = if stale {
            DiagnosticSignIn::Unknown {
                reason: UnknownReason::NotChecked,
            }
        } else {
            match &checked.signed_in {
                SignedIn::Yes => DiagnosticSignIn::Present,
                SignedIn::No { .. } => DiagnosticSignIn::Missing,
                SignedIn::NotRequired => DiagnosticSignIn::NotRequired,
                SignedIn::Unknown { reason } => DiagnosticSignIn::Unknown {
                    reason: reason.clone(),
                },
            }
        };
        let (introduction, refusal) = match &checked.handshake {
            Some(Handshake::Spoke(introduction)) => (Some(introduction), None),
            Some(Handshake::OtherVersion {
                speaks,
                introduction,
            }) => {
                let needs = vessel_agent::SPEAKS;
                let refusal = if stale {
                    None
                } else if *speaks < needs {
                    Some(DiagnosticAgentRefusal::TooOld {
                        speaks: AcpProtocolVersion::from(*speaks),
                        needs: AcpProtocolVersion::from(needs),
                    })
                } else {
                    Some(DiagnosticAgentRefusal::TooNew {
                        speaks: AcpProtocolVersion::from(*speaks),
                        needs: AcpProtocolVersion::from(needs),
                    })
                };
                (Some(introduction), refusal)
            }
            None => (None, None),
        };
        let version = introduction
            .and_then(|said| said.version.as_ref())
            .and_then(|version| version.as_str().parse().ok());
        Some(Cached {
            sign_in,
            version,
            refusal,
            at: checked.at,
            stale,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::readiness::Checked;
    use vessel_agent::Introduction;
    use vessel_proto::{
        HarnessRef,
        agents::{AuthMethod, SignInHelp},
    };

    const PRIVATE_WORDS: &str = "private-diagnostic-source-canary";
    fn key() -> Key {
        Key {
            agent: HarnessRef::Added(PRIVATE_WORDS.parse().unwrap()),
            profile: PRIVATE_WORDS.parse().unwrap(),
        }
    }
    fn introduction() -> Introduction {
        Introduction {
            version: Some(format!("1.2.3-{PRIVATE_WORDS}").parse().unwrap()),
            methods: vec![AuthMethod {
                id: "login".parse().unwrap(),
                name: PRIVATE_WORDS.to_owned(),
                description: Some(PRIVATE_WORDS.to_owned()),
            }],
        }
    }
    fn checked(signed_in: SignedIn, handshake: Option<Handshake>) -> Checked {
        let at = DateTime::UNIX_EPOCH;
        let until = at.checked_add_signed(chrono::Duration::seconds(1)).unwrap();
        Checked {
            signed_in,
            handshake,
            at,
            until,
        }
    }
    #[test]
    fn projection_does_not_clone_help_names_or_version_suffixes() {
        let readiness = Readiness::default();
        let checked = checked(
            SignedIn::No {
                help: SignInHelp {
                    command: Some(format!("login {PRIVATE_WORDS}")),
                    methods: introduction().methods,
                    ..SignInHelp::default()
                },
            },
            Some(Handshake::Spoke(introduction())),
        );
        let at = checked.at;
        readiness.keep(key(), checked);
        let facts = readiness.diagnostic(&key(), at).unwrap();
        assert_eq!(facts.sign_in, DiagnosticSignIn::Missing);
        assert_eq!(facts.version.unwrap().to_string(), "1.2.3 (prerelease)");
        assert_eq!(facts.at, at);
        assert!(!facts.stale);
        assert!(facts.refusal.is_none());
        assert!(!format!("{facts:?}").contains(PRIVATE_WORDS));
    }
    #[test]
    fn stale_or_missing_never_claim_fresh_and_do_not_mutate_the_cache() {
        let readiness = Readiness::default();
        assert!(readiness.diagnostic(&key(), DateTime::UNIX_EPOCH).is_none());
        let checked = checked(
            SignedIn::Yes,
            Some(Handshake::OtherVersion {
                speaks: vessel_agent::SPEAKS.checked_add(1).unwrap(),
                introduction: introduction(),
            }),
        );
        let after = checked
            .until
            .checked_add_signed(chrono::Duration::seconds(1))
            .unwrap();
        readiness.keep(key(), checked);
        let facts = readiness.diagnostic(&key(), after).unwrap();
        assert!(facts.stale);
        assert_eq!(
            facts.sign_in,
            DiagnosticSignIn::Unknown {
                reason: UnknownReason::NotChecked
            }
        );
        assert!(facts.refusal.is_none());
        assert_eq!(readiness.kept.lock().unwrap().len(), 1);
    }
    #[test]
    fn fresh_protocol_refusals_keep_the_agent_domain_and_exact_numbers() {
        let readiness = Readiness::default();
        let needs = vessel_agent::SPEAKS;
        let older = needs.checked_sub(1).unwrap();
        let newer = needs.checked_add(1).unwrap();
        for speaks in [older, newer] {
            let value = checked(
                SignedIn::NotRequired,
                Some(Handshake::OtherVersion {
                    speaks,
                    introduction: introduction(),
                }),
            );
            let at = value.at;
            readiness.keep(key(), value);
            let facts = readiness.diagnostic(&key(), at).unwrap();
            assert_eq!(facts.sign_in, DiagnosticSignIn::NotRequired);
            let expected = if speaks < needs {
                DiagnosticAgentRefusal::TooOld {
                    speaks: speaks.into(),
                    needs: needs.into(),
                }
            } else {
                DiagnosticAgentRefusal::TooNew {
                    speaks: speaks.into(),
                    needs: needs.into(),
                }
            };
            assert_eq!(facts.refusal, Some(expected));
        }
    }
    #[test]
    fn unknown_sign_in_and_unread_versions_are_not_guessed() {
        let readiness = Readiness::default();
        let mut said = introduction();
        said.version = Some(PRIVATE_WORDS.parse().unwrap());
        let value = checked(
            SignedIn::Unknown {
                reason: UnknownReason::InUse,
            },
            Some(Handshake::Spoke(said)),
        );
        let at = value.at;
        readiness.keep(key(), value);
        let facts = readiness.diagnostic(&key(), at).unwrap();
        assert_eq!(
            facts.sign_in,
            DiagnosticSignIn::Unknown {
                reason: UnknownReason::InUse
            }
        );
        assert!(facts.version.is_none());
        let other = Key {
            profile: "another".parse().unwrap(),
            ..key()
        };
        assert!(readiness.diagnostic(&other, at).is_none());
    }
}
