//! A workspace's shell terminals as tabs (TRACKER 481, 482): which of a
//! machine's terminals a workspace shows, the label a tab shows, and what
//! a tab says when its terminal draws no live screen. Paseo:
//! `screens/workspace/terminals/state.ts` (`collectStandaloneTerminalIds`),
//! `panels/terminal-panel.tsx` (the label) and
//! `terminal/runtime/terminal-stream-controller.ts` (the stream's errors).
//!
//! [Decision Log]
//! - Purpose: the native app opens, lists and closes shells in a workspace
//!   as Paseo does; the layout already models `Target::Terminal` and
//!   reconciles from an id list (`layout::EntitySnapshot`).
//! - Existing implementation and constraints: ADR 0031's `terminal.list`
//!   answers every terminal of a place, live and ended, shells and the
//!   runs of a project's scripts and services. Paseo's manager forgets a
//!   terminal when it exits; Vessel's node keeps up to 32 ended ones.
//! - Choice: a tab for each live shell of the workspace's place, in
//!   Paseo's order (sorted ids). Scripts and services stay with the
//!   Scripts feature, as Paseo's standalone set leaves a script's terminal
//!   out. An ended shell has no tab, as an exited terminal leaves Paseo's
//!   list.
//! - Cost: a shell that ended keeps no tab to read its last screen, as in
//!   Paseo.

use vessel_proto::terminals::{TerminalInfo, TerminalKind, TerminalList, TerminalPlace};
use vessel_proto::{ErrorCode, TerminalId};

pub mod places;

pub use places::{
    beside_sessions, folder_sessions, holds, in_place, place_for, workspace_places,
    workspace_sessions,
};

/// The terminals `place` shows as tabs: its live shells, by id.
#[must_use]
pub fn standalone(list: &TerminalList, place: &TerminalPlace) -> Vec<TerminalId> {
    let mut ids: Vec<TerminalId> = list
        .terminals
        .iter()
        .filter(|info| in_place(info, place) && shows_as_tab(info))
        .map(|info| info.id)
        .collect();
    ids.sort_unstable();
    ids
}

/// Whether a terminal is one a workspace shows as a tab: a live shell.
#[must_use]
pub fn shows_as_tab(info: &TerminalInfo) -> bool {
    matches!(info.kind, TerminalKind::Shell { .. }) && info.is_live()
}

/// The title a tab shows; `None` shows the fallback word ("Terminal").
#[must_use]
pub fn title(info: &TerminalInfo) -> Option<&str> {
    info.title
        .as_ref()
        .map(vessel_proto::SessionTitle::as_str)
        .filter(|title| !title.trim().is_empty())
}

/// Why a terminal tab draws no live screen: the red line under it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trouble {
    /// Its program ended ("Terminal exited").
    Exited,
    /// The machine no longer has it ("Terminal not found").
    NotFound,
    /// Following it failed for another reason ("Unable to subscribe to
    /// terminal"); the machine's own words are not shown.
    Unsubscribable,
}

impl Trouble {
    /// A refusal of `terminal.subscribe`, by its code.
    #[must_use]
    pub const fn refused(code: ErrorCode) -> Self {
        match code {
            ErrorCode::NotFound => Self::NotFound,
            ErrorCode::InvalidRequest
            | ErrorCode::Unauthorized
            | ErrorCode::Forbidden
            | ErrorCode::ProtocolMismatch
            | ErrorCode::Conflict
            | ErrorCode::Unsupported
            | ErrorCode::HarnessNotInstalled
            | ErrorCode::SpawnFailed
            | ErrorCode::Unavailable
            | ErrorCode::Internal => Self::Unsubscribable,
        }
    }
}

/// Why a call about a terminal failed, as a code only: a machine's own
/// words do not reach the screen (the Devices page's rule).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// The machine is not connected, or the connection dropped.
    Offline,
    /// It did not answer in time.
    TimedOut,
    /// It answered with a refusal.
    Refused(ErrorCode),
}

impl Failure {
    /// What a tab's red line says when following failed this way.
    #[must_use]
    pub const fn trouble(self) -> Trouble {
        match self {
            Self::Refused(code) => Trouble::refused(code),
            Self::Offline | Self::TimedOut => Trouble::Unsubscribable,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use vessel_proto::SessionStatus;
    use vessel_proto::scripts::ScriptName;
    use vessel_proto::workspace::ProjectCommand;
    use vessel_proto::{SessionId, SessionTitle};

    fn shell(place: &TerminalPlace) -> TerminalInfo {
        TerminalInfo::new(
            TerminalId::random(),
            place.clone(),
            "/work".into(),
            TerminalKind::Shell {
                program: "/bin/sh".into(),
            },
            Utc::now(),
        )
    }

    #[test]
    fn a_place_shows_its_live_shells_in_id_order() {
        let here = TerminalPlace::Session {
            session: SessionId::random(),
        };
        let elsewhere = TerminalPlace::Session {
            session: SessionId::random(),
        };
        let first = shell(&here);
        let second = shell(&here);
        let mut ended = shell(&here);
        ended.status = SessionStatus::Exited { code: Some(0) };
        let mut script = shell(&here);
        script.kind = TerminalKind::Script {
            script: "test".parse::<ScriptName>().unwrap(),
            command: "true".parse::<ProjectCommand>().unwrap(),
        };
        let other = shell(&elsewhere);
        let list = TerminalList {
            terminals: vec![second.clone(), ended, script, other, first.clone()],
            revision: 0,
        };
        let mut expected = vec![first.id, second.id];
        expected.sort_unstable();
        assert_eq!(standalone(&list, &here), expected);
    }

    #[test]
    fn a_blank_title_falls_back() {
        let place = TerminalPlace::Session {
            session: SessionId::random(),
        };
        let info = shell(&place);
        assert_eq!(title(&info), None);
        let named = info.with_title(Some("build".parse::<SessionTitle>().unwrap()));
        assert_eq!(title(&named), Some("build"));
    }

    #[test]
    fn only_a_missing_terminal_is_not_found() {
        assert_eq!(Trouble::refused(ErrorCode::NotFound), Trouble::NotFound);
        assert_eq!(
            Trouble::refused(ErrorCode::Forbidden),
            Trouble::Unsubscribable
        );
    }
}
