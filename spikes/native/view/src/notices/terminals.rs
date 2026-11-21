//! A terminal's agent asking for the person or done: a notice, claimed before
//! it shows (ADR 0089's terminal attention and `notification.claim`; ADR 0114
//! "Terminal notices"). This is the claims follow-up's third row.
//!
//! [Decision Log]
//! - Purpose: an agent in a terminal reports through Vessel's hooks (ADR
//!   0089) that it finished or needs input. Paseo tells one client, which
//!   shows an OS notification titled "Terminal finished" or "Terminal needs
//!   input", with the terminal's name as its body. A tap opens the terminal
//!   (`websocket-server.ts:213-215,2955-3044`; the app's
//!   `session-context.tsx:658-677`).
//! - Constraint: behind ADR 0089's constant. Until it is assigned, the node
//!   fills no terminal's attention, and no machine admits claims
//!   ([`super::claims::claims`]), so nothing here runs.
//! - Source: the machine's whole terminal list (`terminal.list` with no
//!   place), waited on with ADR 0116's `changed_from`. Once the feature is
//!   assigned, the node moves that list's revision when a terminal's
//!   attention changes (`terminals/registry/listed.rs`). No other read
//!   carries every terminal's attention.
//! - The first list of a connection tells nothing ([`Watched`]): attention
//!   already there was an earlier event, and Paseo notifies on the event.
//!   This is the session notices' rule too (`notices`, "The first sighting
//!   is never a notice").
//! - A notice is a terminal whose attention is Finished or Needs input at a
//!   revision the connection has not told of. A failed terminal never
//!   counts; the node's push worker drops it too (`push/terminal.rs`).
//! - Showing: only an `Admitted` claim shows. A terminal has no inbox, so
//!   there is no first-page fallback as there is for sessions.
//! - Text: the terminal's title, never its command (ADR 0089: a
//!   notification names no command).
//! - Alternative: following each terminal (`terminal.get` with
//!   `changed_from`) costs one long poll per terminal (up to 96 a machine)
//!   where the list costs one.

use std::collections::HashMap;

use vessel_proto::SessionStatus;
use vessel_proto::TerminalId;
use vessel_proto::notifications::{
    NotificationSource, TerminalActivity, TerminalAttentionReason, TerminalAttentionRevision,
};
use vessel_proto::terminals::{TerminalInfo, TerminalPlace};

use super::NoticeKind;
use crate::prefs::Alerts;

/// A terminal that started to need the person, or finished.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalNotice {
    /// Needs you, or finished.
    pub kind: NoticeKind,
    /// The machine, by its book name.
    pub machine: String,
    /// Which terminal.
    pub terminal: TerminalId,
    /// The attention's exact revision: what a claim and a clear name.
    pub revision: TerminalAttentionRevision,
    /// Where it lives, to open it on a click.
    pub place: TerminalPlace,
    /// What a person called it; empty when untitled.
    pub title: String,
}

impl TerminalNotice {
    /// What a claim names: this exact revision, never just the terminal.
    #[must_use]
    pub const fn source(&self) -> NotificationSource {
        NotificationSource::Terminal {
            terminal: self.terminal,
            revision: self.revision,
        }
    }
}

/// What `reason` tells the person, if anything.
const fn kind_of(reason: TerminalAttentionReason) -> Option<NoticeKind> {
    match reason {
        TerminalAttentionReason::Finished => Some(NoticeKind::Finished),
        TerminalAttentionReason::NeedsInput => Some(NoticeKind::NeedsYou),
        TerminalAttentionReason::Unknown => None,
    }
}

/// The revision and kind `info` asks attention at, if it does.
fn asking(info: &TerminalInfo) -> Option<(TerminalAttentionRevision, NoticeKind)> {
    if matches!(info.status, SessionStatus::Failed { .. }) {
        return None;
    }
    let attention = info.attention?;
    match attention.activity {
        TerminalActivity::Attention { reason } => {
            kind_of(reason).map(|kind| (attention.revision, kind))
        }
        _ => None,
    }
}

/// One connection's terminal lists, as read: what it has told of.
#[derive(Debug, Clone, Default)]
pub struct Watched {
    /// The attention revision each terminal was last seen at; `None` before
    /// the connection's first list.
    seen: Option<HashMap<TerminalId, TerminalAttentionRevision>>,
}

impl Watched {
    /// A new list of `machine`'s terminals: the notices it brings. The
    /// first list brings none.
    pub fn read(&mut self, machine: &str, list: &[TerminalInfo]) -> Vec<TerminalNotice> {
        let now: HashMap<TerminalId, TerminalAttentionRevision> = list
            .iter()
            .filter_map(|info| {
                info.attention
                    .map(|attention| (info.id, attention.revision))
            })
            .collect();
        let Some(before) = self.seen.replace(now) else {
            return Vec::new();
        };
        list.iter()
            .filter_map(|info| {
                let (revision, kind) = asking(info)?;
                let told = before.get(&info.id) == Some(&revision);
                (!told).then(|| TerminalNotice {
                    kind,
                    machine: machine.to_owned(),
                    terminal: info.id,
                    revision,
                    place: info.place.clone(),
                    title: info
                        .title
                        .as_ref()
                        .map(|title| title.as_str().to_owned())
                        .unwrap_or_default(),
                })
            })
            .collect()
    }
}

/// How a terminal notice is told, under Settings' `alerts`: `None` when
/// neither a notification nor a sound is wanted, and then nothing is
/// claimed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalAnnounce {
    /// What.
    pub notice: TerminalNotice,
    /// As the system's notification.
    pub system: bool,
    /// With the sound.
    pub sound: bool,
}

/// The terminal notices to claim, in order, under `alerts`. A window that
/// tells nothing never claims, so it never takes the notice from one that
/// would.
#[must_use]
pub fn announce_terminals(found: Vec<TerminalNotice>, alerts: Alerts) -> Vec<TerminalAnnounce> {
    let (system, sound) = (alerts.system(), alerts.sound());
    if !system && !sound {
        return Vec::new();
    }
    found
        .into_iter()
        .map(|notice| TerminalAnnounce {
            notice,
            system,
            sound,
        })
        .collect()
}

#[cfg(test)]
mod tests;
