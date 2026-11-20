//! "Open on <forge>" (TRACKER 467 step B, ADR 0123 "Step B"): the web page
//! of the workspace's branch, or of the shown file at its line, on the
//! forge a status's `origin` names. Paseo's rules (pinned 0.15.4), as the
//! page has them in `web/src/data/forge-links.ts`: the forge
//! (`git/forge.ts:49`, `protocol/forge-manifest.ts:59-111`), the web
//! location (`git/forge-url.ts:69`), the branch tree (`:124`), the file
//! (`:137`, its path `:102`), each forge's infixes and line anchor
//! (`git/forges/github.ts:85`, `gitlab.ts:23,163`,
//! `git/client-forge-module.ts:47-60`).
//!
//! [Decision Log]
//! - Purpose: Paseo's open-in-editor list ends with the forge's page
//!   (`open-in-editor/planner.ts:179`), on any machine (its forge plan does
//!   not ask whether the daemon is local).
//! - Constraint: the node sends `origin`'s host and path only, never its
//!   address (which may hold a token), and a kind only for a host it alone
//!   knows (a self-hosted GitHub it holds a token for); the public hosts of
//!   every forge are mapped here, as Paseo's client maps them.
//! - Choice: Paseo's five forges in its order, as pure rules; a page is
//!   always `https://` (the app opens nothing else, ADR 0078's opener rule).
//! - Cost: the page has the same rules in TypeScript; both run Paseo's
//!   `forge-url.test.ts` cases.

use std::fmt::Write as _;

use vessel_proto::forge::{ForgeHost, ForgeKind, RepoPath};

/// A forge Paseo links to, in its manifest's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Forge {
    /// GitHub (and GitHub Enterprise).
    GitHub,
    /// GitLab.
    GitLab,
    /// Gitea.
    Gitea,
    /// Forgejo: no public host, so only a node's word could name it.
    Forgejo,
    /// Codeberg.
    Codeberg,
}

impl Forge {
    /// Every forge, in Paseo's order.
    pub const ALL: [Self; 5] = [
        Self::GitHub,
        Self::GitLab,
        Self::Gitea,
        Self::Forgejo,
        Self::Codeberg,
    ];

    /// Paseo's id, which a preferred editor stores.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::GitHub => "github",
            Self::GitLab => "gitlab",
            Self::Gitea => "gitea",
            Self::Forgejo => "forgejo",
            Self::Codeberg => "codeberg",
        }
    }

    /// The forge `id` names.
    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|forge| forge.id() == id)
    }

    /// The brand's name ("Open on GitHub").
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::GitHub => "GitHub",
            Self::GitLab => "GitLab",
            Self::Gitea => "Gitea",
            Self::Forgejo => "Forgejo",
            Self::Codeberg => "Codeberg",
        }
    }

    /// Its public hosts, the web host first (`forge-manifest.ts`).
    const fn cloud_hosts(self) -> &'static [&'static str] {
        match self {
            Self::GitHub => &["github.com", "ssh.github.com"],
            Self::GitLab => &["gitlab.com"],
            Self::Gitea => &["gitea.com"],
            Self::Forgejo => &[],
            Self::Codeberg => &["codeberg.org"],
        }
    }

    const fn tree_infix(self) -> &'static str {
        match self {
            Self::GitHub => "/tree/",
            Self::GitLab => "/-/tree/",
            Self::Gitea | Self::Forgejo | Self::Codeberg => "/src/branch/",
        }
    }

    const fn blob_infix(self) -> &'static str {
        match self {
            Self::GitHub => "/blob/",
            Self::GitLab => "/-/blob/",
            Self::Gitea | Self::Forgejo | Self::Codeberg => "/src/branch/",
        }
    }

    /// `#L1`, or a range: `#L1-L3` (GitHub, the Gitea family), `#L1-3`
    /// (GitLab).
    fn anchor(self, start: u32, end: Option<u32>) -> String {
        match (self, end.filter(|end| *end > start)) {
            (_, None) => format!("#L{start}"),
            (Self::GitLab, Some(end)) => format!("#L{start}-{end}"),
            (Self::GitHub | Self::Gitea | Self::Forgejo | Self::Codeberg, Some(end)) => {
                format!("#L{start}-L{end}")
            }
        }
    }
}

/// `host` without the port the node keeps for an `http(s)` address.
fn host_name(host: &str) -> &str {
    host.rsplit_once(':').map_or(host, |(name, _port)| name)
}

/// The forge `origin` is on: the node's word for a host it alone knows,
/// else the public host's (Paseo: `resolvedForge ?? forgeFromRemoteUrl`).
#[must_use]
pub fn forge_of(host: &ForgeHost, kind: Option<ForgeKind>) -> Option<Forge> {
    if kind == Some(ForgeKind::Github) {
        return Some(Forge::GitHub);
    }
    let name = host_name(host.as_str());
    Forge::ALL
        .into_iter()
        .find(|forge| forge.cloud_hosts().contains(&name))
}

/// A file to link and the lines its view was opened at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkedFile<'a> {
    /// Its path in the repository.
    pub path: &'a str,
    /// The first line, from 1.
    pub line_start: Option<u32>,
    /// The last line, when a range.
    pub line_end: Option<u32>,
}

/// The page on `forge` of `branch` in the repository at `host`/`path`, or
/// of `file` on it; `None` with no branch (a detached `HEAD`) or a file
/// outside the repository (Paseo's `buildForgeBranchTreeUrl` and
/// `buildForgeBlobUrl`).
#[must_use]
pub fn page(
    forge: Forge,
    host: &ForgeHost,
    path: &RepoPath,
    branch: &str,
    file: Option<&LinkedFile<'_>>,
) -> Option<String> {
    let branch = branch.trim();
    if branch.is_empty() || branch == "HEAD" {
        return None;
    }
    let host = host.as_str();
    // A public host is its forge's web host (`ssh.github.com` is GitHub's
    // `github.com`); any other keeps the port the node kept.
    let authority = if forge.cloud_hosts().contains(&host_name(host)) {
        forge.cloud_hosts().first().copied().unwrap_or(host)
    } else {
        host
    };
    let base = format!("https://{authority}/{path}");
    let reference = encode_segments(branch);
    let Some(file) = file else {
        return Some(format!("{base}{}{reference}", forge.tree_infix()));
    };
    let file_path = blob_path(file.path)?;
    let anchor = file
        .line_start
        .filter(|start| *start > 0)
        .map(|start| forge.anchor(start, file.line_end))
        .unwrap_or_default();
    Some(format!(
        "{base}{}{reference}/{}{anchor}",
        forge.blob_infix(),
        encode_segments(&file_path)
    ))
}

/// A file's path within the repository, or `None` when it climbs out of
/// it (`forge-url.ts:102`).
fn blob_path(path: &str) -> Option<String> {
    let normalised = path.trim().replace('\\', "/");
    let mut segments: Vec<&str> = Vec::new();
    for segment in normalised.trim_start_matches('/').split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                let _parent = segments.pop()?;
            }
            _ => segments.push(segment),
        }
    }
    (!segments.is_empty()).then(|| segments.join("/"))
}

/// Each `/`-separated part as `encodeURIComponent` writes it.
fn encode_segments(text: &str) -> String {
    text.split('/')
        .map(encode_component)
        .collect::<Vec<_>>()
        .join("/")
}

/// `encodeURIComponent`: everything but `A-Z a-z 0-9 - _ . ! ~ * ' ( )`
/// as `%XX` of its UTF-8 bytes.
fn encode_component(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte) {
            out.push(char::from(byte));
        } else {
            let _written = write!(out, "%{byte:02X}");
        }
    }
    out
}

#[cfg(test)]
#[path = "forge_tests.rs"]
mod tests;
