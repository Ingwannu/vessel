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

/// Where `url`'s repository's web pages are (TRACKER 467 step B, ADR
/// 0123), read as Paseo reads a remote for its "Open on" link
/// (`git-remote.ts:55`, `forge-url.ts:69`): `user@host:path` (scp form,
/// with a user and without `://`), or an `https`, `http` or `ssh` URL. The
/// host is lower-cased with trailing dots dropped; the path is
/// percent-decoded, trimmed of slashes and `.git`, and may not climb with
/// `..`. A port is kept only for `http(s)` and only when it is not the
/// scheme's own (an SSH port is the SSH server's, not the web's). User
/// information is never kept. `None` for anything else (`git://`, a path,
/// `file://`).
///
/// [`parse`] stays as pull requests read remotes (it also reads `git://`
/// and a user-less `host:path`); the link follows Paseo's narrower forms.
#[must_use]
pub fn web(url: &str) -> Option<Remote> {
    let url = url.trim();
    let (host, port, path) = if url.contains("://") {
        let parsed = reqwest::Url::parse(url).ok()?;
        let port = match parsed.scheme() {
            // `port()` is `None` for the scheme's own port.
            "https" | "http" => parsed.port(),
            "ssh" => None,
            _ => return None,
        };
        (parsed.host_str()?.to_owned(), port, decoded(parsed.path())?)
    } else {
        // Paseo's `^[^@]+@([^:]+):(.+)$`.
        let (user, rest) = url.split_once('@')?;
        let (host, path) = rest.split_once(':')?;
        if user.is_empty() || host.is_empty() || path.is_empty() {
            return None;
        }
        (host.to_owned(), None, path.to_owned())
    };
    let host = host.trim().trim_end_matches('.').to_ascii_lowercase();
    let path = path.trim().trim_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    if path.split('/').any(|segment| segment == "..") {
        return None;
    }
    let host = match port {
        Some(port) => format!("{host}:{port}"),
        None => host,
    };
    Some(Remote {
        host: host.parse().ok()?,
        path: path.parse().ok()?,
    })
}

/// `path` percent-decoded, as `decodeURIComponent`: `None` for a `%` not
/// followed by two hex digits, or bytes that are not UTF-8.
fn decoded(path: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(path.len());
    let mut rest = path.as_bytes();
    while let Some((&byte, tail)) = rest.split_first() {
        if byte == b'%' {
            let (hex, tail) = tail.split_at_checked(2)?;
            if !hex.iter().all(u8::is_ascii_hexdigit) {
                return None;
            }
            bytes.push(u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok()?);
            rest = tail;
        } else {
            bytes.push(byte);
            rest = tail;
        }
    }
    String::from_utf8(bytes).ok()
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

    fn web_of(url: &str) -> Option<(String, String)> {
        web(url).map(|remote| (remote.host.to_string(), remote.path.to_string()))
    }

    /// Paseo's `git-remote.test.ts` forms, as the link reads them.
    #[test]
    fn the_link_reads_paseo_s_forms() {
        for url in [
            "https://github.com/owner/repo",
            "  https://github.com/owner/repo  ",
            "https://user:token@github.com/owner/repo.git",
            "https://GitHub.com./owner/repo/",
            "ssh://git@github.com/owner/repo",
            "ssh://git@github.com:22/owner/repo.git",
            "git@github.com:owner/repo.git",
            "https://github.com:443/owner/repo",
            "https://github.com/owner/re%70o",
        ] {
            assert_eq!(web_of(url), Some(named("github.com", "owner/repo")), "{url}");
        }
        assert_eq!(
            web_of("https://home-git.example.com:60443/team/repo.git"),
            Some(named("home-git.example.com:60443", "team/repo"))
        );
        assert_eq!(
            web_of("http://internal.example.com:3000/team/repo.git"),
            Some(named("internal.example.com:3000", "team/repo"))
        );
        // An SSH port is the SSH server's, never the web's.
        assert_eq!(
            web_of("ssh://git@example.com:2222/acme/repo.git"),
            Some(named("example.com", "acme/repo"))
        );
        assert_eq!(
            web_of("git@gitlab.example.com:group/sub/project.git"),
            Some(named("gitlab.example.com", "group/sub/project"))
        );
    }

    #[test]
    fn the_link_refuses_what_paseo_refuses() {
        for url in [
            "",
            "owner/repo",
            "git://github.com/owner/repo",
            "ftp://host/owner/repo",
            "file:///tmp/owner/repo",
            "github.com:owner/repo",
            "@github.com:owner/repo",
            "git@github.com:owner/../repo",
            "https://github.com/owner/re%zzpo",
            "https://github.com/owner/re%ffpo",
            "https://[::1]/owner/repo",
            "/srv/repos/name.git",
            "C:\\repos\\name",
        ] {
            assert_eq!(web_of(url), None, "{url}");
        }
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
