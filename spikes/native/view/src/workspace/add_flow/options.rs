//! Add project's rows that are not the machine's folders: machines,
//! methods, the repositories typed by hand, parents and clone
//! destinations. Paseo's `add-project-flow/options.ts` (pinned 0.15.4; the
//! page's `add-project/options.ts`), host as machine
//! (docs/plans/wp-add-project-flow.md, differences 1 and 9-11).

use super::git_remote;
use super::model::{CloneProtocol, Machine, Repository};
use super::paths::shorten_path;

/// The four ways, in Paseo's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
    /// Search for directory.
    DirectorySearch,
    /// Browse (the app's own machine only).
    Browse,
    /// Clone from GitHub.
    Github,
    /// New directory.
    NewDirectory,
}

/// What a method's row says under its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MethodNote {
    /// "Find a directory on {machine}".
    FindOn(String),
    /// "Choose or create a directory in {the platform's file manager}".
    BrowseIn,
    /// "Update this machine to clone GitHub repositories".
    CloneNeedsUpdate,
    /// "Search projects available to your GitHub account".
    CloneSearch,
    /// "Enter a GitHub URL or owner/repo".
    CloneByHand,
    /// "Create an empty directory on {machine}".
    CreateOn(String),
    /// "Update this machine to create directories".
    CreateNeedsUpdate,
}

/// A method's row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodOption {
    /// Which method.
    pub method: Method,
    /// Its line under the name.
    pub note: MethodNote,
    /// The machine cannot do it.
    pub disabled: bool,
}

/// The machines whose label or name holds the field's text.
#[must_use]
pub fn filter_machines<'a>(machines: &'a [Machine], query: &str) -> Vec<&'a Machine> {
    let wanted = query.trim().to_lowercase();
    machines
        .iter()
        .filter(|machine| {
            wanted.is_empty()
                || machine.label.to_lowercase().contains(&wanted)
                || machine.name.to_lowercase().contains(&wanted)
        })
        .collect()
}

fn github_note(machine: &Machine) -> MethodNote {
    if !machine.can_clone {
        MethodNote::CloneNeedsUpdate
    } else if machine.can_search_github {
        MethodNote::CloneSearch
    } else {
        MethodNote::CloneByHand
    }
}

/// The methods `machine` offers; none for a machine that cannot add
/// projects.
#[must_use]
pub fn methods(machine: &Machine) -> Vec<MethodOption> {
    if !machine.can_add_project {
        return Vec::new();
    }
    let mut options = vec![MethodOption {
        method: Method::DirectorySearch,
        note: MethodNote::FindOn(machine.label.clone()),
        disabled: false,
    }];
    if machine.can_browse {
        options.push(MethodOption {
            method: Method::Browse,
            note: MethodNote::BrowseIn,
            disabled: false,
        });
    }
    options.push(MethodOption {
        method: Method::Github,
        note: github_note(machine),
        disabled: !machine.can_clone,
    });
    options.push(MethodOption {
        method: Method::NewDirectory,
        note: if machine.can_create_directory {
            MethodNote::CreateOn(machine.label.clone())
        } else {
            MethodNote::CreateNeedsUpdate
        },
        disabled: !machine.can_create_directory,
    });
    options
}

/// The last segment of a path.
#[must_use]
pub fn base_name(path: &str) -> &str {
    let trimmed = path.trim_end_matches(['/', '\\']);
    trimmed.rsplit(['/', '\\']).next().unwrap_or(trimmed)
}

/// `owner/repo`: two parts, neither empty, without white space or a third
/// `/` (Paseo's `^([^\s/]+)\/([^\s/]+)$`).
fn shorthand(text: &str) -> Option<(&str, &str)> {
    let (owner, name) = text.split_once('/')?;
    let part = |part: &str| {
        !part.is_empty() && !part.contains(['/']) && !part.contains(char::is_whitespace)
    };
    (part(owner) && part(name)).then_some((owner, name))
}

/// Rows for what was typed: a complete remote as itself, `owner/repo` via
/// HTTPS and via SSH, else none.
#[must_use]
pub fn typed_repositories(query: &str) -> Vec<Repository> {
    let repo = query.trim();
    if repo.is_empty() {
        return Vec::new();
    }
    if let Some(location) = git_remote::location(repo) {
        let name = git_remote::github_repository(repo).unwrap_or_else(|| {
            let last = base_name(&location.path);
            last.strip_suffix(".git").unwrap_or(last).to_owned()
        });
        return vec![Repository {
            id: format!("manual:{repo}"),
            name_with_owner: name,
            clone_url: repo.to_owned(),
            protocol: None,
        }];
    }
    let Some((owner, name)) = shorthand(repo) else {
        return Vec::new();
    };
    let name_with_owner = format!("{owner}/{name}");
    [(CloneProtocol::Https, "https"), (CloneProtocol::Ssh, "ssh")]
        .into_iter()
        .map(|(protocol, word)| Repository {
            id: format!("manual:{word}:{name_with_owner}"),
            name_with_owner: name_with_owner.clone(),
            clone_url: name_with_owner.clone(),
            protocol: Some(protocol),
        })
        .collect()
}

/// The remote `project.clone` fetches for a choice. Paseo's server turns
/// `owner/repo` and a protocol into a GitHub remote; Vessel's node takes a
/// URL, so the client does (difference 11).
#[must_use]
pub fn clone_remote(repository: &Repository) -> String {
    match repository.protocol {
        Some(CloneProtocol::Https) => {
            format!("https://github.com/{}.git", repository.name_with_owner)
        }
        Some(CloneProtocol::Ssh) => format!("git@github.com:{}.git", repository.name_with_owner),
        None => repository.clone_url.clone(),
    }
}

/// The folder a path is in; `None` for a bare name.
#[must_use]
pub fn parent_directory(path: &str) -> Option<&str> {
    let trimmed = path.trim_end_matches(['/', '\\']);
    let index = trimmed.rfind(['/', '\\'])?;
    if index == 0 {
        return trimmed.get(..1);
    }
    trimmed.get(..index)
}

/// `name` in `parent`, with the parent's own separator.
#[must_use]
pub fn join_directory_path(parent: &str, name: &str) -> String {
    let trimmed = parent.trim_end_matches(['/', '\\']);
    let separator = if trimmed.contains('\\') && !trimmed.contains('/') {
        '\\'
    } else {
        '/'
    };
    format!("{trimmed}{separator}{name}")
}

/// Where a clone or a new directory may go: the folders the machine's
/// projects are in, then Paseo's usual places, without repeats.
#[must_use]
pub fn suggested_parents(project_paths: &[String]) -> Vec<String> {
    let mut parents: Vec<String> = Vec::new();
    let usual = [
        "~/dev",
        "~/Developer",
        "~/src",
        "~/projects",
        "~/workspace",
        "~",
    ];
    let candidates = project_paths
        .iter()
        .filter_map(|path| parent_directory(path))
        .chain(usual);
    for candidate in candidates {
        if !parents.iter().any(|seen| seen == candidate) {
            parents.push(candidate.to_owned());
        }
    }
    parents
}

/// A path's identity for comparing: the home as `~`, `/` separators, no
/// trailing separator; a Windows path in lower case.
fn path_identity(path: &str) -> String {
    let shown = shorten_path(path.trim()).replace('\\', "/");
    let normalized = shown.trim_end_matches('/');
    let mut chars = normalized.chars();
    let drive = matches!(
        (chars.next(), chars.next(), chars.next()),
        (Some(letter), Some(':'), Some('/')) if letter.is_ascii_alphabetic()
    );
    if drive || normalized.starts_with("//") {
        normalized.to_lowercase()
    } else {
        normalized.to_owned()
    }
}

/// A clone destination: the parent it is cloned into, and the folder it
/// makes there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloneLocation {
    /// The parent sent to `project.clone`.
    pub parent: String,
    /// The folder the clone makes.
    pub path: String,
    /// That folder already exists (the row is disabled).
    pub exists: bool,
}

/// Each parent once (a home path and its `~` form are one), with the
/// folder the clone would make there.
#[must_use]
pub fn clone_locations(
    parents: &[String],
    repository_name: &str,
    existing: &[String],
) -> Vec<CloneLocation> {
    let existing: Vec<String> = existing.iter().map(|path| path_identity(path)).collect();
    let mut seen: Vec<String> = Vec::new();
    let mut locations = Vec::new();
    for parent in parents {
        let path = join_directory_path(parent, repository_name);
        let identity = path_identity(&path);
        if seen.contains(&identity) {
            continue;
        }
        let exists = existing.contains(&identity);
        seen.push(identity);
        locations.push(CloneLocation {
            parent: parent.clone(),
            path,
            exists,
        });
    }
    locations
}
