//! Title changes sent to the owning actor. A generated proposal carries
//! the expected title so validation and mutation share the actor's order.

use super::{MetadataChange, SessionHandle};
use crate::SessionError;
use tokio::sync::oneshot;
use vessel_proto::{GeneratedTitle, SessionInfo, SessionTitle, TitleOrigin};

/// Whether a generated proposal still owned the provisional title.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub enum TitleUpdate {
    /// The actor committed the proposed title and its generated origin.
    Applied,
    /// The proposal was not published; the current title was kept.
    Kept,
}

#[derive(Debug)]
pub(crate) struct Rename {
    pub(crate) change: RenameChange,
    pub(crate) done: oneshot::Sender<TitleUpdate>,
}

#[derive(Debug)]
pub(crate) enum RenameChange {
    Typed(SessionTitle),
    Generated {
        expected: SessionTitle,
        title: GeneratedTitle,
    },
}

impl RenameChange {
    pub(crate) fn resolve(self, current: &SessionInfo) -> Option<(SessionTitle, TitleOrigin)> {
        match self {
            Self::Typed(title) => Some((title, TitleOrigin::Typed)),
            Self::Generated { expected, title } if eligible(current, &expected) => {
                Some((title.into_title(), TitleOrigin::Generated))
            }
            Self::Generated { .. } => None,
        }
    }
}

// A given label on a side/fork specification is not a fresh unnamed
// session. Keeping this guard beside the actor rule prevents a caller from
// bypassing that distinction just by knowing the old words.
fn eligible(current: &SessionInfo, expected: &SessionTitle) -> bool {
    let fresh_mode = match &current.spec.mode {
        vessel_proto::SessionMode::Structured { resume: None, .. } => true,
        vessel_proto::SessionMode::Structured {
            resume: Some(_), ..
        }
        | vessel_proto::SessionMode::Terminal { .. } => false,
    };
    let fresh_origin = match current.origin.as_ref() {
        None | Some(vessel_proto::SessionOrigin::Remote { .. }) => true,
        Some(vessel_proto::SessionOrigin::Imported { .. }) => false,
    };
    current.status.is_live()
        && current.parked.is_none()
        && fresh_origin
        && fresh_mode
        && current.spec.title.is_none()
        && current.title_origin == TitleOrigin::Given
        && current.title.as_ref() == Some(expected)
}

impl SessionHandle {
    /// Propose a generated replacement. The actor compares both the
    /// expected title and its given origin before publishing, so even a
    /// person's rename to identical words takes precedence. Explicitly
    /// titled, terminal, resuming, parked and imported sessions are kept too.
    ///
    /// # Errors
    /// [`SessionError::Ended`] if the actor has ended.
    pub async fn apply_generated_title(
        &self,
        expected: SessionTitle,
        title: GeneratedTitle,
    ) -> Result<TitleUpdate, SessionError> {
        self.rename_change(RenameChange::Generated { expected, title })
            .await
    }

    pub(super) async fn rename_change(
        &self,
        change: RenameChange,
    ) -> Result<TitleUpdate, SessionError> {
        if !self.0.info.borrow().status.is_live() {
            return Err(SessionError::Ended);
        }
        let (done, answered) = oneshot::channel();
        self.0
            .metadata
            .send(MetadataChange::Rename(Rename { change, done }))
            .await
            .map_err(|_ended| SessionError::Ended)?;
        answered.await.map_err(|_ended| SessionError::Ended)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn current(origin: TitleOrigin) -> SessionInfo {
        let spec = vessel_proto::SessionSpec::new(
            vessel_proto::HarnessKind::Codex,
            vessel_proto::SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        );
        let mut info =
            SessionInfo::new(vessel_proto::SessionId::random(), spec, chrono::Utc::now());
        info.title = Some("Seed".parse().unwrap());
        info.title_origin = origin;
        info
    }

    fn proposal(expected: &str) -> RenameChange {
        RenameChange::Generated {
            expected: expected.parse().unwrap(),
            title: "Generated".parse().unwrap(),
        }
    }

    #[test]
    fn only_the_exact_given_title_can_be_replaced() {
        let given = current(TitleOrigin::Given);
        let (title, origin) = proposal("Seed").resolve(&given).unwrap();
        assert_eq!(title.as_str(), "Generated");
        assert_eq!(origin, TitleOrigin::Generated);
        assert!(proposal("Different").resolve(&given).is_none());
        for origin in [
            TitleOrigin::Typed,
            TitleOrigin::Generated,
            TitleOrigin::Unknown,
        ] {
            assert!(proposal("Seed").resolve(&current(origin)).is_none());
        }
    }
    fn rejected(info: &SessionInfo) {
        assert!(proposal("Seed").resolve(info).is_none());
    }

    #[test]
    fn explicit_labels_and_nonfresh_modes_are_kept() {
        let mut info = current(TitleOrigin::Given);
        info.spec.title = info.title.clone();
        rejected(&info);
        info.spec.title = None;
        info.spec.mode = vessel_proto::SessionMode::Structured {
            model: None,
            resume: Some(vessel_proto::HarnessSessionId::new("existing")),
            options: Vec::new(),
        };
        rejected(&info);
        info.spec.mode = vessel_proto::SessionMode::Terminal {
            args: Vec::new(),
            size: vessel_proto::TerminalSize::default(),
        };
        rejected(&info);
    }

    #[test]
    fn ended_and_missing_titles_are_kept() {
        let mut info = current(TitleOrigin::Given);
        info.status = vessel_proto::SessionStatus::Interrupted;
        rejected(&info);
        let mut info = current(TitleOrigin::Given);
        info.title = None;
        rejected(&info);
    }

    #[test]
    fn parked_and_imported_are_kept_but_fresh_remote_is_eligible() {
        let mut info = current(TitleOrigin::Given);
        assert!(info.status.is_live());
        info.parked = Some(vessel_proto::parking::Parked {
            since: chrono::Utc::now(),
            reason: vessel_proto::parking::ParkReason::Asked,
        });
        rejected(&info);
        info.parked = None;
        info.origin = Some(vessel_proto::SessionOrigin::Imported {
            node: vessel_proto::NodeId::random(),
            session: vessel_proto::SessionId::random(),
        });
        rejected(&info);
        info.origin = Some(vessel_proto::SessionOrigin::Remote {
            node: vessel_proto::NodeId::random(),
            by: vessel_proto::RemoteCreator::Owner,
        });
        assert!(proposal("Seed").resolve(&info).is_some());
    }
}
