//! Credential-free links address authenticated node identities, never book aliases.

use vessel_proto::{NodeId, SessionId};

/// A destination shared by clipboard generation and OS link delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationTarget {
    /// Open a conversation on the node that minted it.
    Session {
        /// Stable authenticated host identity.
        node: NodeId,
        /// Conversation identity.
        session: SessionId,
    },
}

/// Refusals never retain external URL text, which may contain a credential.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum NavigationError {
    /// Scheme, authority or path does not name a supported route.
    #[error("unsupported desktop link")]
    Route,
    /// Credentials, query, fragment and encoded segments are not navigation.
    #[error("desktop links cannot contain credentials or extra parameters")]
    Extra,
    /// One of the protocol identifiers is malformed.
    #[error("invalid desktop destination identifier")]
    Identifier,
}

impl NavigationTarget {
    /// Decode the one supported authority and exact path shape at the OS edge.
    ///
    /// # Errors
    /// Rejects every component that is not a validated destination identifier.
    pub fn parse(text: &str) -> Result<Self, NavigationError> {
        if text.contains(['?', '#', '@', '%', '\\']) || text.chars().any(char::is_whitespace) {
            return Err(NavigationError::Extra);
        }
        let path = text
            .strip_prefix("vessel://h/")
            .ok_or(NavigationError::Route)?;
        let parts: Vec<_> = path.split('/').collect();
        match parts.as_slice() {
            [node, "session", session] => Ok(Self::Session {
                node: node
                    .parse()
                    .map_err(|_invalid| NavigationError::Identifier)?,
                session: session
                    .parse()
                    .map_err(|_invalid| NavigationError::Identifier)?,
            }),
            _ => Err(NavigationError::Route),
        }
    }

    /// The exact canonical format accepted by OS delivery.
    #[must_use]
    pub fn to_url(self) -> String {
        match self {
            Self::Session { node, session } => format!("vessel://h/{node}/session/{session}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_generated_target_round_trips() {
        let node = NodeId::random();
        let target = NavigationTarget::Session {
            node,
            session: SessionId::random(),
        };
        assert_eq!(NavigationTarget::parse(&target.to_url()), Ok(target));
    }

    #[test]
    fn credentials_and_extra_routes_never_become_destinations() {
        let good = NavigationTarget::Session {
            node: NodeId::random(),
            session: SessionId::random(),
        }
        .to_url();
        for bad in [
            format!("{good}?token=secret"),
            format!("{good}#fragment"),
            format!("{good}/extra"),
            good.replace("h/", "user@h/"),
            good.replace("h/", "h:123/"),
            good.replace("session", "%73ession"),
            format!(" {good}"),
            good.replace("vessel:", "https:"),
        ] {
            assert!(NavigationTarget::parse(&bad).is_err());
        }
    }
}
