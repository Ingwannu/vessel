//! Git owns projection, availability and effects. This module only translates
//! its captured command snapshot into source registrations for the one palette.
//! A stable fingerprint avoids replacing callbacks on every animation frame;
//! repository incarnation is part of the opaque owner's equality contract.
use super::source::Command;
use crate::areas::git::commands::GitCommandTarget;
use crate::routes::Route;
use crate::shell::Shell;
use vessel_app_kit::assets::Glyph;
use vessel_app_view::palette::registration::{
    Contribution, Owner, Presentation, Rank, RegistrationError, Visibility,
};
use vessel_app_view::workspace::git::{Action, StashScope};
use vessel_app_view::workspace::git_quick::GitAction;
use vessel_app_view::workspace::pull::MenuEntry;
use vessel_proto::forge::MergeMethod;

const SOURCE: &str = "workspace-git";
const SECONDARY_START: i16 = 100;

#[derive(PartialEq, Eq)]
struct Signature {
    target: GitCommandTarget,
    primary: Option<Action>,
    entries: Vec<Entry>,
}
#[derive(PartialEq, Eq)]
struct Entry {
    action: MenuEntry,
    title: String,
    selectable: bool,
}
#[derive(Default)]
pub(super) struct State {
    signature: Option<Signature>,
    owner: Option<Owner>,
}

/// No I/O or notification during projection. The current frame or event already
/// requires drawing; unchanged snapshots keep their existing registration token.
pub(super) fn refresh(shell: &mut Shell) {
    let next = if shell.areas.palette.is_open() && shell.route == Route::Session {
        signature(shell)
    } else {
        None
    };
    if shell.areas.palette.git.signature == next {
        return;
    }
    let replacement = match &next {
        Some(signature) => match contributions(signature) {
            Ok(entries) => match SOURCE
                .parse()
                .and_then(|source| shell.areas.palette.sources.replace(source, entries))
            {
                Ok(owner) => Some(owner),
                Err(error) => {
                    tracing::warn!(%error, "git command source refused");
                    return;
                }
            },
            Err(error) => {
                tracing::warn!(%error, "git command projection refused");
                return;
            }
        },
        None => None,
    };
    if let Some(old) = shell.areas.palette.git.owner.take() {
        shell.areas.palette.sources.remove(&old);
    }
    shell.areas.palette.git.owner = replacement;
    shell.areas.palette.git.signature = next;
}

pub(super) fn close(shell: &mut Shell) {
    if let Some(owner) = shell.areas.palette.git.owner.take() {
        shell.areas.palette.sources.remove(&owner);
    }
    shell.areas.palette.git.signature = None;
}

fn signature(shell: &Shell) -> Option<Signature> {
    let commands = shell.git_command_entries()?;
    let entries = commands
        .entries
        .into_iter()
        .filter_map(|action| {
            let item = shell.git_command_item(&action)?;
            Some(Entry {
                action,
                title: item.label.to_string(),
                selectable: item.selectable(),
            })
        })
        .collect();
    Some(Signature {
        target: commands.target,
        primary: commands.primary,
        entries,
    })
}

fn contributions(signature: &Signature) -> Result<Vec<Contribution<Command>>, RegistrationError> {
    signature.entries.iter().enumerate().map(|(at, entry)| {
        let promoted = matches!(&entry.action, MenuEntry::Git(row) if signature.primary == Some(row.action) && row.unavailable.is_none()) && entry.selectable;
        let at = i16::try_from(at).map_err(|_overflow| RegistrationError::InvalidId)?;
        let rank = if promoted { 1 } else { SECONDARY_START.checked_add(at).ok_or(RegistrationError::InvalidId)? };
        let target = signature.target.clone();
        let action = entry.action.clone();
        Ok(Contribution {
            id: identity(&entry.action).parse()?, group: "workspace".parse()?, group_rank: Rank::WORKSPACE,
            section_title: None, rank: Rank::new(rank), keywords: vec![identity(&entry.action).to_owned(), "git".to_owned()],
            visibility: if promoted { Visibility::Always } else { Visibility::Query },
            presentation: Presentation::Action { title: entry.title.clone(), subtitle: None },
            target: Command::callback(Some(glyph(&entry.action)), None, move |shell, window, cx| {
                shell.git_command_item(&action).is_some_and(|item| item.selectable())
                    && shell.run_git_command(&target, &action, window, cx)
            }),
        })
    }).collect()
}

const fn identity(entry: &MenuEntry) -> &'static str {
    match entry {
        MenuEntry::Git(row) => match row.action {
            Action::Quick(GitAction::Commit) => "git:commit",
            Action::Quick(GitAction::Push) => "git:push",
            Action::Quick(GitAction::Pull) => "git:pull",
            Action::Quick(GitAction::Fetch) => "git:fetch",
            Action::SwitchBranch => "git:switch-branch",
            Action::CreateBranch => "git:create-branch",
            Action::RenameBranch => "git:rename-branch",
            Action::SaveStash(StashScope::Tracked) => "git:stash-tracked",
            Action::SaveStash(StashScope::WithUntracked) => "git:stash-all",
            Action::Stashes => "git:stashes",
        },
        MenuEntry::Pull(_) => "git:pull-request",
        MenuEntry::Merge(MergeMethod::Merge) => "git:merge",
        MenuEntry::Merge(MergeMethod::Rebase) => "git:rebase",
        MenuEntry::Merge(MergeMethod::Squash) => "git:squash",
    }
}
const fn glyph(entry: &MenuEntry) -> Glyph {
    match entry {
        MenuEntry::Git(row) => crate::areas::git::glyph(row.action),
        MenuEntry::Pull(_) => Glyph::PullRequest,
        MenuEntry::Merge(_) => Glyph::GitMerge,
    }
}
