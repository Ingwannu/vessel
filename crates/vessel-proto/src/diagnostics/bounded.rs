//! Bounded lists reject the first excess element before decoding its body.
use super::{DiagnosticAgent, DiagnosticFailure, InvalidDiagnostic};
use serde::de::{DeserializeSeed, Error as _, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::{collections::HashSet, fmt};

/// Largest complete agent inventory in a diagnostic report.
pub const MAX_DIAGNOSTIC_AGENTS: usize = 64;
/// Most retained RPC failures in a diagnostic report.
pub const MAX_DIAGNOSTIC_FAILURES: usize = 32;

struct Reject(InvalidDiagnostic);
impl<'de> DeserializeSeed<'de> for Reject {
    type Value = ();
    fn deserialize<D: Deserializer<'de>>(self, _: D) -> Result<(), D::Error> {
        Err(D::Error::custom(self.0))
    }
}

// Both lists are report values, not shared storage. Their decoder stops at
// the first excess element; its body must not be allocated just to refuse it.
macro_rules! bounded_list {
    ($name:ident, $item:ty, $max:ident, $error:ident, $doc:literal $(, $validate:ident)?) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS))]
        #[serde(transparent)]
        pub struct $name(Vec<$item>);
        impl $name {
            /// Read the bounded records without permitting mutation.
            #[must_use]
            pub fn as_slice(&self) -> &[$item] {
                &self.0
            }
            /// Number of records.
            #[must_use]
            pub fn len(&self) -> usize {
                self.0.len()
            }
            /// Whether there are no records (not a source-availability claim).
            #[must_use]
            pub fn is_empty(&self) -> bool {
                self.0.is_empty()
            }
        }
        impl TryFrom<Vec<$item>> for $name {
            type Error = InvalidDiagnostic;
            fn try_from(values: Vec<$item>) -> Result<Self, Self::Error> {
                if values.len() > $max {
                    return Err(InvalidDiagnostic::$error);
                }
                $( $validate(&values)?; )?
                Ok(Self(values))
            }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
                struct Rows;
                impl<'de> Visitor<'de> for Rows {
                    type Value = $name;
                    fn expecting(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
                        fmt::Display::fmt(&InvalidDiagnostic::$error, output)
                    }
                    fn visit_seq<A: SeqAccess<'de>>(
                        self,
                        mut source: A,
                    ) -> Result<Self::Value, A::Error> {
                        if source.size_hint().is_some_and(|count| count > $max) {
                            return Err(A::Error::custom(InvalidDiagnostic::$error));
                        }
                        let mut values = Vec::new();
                        loop {
                            if values.len() == $max {
                                match source.next_element_seed(Reject(InvalidDiagnostic::$error))? {
                                    None => {
                                        return $name::try_from(values).map_err(A::Error::custom);
                                    }
                                    Some(()) => {
                                        return Err(A::Error::custom(InvalidDiagnostic::$error));
                                    }
                                }
                            }
                            match source.next_element::<$item>()? {
                                None => return $name::try_from(values).map_err(A::Error::custom),
                                Some(value) => values.push(value),
                            }
                        }
                    }
                }
                // Decoder errors may quote unknown input strings. The public
                // diagnostic boundary retains only this rule, never that text.
                input
                    .deserialize_seq(Rows)
                    .map_err(|_untrusted| D::Error::custom(InvalidDiagnostic::$error))
            }
        }
    };
}
fn distinct_agents(rows: &[DiagnosticAgent]) -> Result<(), InvalidDiagnostic> {
    let mut found = HashSet::new();
    if rows.iter().any(|row| !found.insert(row.id)) {
        return Err(InvalidDiagnostic::DuplicateAgents);
    }
    Ok(())
}

bounded_list!(
    DiagnosticAgents,
    DiagnosticAgent,
    MAX_DIAGNOSTIC_AGENTS,
    Agents,
    "A complete, bounded inventory with distinct safe agent identities.",
    distinct_agents
);
bounded_list!(
    RecentFailures,
    DiagnosticFailure,
    MAX_DIAGNOSTIC_FAILURES,
    Failures,
    "Bounded recent failed local RPC completions, not raw log lines."
);
