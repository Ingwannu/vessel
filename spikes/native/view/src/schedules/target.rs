//! The reference target line resolved from already-read node facts. A cache miss
//! never proves deletion. Remote paths are displayed, never joined with the
//! viewer platform's path rules (a Windows node may be viewed from Linux).
use crate::Card;
use std::path::Path;
use vessel_proto::workspace::{ProjectInfo, ProjectName, RelPath, SessionPlace, WorktreeInfo};
use vessel_proto::{HarnessRef, automation::ScheduleTarget};

/// Identity versus code/path determines its typeface; no translated fragments.
#[derive(Debug, PartialEq, Eq)]
pub enum Line<'a> {
    /// Existing chat's actual title, excluding the card's folder fallback.
    Chat(&'a str),
    /// A known chat with no explicit title.
    UntitledChat,
    /// Project identity, with an optional node-authored relative folder.
    Project {
        /// Node-authored project identity, not a fabricated folder label.
        name: &'a ProjectName,
        /// Validated relative folder, retained without local path joining.
        dir: Option<&'a RelPath>,
    },
    /// Node-authored path, without interpreting it on this platform.
    Path(&'a Path),
    /// The agent chooses its default folder; no fake path is supplied.
    DefaultFolder,
    /// Cached target facts are unavailable, without a deletion claim.
    Unavailable(Missing),
    /// A target variant newer than the app understands.
    Unknown,
}
/// Which unavailable target needs a complete localized explanation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Missing {
    /// Existing chat directory has no matching cached entry.
    Chat,
    /// Explicit project id has no known project name.
    Project,
    /// Explicit worktree id has no known node path.
    Worktree,
}
/// The same target facts supply the leading official mark when known.
#[derive(Debug, PartialEq, Eq)]
pub struct Target<'a> {
    /// One reference-style target line, with no invented new-chat prefix.
    pub line: Line<'a>,
    /// Missing session facts omit the provider mark rather than guessing one.
    pub agent: Option<&'a HarnessRef>,
}

/// Resolve identity only. Schedule state remains the node's authoritative value.
#[must_use]
pub fn resolve<'a>(
    target: &'a ScheduleTarget,
    chat: Option<&'a Card>,
    projects: Option<&'a [ProjectInfo]>,
    worktrees: Option<&'a [WorktreeInfo]>,
) -> Target<'a> {
    match target {
        ScheduleTarget::Session { session } => {
            let card = chat.filter(|card| card.session == *session);
            let line = card.map_or(Line::Unavailable(Missing::Chat), |card| {
                let title = card.title.trim();
                if card.detail.titled && !title.is_empty() {
                    Line::Chat(title)
                } else {
                    Line::UntitledChat
                }
            });
            Target {
                line,
                agent: card.and_then(|card| card.detail.harness.as_ref()),
            }
        }
        ScheduleTarget::NewSession { spec, .. } => {
            let line = match spec.place.as_deref() {
                Some(SessionPlace::Project { project, dir }) => projects
                    .and_then(|projects| projects.iter().find(|known| known.id == *project))
                    .map_or(Line::Unavailable(Missing::Project), |project| {
                        Line::Project {
                            name: &project.name,
                            dir: dir.as_ref(),
                        }
                    }),
                Some(SessionPlace::Worktree { worktree }) => worktrees
                    .and_then(|trees| trees.iter().find(|tree| tree.id == *worktree))
                    .map_or(Line::Unavailable(Missing::Worktree), |tree| {
                        Line::Path(&tree.path)
                    }),
                None => folder(spec.cwd.as_deref(), projects),
            };
            Target {
                line,
                agent: Some(&spec.harness),
            }
        }
        ScheduleTarget::Unknown => Target {
            line: Line::Unknown,
            agent: None,
        },
    }
}
fn folder<'a>(cwd: Option<&'a Path>, projects: Option<&'a [ProjectInfo]>) -> Line<'a> {
    let Some(cwd) = cwd else {
        return Line::DefaultFolder;
    };
    projects
        .and_then(|list| list.iter().find(|project| project.root == cwd))
        .map_or(Line::Path(cwd), |project| Line::Project {
            name: &project.name,
            dir: None,
        })
}

/// The reference abbreviates remote Unix home prefixes without consulting the
/// viewer's home. Windows/UNC paths remain unchanged; no platform path joining.
#[must_use]
pub fn display_path(path: &Path) -> std::borrow::Cow<'_, str> {
    let text = path.to_string_lossy();
    let remainder = text
        .strip_prefix("/Users/")
        .or_else(|| text.strip_prefix("/home/"));
    let Some(remainder) = remainder else {
        return text;
    };
    match remainder.split_once('/') {
        Some(("", _)) => text,
        Some((_user, tail)) => std::borrow::Cow::Owned(format!("~/{tail}")),
        None if remainder.is_empty() => text,
        None => std::borrow::Cow::Owned("~".to_owned()),
    }
}

#[cfg(test)]
mod tests;
