//! Naming a session on the command line: its full id, or any prefix of it
//! that matches exactly one session (ids are long; the first few characters
//! are usually enough, as with git commits). A queue's entry, a question
//! or a permission request is named the same way among the ones there are
//! ([`IdPrefix`]).

use std::fmt::Display;
use std::str::FromStr;

use vessel_client::NodeClient;
use vessel_proto::{SessionId, SessionInfo};

use crate::error::CliError;

/// A full session id or a prefix of one, as typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionSelector(String);

/// Text that cannot be (part of) an id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("an id (or its prefix) is made of hex digits and dashes")]
pub(crate) struct InvalidSelector;

/// `text` as an id or the prefix of one: trimmed, lower case, hex digits
/// and dashes only.
fn id_text(text: &str) -> Result<String, InvalidSelector> {
    let text = text.trim().to_ascii_lowercase();
    let valid = !text.is_empty()
        && text
            .chars()
            .all(|character| character.is_ascii_hexdigit() || character == '-');
    if valid {
        Ok(text)
    } else {
        Err(InvalidSelector)
    }
}

impl FromStr for SessionSelector {
    type Err = InvalidSelector;

    fn from_str(text: &str) -> Result<Self, InvalidSelector> {
        id_text(text).map(Self)
    }
}

/// An id, or a prefix of one, naming one of a few things a command has
/// just read (a queue's entries, the requests waiting).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IdPrefix(String);

impl FromStr for IdPrefix {
    type Err = InvalidSelector;

    fn from_str(text: &str) -> Result<Self, InvalidSelector> {
        id_text(text).map(Self)
    }
}

impl IdPrefix {
    /// The one of `items` whose id (`id`) starts with this: `what` names
    /// them in the error when none or several do.
    pub(crate) fn pick<'a, T, I: Display>(
        &self,
        items: &'a [T],
        id: impl Fn(&T) -> I,
        what: &'static str,
    ) -> Result<&'a T, CliError> {
        let mut matching = items
            .iter()
            .filter(|item| id(item).to_string().starts_with(&self.0));
        match (matching.next(), matching.next()) {
            (None, _) => Err(CliError::NoMatch {
                what,
                prefix: self.0.clone(),
            }),
            (Some(item), None) => Ok(item),
            (Some(_), Some(_)) => Err(CliError::AmbiguousMatch {
                what,
                prefix: self.0.clone(),
                matches: matching.count().saturating_add(2),
            }),
        }
    }
}

impl SessionSelector {
    /// The session this names on `client`'s node.
    pub(crate) async fn resolve(&self, client: &NodeClient) -> Result<SessionInfo, CliError> {
        if let Ok(id) = self.0.parse::<SessionId>() {
            return Ok(client.session(id).await?);
        }
        let mut matching = client
            .sessions()
            .await?
            .into_iter()
            .filter(|session| session.id.to_string().starts_with(&self.0));
        match (matching.next(), matching.next()) {
            (None, _) => Err(CliError::NoSuchSession(self.0.clone())),
            (Some(session), None) => Ok(session),
            (Some(_), Some(_)) => Err(CliError::AmbiguousSession {
                prefix: self.0.clone(),
                matches: matching.count().saturating_add(2),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_ids_and_prefixes_only() {
        assert_eq!(
            " 3F2a ".parse::<SessionSelector>(),
            Ok(SessionSelector("3f2a".to_owned()))
        );
        assert!(
            "00000000-0000-4000-8000-000000000001"
                .parse::<SessionSelector>()
                .is_ok()
        );
        assert!("".parse::<SessionSelector>().is_err());
        assert!("nope".parse::<SessionSelector>().is_err());
    }

    #[test]
    fn a_prefix_picks_one_or_says_why_not() {
        let ids = ["3f2a-01", "3f2b-02", "9c00-03"];
        let prefix = |text: &str| text.parse::<IdPrefix>().unwrap();
        let pick = |text: &str| prefix(text).pick(&ids, |id| *id, "entry").copied();
        assert_eq!(pick("9C").unwrap(), "9c00-03");
        assert_eq!(pick("3f2a-01").unwrap(), "3f2a-01");
        assert!(matches!(
            pick("3f2"),
            Err(CliError::AmbiguousMatch { matches: 2, .. })
        ));
        assert!(matches!(pick("7"), Err(CliError::NoMatch { .. })));
        assert!("x".parse::<IdPrefix>().is_err());
    }
}
