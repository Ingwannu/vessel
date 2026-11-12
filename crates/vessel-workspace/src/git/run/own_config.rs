//! A repository's own configuration, judged before every run (ADR 0030):
//! what of it the run turns off on its command line, and what makes the
//! run refuse. Reading it is `output`'s (one `git config --list` in the
//! run's own slot and time); this module only decides.
//!
//! [Decision Log]
//! - Purpose: a repository's `.git/config` (and `config.worktree`, and
//!   every file they include) is written by whoever can write in its
//!   folder: an agent whose tools may edit files but not run commands, or
//!   whoever packed the folder the user added. Several keys there name a
//!   program git runs (a filter, a diff or merge driver, an `ssh`, a
//!   credential helper, a signing program). The node runs git on the
//!   user's behalf, often unasked (the page's status, the forge's
//!   watcher), so such a key would run as the user without anyone having
//!   chosen it (wave 1's review, finding 1).
//! - Constraint: git cannot be told to skip a repository's configuration
//!   (no variable does it), and the user's own global and system
//!   configuration must keep working (their credential helper, their
//!   `ssh`, `git lfs` installed globally). `GIT_ATTR_SOURCE` (git 2.40)
//!   would hide `.gitattributes` but not `.git/info/attributes`
//!   (measured on 2.43), and the node supports git from 2.25.
//! - Alternatives: refuse every run in a repository whose configuration
//!   names any program (a per-repository `core.sshCommand` for a deploy
//!   key would then refuse even `git status`); a node-held copy of the
//!   configuration (git always reads `$GIT_COMMON_DIR/config`); overrides
//!   on the command line only for keys known in advance (a driver's name
//!   is the repository's choice, so its keys cannot be known).
//! - Choice: list the repository's own configuration first (`git config
//!   --local --list -z --includes`, and `--worktree` when it is on), then
//!   (1) turn off on the command line every filter and merge driver it
//!   defines (a `-c` beats the repository's files and their includes; an
//!   empty filter command turns the filter off, an empty merge driver
//!   makes git's merge of that file fail rather than run anything,
//!   measured), and (2) refuse the run when
//!   it sets a key the run would use and that cannot be turned off
//!   without losing the user's own global value (`ssh`, credential
//!   helpers, signing programs), or one that moves the working tree.
//!   A diff driver (`diff.<driver>.command`, `.textconv`) cannot be
//!   emptied (git then runs an empty command and fails, measured); every
//!   diff the node asks for passes `--no-ext-diff --no-textconv` instead
//!   (ADR 0023), which ignore both.
//!   Keys git runs only from commands the node never runs (`difftool`,
//!   `mergetool`, `submodule.<name>.update`, `trailer.*.cmd`,
//!   `sendemail.*`, `web.browser`) are left alone; aliases never shadow
//!   git's own commands, so they cannot either.
//! - Cost: one more `git config` per run (about 2 ms), inside the run's
//!   slot and time; a race with a process of the user's that rewrites the
//!   file between the listing and the run is not closed (ADR 0027: these
//!   are checks inside the node, not a sandbox).
//! - Must stay true: a key git adds later that names a program is not
//!   covered until it is listed here, with a test that plants it
//!   (`own_config/tests.rs`).

use std::collections::BTreeSet;

/// What a run will do that a repository's configuration could name a
/// program for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Uses {
    /// It talks to a remote (fetch, pull, push).
    pub(crate) network: bool,
    /// It runs the repository's hooks (commit, push), and so may sign.
    pub(crate) hooks: bool,
}

/// Why a run was refused for its repository's own configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnConfigRefusal {
    /// The key names a program this run would start, and turning it off
    /// would also turn off the user's own global setting.
    Program,
    /// The key moves the working tree out of the folder.
    Layout,
    /// The key could send the credential a network run gets to someone
    /// else (a proxy, a weakened TLS check, a resolver, an extra header).
    Credential,
    /// A driver's name cannot be written on git's command line (it holds
    /// `=`), so it cannot be turned off there.
    Unnamable,
    /// More drivers than one run turns off.
    TooMany,
}

impl std::fmt::Display for OwnConfigRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Program => {
                "it names a program for git to run; Vessel's git runs no program a repository's \
                 own configuration names (set it in your global configuration, under `includeIf`, \
                 for Vessel to use it)"
            }
            Self::Layout => "it moves the working tree out of the project's folder",
            Self::Credential => {
                "it could send this run's credential elsewhere (a proxy, a TLS check, a \
                 resolver or a header); Vessel's git does not take it from a repository's own \
                 configuration"
            }
            Self::Unnamable => "its name cannot be turned off on git's command line",
            Self::TooMany => "it defines more drivers than one run turns off",
        })
    }
}

/// A run refused: the key (never its value) and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Refused {
    pub(crate) key: String,
    pub(crate) why: OwnConfigRefusal,
}

/// The most drivers of one kind (filter, merge) a run turns off.
/// Each costs a few `-c` arguments; Windows bounds a command line at
/// 32 KiB.
const MAX_DRIVERS: usize = 64;

/// One `key=value` entry of `git config --list -z`, split. git writes
/// the section and the variable in lower case and the subsection as
/// written.
#[derive(Debug, PartialEq, Eq)]
struct Entry<'a> {
    key: &'a str,
    section: &'a str,
    subsection: Option<&'a str>,
    name: &'a str,
    value: Option<&'a str>,
}

/// The entries of `git config --list -z`: each `key\nvalue\0`, or `key\0`
/// for a key without a value. Entries that are not text are skipped: git
/// writes keys it parsed, so a key is always text.
fn entries(listing: &[u8]) -> impl Iterator<Item = Entry<'_>> {
    listing
        .split(|byte| *byte == 0)
        .filter(|raw| !raw.is_empty())
        .filter_map(|raw| std::str::from_utf8(raw).ok())
        .filter_map(|text| {
            let (key, value) = match text.split_once('\n') {
                Some((key, value)) => (key, Some(value)),
                None => (text, None),
            };
            let (section, rest) = key.split_once('.')?;
            let (subsection, name) = match rest.rsplit_once('.') {
                Some((subsection, name)) => (Some(subsection), name),
                None => (None, rest),
            };
            Some(Entry {
                key,
                section,
                subsection,
                name,
                value,
            })
        })
}

/// git's boolean: no value, `true`, `yes`, `on` or a non-zero number.
fn truthy(value: Option<&str>) -> bool {
    value.is_none_or(|value| {
        let value = value.trim();
        value.eq_ignore_ascii_case("true")
            || value.eq_ignore_ascii_case("yes")
            || value.eq_ignore_ascii_case("on")
            || value.parse::<i64>().is_ok_and(|number| number != 0)
    })
}

/// Whether the repository keeps a configuration per worktree as well
/// (`extensions.worktreeConfig`), which `--local` does not list.
pub(crate) fn has_worktree_config(listing: &[u8]) -> bool {
    entries(listing).any(|entry| {
        entry.section == "extensions"
            && entry.subsection.is_none()
            && entry.name == "worktreeconfig"
            && truthy(entry.value)
    })
}

/// The `-c` settings that turn off every filter and merge driver
/// `listing` defines, or the key that makes a run that `uses` this
/// refuse.
///
/// # Errors
/// [`Refused`] for the first key the run cannot go ahead with.
pub(crate) fn judge(listing: &[u8], uses: Uses) -> Result<Vec<String>, Refused> {
    let mut filters = BTreeSet::new();
    let mut merges = BTreeSet::new();
    for entry in entries(listing) {
        let refuse = |why| {
            Err(Refused {
                key: entry.key.to_owned(),
                why,
            })
        };
        let driver = match (entry.section, entry.subsection, entry.name) {
            ("filter", Some(driver), "clean" | "smudge" | "process") => {
                Some((&mut filters, driver))
            }
            ("merge", Some(driver), "driver") => Some((&mut merges, driver)),
            _ => None,
        };
        if let Some((set, driver)) = driver {
            // `-c` splits at the first `=`: a driver named with one could
            // not be turned off, and would run.
            if driver.contains('=') {
                return refuse(OwnConfigRefusal::Unnamable);
            }
            let _new = set.insert(driver);
            if set.len() > MAX_DRIVERS {
                return refuse(OwnConfigRefusal::TooMany);
            }
            continue;
        }
        if let Some(why) = why_refused(&entry, uses) {
            return refuse(why);
        }
    }
    let mut settings = Vec::new();
    for driver in filters {
        for name in ["clean", "smudge", "process"] {
            settings.push(format!("filter.{driver}.{name}="));
        }
        settings.push(format!("filter.{driver}.required=false"));
    }
    for driver in merges {
        settings.push(format!("merge.{driver}.driver="));
    }
    Ok(settings)
}

/// Whether `entry` refuses a run that `uses` this, and why. Everything a
/// run turns off on every run whatever the repository says (`fsmonitor`,
/// the hooks folder, an external diff, a pager, an editor) is in `run`'s
/// fixed overrides, not here.
fn why_refused(entry: &Entry<'_>, uses: Uses) -> Option<OwnConfigRefusal> {
    let program = match (entry.section, entry.subsection, entry.name) {
        // Where the working tree is: a reader of the project would read
        // another folder. (`core.bare` is not refused: a bare repository's
        // linked worktrees keep it in the configuration they share, and
        // git ignores it there; a bare repository is refused as a project
        // when it is added.)
        ("core", None, "worktree") => return Some(OwnConfigRefusal::Layout),
        // Programs a network run starts.
        ("core", None, "sshcommand" | "gitproxy" | "alternaterefscommand")
        | ("credential", _, "helper")
        | ("remote", Some(_), "uploadpack" | "receivepack" | "vcs") => uses.network,
        // Where a network run's credential would go: a proxy, and the
        // checks that keep a proxy or a resolver from reading it.
        (
            "http",
            _,
            "proxy" | "sslverify" | "sslcainfo" | "sslcapath" | "sslcert" | "sslkey" | "sslbackend"
            | "curloptresolve" | "extraheader" | "cookiefile",
        )
        | ("remote", Some(_), "proxy") => {
            return uses.network.then_some(OwnConfigRefusal::Credential);
        }
        // Programs a commit or a push signs with.
        ("gpg", _, "program") | ("gpg", Some("ssh"), "defaultkeycommand") => uses.hooks,
        _ => false,
    };
    program.then_some(OwnConfigRefusal::Program)
}

#[cfg(test)]
mod tests;
