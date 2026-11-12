//! A git remote's URL read into the repository it names on a forge: its
//! host and its path (`owner/name`).
//!
//! [Decision Log]
//! - Purpose: the node finds a session's pull request from the remote its
//!   branch pushes to; every reference reads the same three forms (scp's
//!   `user@host:path`, `ssh://`, `https://`), takes the port separately
//!   and strips `.git` (Paseo's `git-remote.ts`, GOAT's `definition.ts`).
//! - Choice: scp's form only when there is no `://` (Paseo's rule, so a
//!   `C:\path` or a URL is never read as one); the host lower-cased; for
//!   `ssh` the port is dropped (it is the SSH server's, not the web's), for
//!   `http(s)` it is kept; user information is never kept. Anything else
//!   (a local path, `file://`, `ext::`) names no forge.

use vessel_proto::forge::{ForgeHost, RepoPath};

/// Where a remote's repository is.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Remote {
    /// The host, as the forge's web and API answer on it.
    pub host: ForgeHost,
    /// `owner/name`.
    pub path: RepoPath,
}

/// The repository `url` names, or `None` when it names no forge.
#[must_use]
pub fn parse(url: &str) -> Option<Remote> {
    let url = url.trim();
    let (host, path) = if let Some((scheme, rest)) = url.split_once("://") {
        let keep_port = match scheme.to_ascii_lowercase().as_str() {
            "https" | "http" => true,
            "ssh" | "git+ssh" | "ssh+git" | "git" => false,
            _ => return None,
        };
        let (authority, path) = rest.split_once('/')?;
        let host = authority
            .rsplit_once('@')
            .map_or(authority, |(_, host)| host);
        let host = if keep_port {
            host
        } else {
            host.split_once(':').map_or(host, |(name, _port)| name)
        };
        (host, path)
    } else {
        // `[user@]host:path`, with no `/` before the `:` (else it is a
        // local path).
        let (authority, path) = url.split_once(':')?;
        if authority.contains('/') || authority.is_empty() {
            return None;
        }
        let host = authority
            .rsplit_once('@')
            .map_or(authority, |(_, host)| host);
        (host, path)
    };
    let path = path
        .trim_matches('/')
        .split(['?', '#'])
        .next()
        .unwrap_or_default();
    let path = path.strip_suffix(".git").unwrap_or(path);
    Some(Remote {
        host: host.to_ascii_lowercase().parse().ok()?,
        path: path.parse().ok()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(url: &str) -> Option<(String, String)> {
        parse(url).map(|remote| (remote.host.to_string(), remote.path.to_string()))
    }

    fn named(host: &str, path: &str) -> (String, String) {
        (host.to_owned(), path.to_owned())
    }

    #[test]
    fn the_three_forms_name_one_repository() {
        for url in [
            "https://github.com/owner/name.git",
            "https://github.com/owner/name",
            "https://user@github.com/owner/name/",
            "git@github.com:owner/name.git",
            "ssh://git@github.com/owner/name.git",
            "ssh://git@GitHub.com:22/owner/name",
            "git://github.com/owner/name",
        ] {
            assert_eq!(read(url), Some(named("github.com", "owner/name")), "{url}");
        }
        assert_eq!(
            read("https://git.example.org:8443/group/sub/project.git"),
            Some(named("git.example.org:8443", "group/sub/project"))
        );
    }

    #[test]
    fn what_names_no_forge_is_none() {
        for url in [
            "/srv/repos/name.git",
            "./relative",
            "file:///srv/repos/name.git",
            "ext::sh -c evil",
            "C:\\repos\\name",
            "https://github.com/owner",
            "https://github.com/",
            "git@github.com:",
            "https://github.com/owner/../name",
        ] {
            assert_eq!(read(url), None, "{url}");
        }
    }
}
