//! [`CloneUrl`]: what `project.clone` may fetch.
//!
//! [Decision Log]
//! - Purpose: the URL goes to `git clone`. git reads `ext::<command>` as a
//!   command to run, `-u<command>` as an option, and `file://` or a bare
//!   path as a folder on the node; a password in an `https` URL is written
//!   into `.git/config` and repeated in git's errors.
//! - Alternatives: passing the text after `--` and trusting git's own
//!   `protocol.allow` (`ext` is refused by git's defaults only since 2.12,
//!   and a node may run with a user configuration that allows it); a
//!   regular expression (hard to read and to test exhaustively).
//! - Choice: a type that accepts only the four network transports git
//!   ships (`https`, `http`, `ssh`, `git`) and the scp form, refuses any
//!   `<transport>::` form, a leading `-`, white space, control characters,
//!   and credentials, and is decoded through that parse. The node also
//!   passes it after `--` and with `GIT_ALLOW_PROTOCOL` set to those
//!   transports (ADR 0023): three walls.
//! - The scp form (`git@github.com:owner/repo.git`) is what most people
//!   copy from a forge; its user part is a login name, not a secret.

use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

/// A repository URL `git clone` may fetch.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct CloneUrl(Arc<str>);

/// Why a URL was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidCloneUrl {
    /// Empty, longer than 2,048 bytes, or holding white space or a control
    /// character.
    Text,
    /// Starts with `-`.
    Option,
    /// A transport other than `https`, `http`, `ssh` and `git`, or a
    /// `<transport>::` form.
    Transport,
    /// A local path or `file://`: a folder on the node is `project.add`.
    Local,
    /// A user and password, or a token, in an `http(s)` URL; a password in
    /// an `ssh` one.
    Credentials,
    /// No host.
    Host,
}

impl fmt::Display for InvalidCloneUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Text => "a clone URL is one word of at most 2,048 bytes",
            Self::Option => "a clone URL cannot start with `-`",
            Self::Transport => "a clone URL uses https, http, ssh, git, or user@host:path",
            Self::Local => "a folder on the machine is added as a project, not cloned",
            Self::Credentials => {
                "a clone URL cannot carry a password or a token; let git's credential helper keep it"
            }
            Self::Host => "a clone URL names a host",
        })
    }
}

impl std::error::Error for InvalidCloneUrl {}

impl CloneUrl {
    /// The longest URL, in bytes.
    pub const MAX_BYTES: usize = 2048;

    /// The URL.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The repository's name from its last path part, without `.git`: the
    /// default folder of a clone. `None` when that part is not a plain
    /// folder name (letters, digits, `.`, `_`, `-`; not starting with `.`).
    #[must_use]
    pub fn repository_name(&self) -> Option<&str> {
        let path = match self.0.split_once("://") {
            Some((_, rest)) => rest.split_once('/').map_or("", |(_, path)| path),
            None => self.0.split_once(':').map_or("", |(_, path)| path),
        };
        let last = path.trim_end_matches('/').rsplit('/').next()?;
        let name = last.strip_suffix(".git").unwrap_or(last);
        let plain = !name.is_empty()
            && !name.starts_with('.')
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
        plain.then_some(name)
    }
}

impl FromStr for CloneUrl {
    type Err = InvalidCloneUrl;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.is_empty()
            || text.len() > Self::MAX_BYTES
            || text.chars().any(|c| c.is_whitespace() || c.is_control())
        {
            return Err(InvalidCloneUrl::Text);
        }
        if text.starts_with('-') {
            return Err(InvalidCloneUrl::Option);
        }
        if text.contains("::") {
            return Err(InvalidCloneUrl::Transport);
        }
        match text.split_once("://") {
            Some((scheme, rest)) => url_form(scheme, rest)?,
            None => scp_form(text)?,
        }
        Ok(Self(Arc::from(text)))
    }
}

/// `scheme://[user@]host[:port]/path`.
fn url_form(scheme: &str, rest: &str) -> Result<(), InvalidCloneUrl> {
    let web = match scheme.to_ascii_lowercase().as_str() {
        "https" | "http" => true,
        "ssh" | "git" => false,
        "file" => return Err(InvalidCloneUrl::Local),
        _ => return Err(InvalidCloneUrl::Transport),
    };
    let authority = rest
        .split_once('/')
        .map_or(rest, |(authority, _)| authority);
    let host = match authority.rsplit_once('@') {
        Some((user, host)) => {
            if web || user.contains(':') {
                return Err(InvalidCloneUrl::Credentials);
            }
            if user.starts_with('-') {
                return Err(InvalidCloneUrl::Option);
            }
            host
        }
        None => authority,
    };
    if host.is_empty() || host.starts_with(':') || host.starts_with('-') {
        return Err(InvalidCloneUrl::Host);
    }
    Ok(())
}

/// `[user@]host:path`, git's scp-like form: a `:` before any `/`. A single
/// letter before the `:` is a Windows drive, a local path.
fn scp_form(text: &str) -> Result<(), InvalidCloneUrl> {
    let Some((before, _)) = text.split_once(':') else {
        return Err(InvalidCloneUrl::Local);
    };
    if before.contains('/') {
        return Err(InvalidCloneUrl::Local);
    }
    let (user, host) = before
        .rsplit_once('@')
        .map_or(("", before), |(user, host)| (user, host));
    if user.starts_with('-') {
        return Err(InvalidCloneUrl::Option);
    }
    if host.is_empty() || host.starts_with('-') {
        return Err(InvalidCloneUrl::Host);
    }
    if host.len() == 1 {
        return Err(InvalidCloneUrl::Local);
    }
    Ok(())
}

impl fmt::Display for CloneUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

crate::key::serde_via_text!(CloneUrl);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_urls_and_the_scp_form_are_taken() {
        for (url, name) in [
            ("https://github.com/owner/repo.git", Some("repo")),
            ("https://example.com/group/sub/project", Some("project")),
            ("http://example.com/repo/", Some("repo")),
            ("ssh://git@example.com:2222/owner/repo.git", Some("repo")),
            ("git://example.com/repo.git", Some("repo")),
            ("git@github.com:owner/repo.git", Some("repo")),
            ("example.com:repo", Some("repo")),
            ("https://example.com", None),
        ] {
            let parsed: CloneUrl = url.parse().unwrap_or_else(|e| panic!("{url}: {e}"));
            assert_eq!(parsed.repository_name(), name, "{url}");
        }
    }

    #[test]
    fn commands_options_local_paths_and_secrets_are_refused() {
        for (url, problem) in [
            ("", InvalidCloneUrl::Text),
            ("https://a b/c", InvalidCloneUrl::Text),
            ("https://a/b\n", InvalidCloneUrl::Text),
            ("-uevil", InvalidCloneUrl::Option),
            ("--upload-pack=touch /tmp/x", InvalidCloneUrl::Text),
            ("ext::sh -c touch% /tmp/x", InvalidCloneUrl::Text),
            ("ext::sh", InvalidCloneUrl::Transport),
            ("fd::17", InvalidCloneUrl::Transport),
            ("transport::https://x", InvalidCloneUrl::Transport),
            ("ftp://example.com/repo", InvalidCloneUrl::Transport),
            ("file:///home/me/repo", InvalidCloneUrl::Local),
            ("/home/me/repo", InvalidCloneUrl::Local),
            ("../repo", InvalidCloneUrl::Local),
            ("C:/repo", InvalidCloneUrl::Local),
            ("c:repo", InvalidCloneUrl::Local),
            (
                "https://user:pass@github.com/o/r",
                InvalidCloneUrl::Credentials,
            ),
            (
                "https://ghp_token@github.com/o/r",
                InvalidCloneUrl::Credentials,
            ),
            ("ssh://me:pw@host/r", InvalidCloneUrl::Credentials),
            ("https:///repo", InvalidCloneUrl::Host),
            ("ssh://-oProxyCommand=x/r", InvalidCloneUrl::Host),
            ("-x:repo", InvalidCloneUrl::Option),
            ("@:repo", InvalidCloneUrl::Host),
            ("-oProxyCommand=x@host:repo", InvalidCloneUrl::Option),
            ("ssh://-oProxyCommand=x@host/r", InvalidCloneUrl::Option),
        ] {
            assert_eq!(url.parse::<CloneUrl>(), Err(problem), "{url:?}");
        }
        assert!(serde_json::from_str::<CloneUrl>(r#""ext::sh""#).is_err());
    }
}
