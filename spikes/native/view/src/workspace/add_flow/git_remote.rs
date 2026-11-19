//! Git remote addresses, Paseo's `protocol/src/git-remote.ts` (pinned
//! 0.15.4; the page's `data/git-remote.ts`): which text is a complete
//! remote (an `https`, `http` or `ssh` URL, or the scp form) and which
//! GitHub repository it names.
//!
//! The node's clone URL takes these transports and `git`, so whatever is
//! taken here the node takes. Paseo reads URLs with the WHATWG parser; this
//! reads the same parts by hand (scheme, login, host, port, path) and
//! differs only on inputs no clone row needs: a special scheme without
//! `//` (`https:host/x`), dot segments in the path, and hosts outside
//! ASCII letters, digits, `.`, `_` and `-` (refused on both sides).

/// GitHub's cloud hosts (Paseo's forge manifest).
const GITHUB_HOSTS: [&str; 2] = ["github.com", "ssh.github.com"];

/// How a remote is reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    /// `user@host:path`.
    Scp,
    /// `ssh://`.
    Ssh,
    /// `http://`.
    Http,
    /// `https://`.
    Https,
}

impl Transport {
    fn from_scheme(scheme: &str) -> Option<(Self, &'static str)> {
        match scheme.to_ascii_lowercase().as_str() {
            "https" => Some((Self::Https, "443")),
            "http" => Some((Self::Http, "80")),
            "ssh" => Some((Self::Ssh, "22")),
            _ => None,
        }
    }
}

/// Where a remote points.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    /// Its transport.
    pub transport: Transport,
    /// The host, lower case, without trailing dots.
    pub host: String,
    /// A port other than the transport's default.
    pub port: Option<String>,
    /// The path without surrounding slashes and without `.git`.
    pub path: String,
}

fn normalize_host(host: &str) -> String {
    host.trim().trim_end_matches('.').to_lowercase()
}

/// Paseo's host rule: a letter or digit, then letters, digits, `.`, `_`
/// or `-`, ending on a letter or digit.
fn host_is_valid(host: &str) -> bool {
    let edge = |c: Option<char>| c.is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    edge(host.chars().next())
        && edge(host.chars().last())
        && host
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-'))
}

fn normalize_path(path: &str) -> Option<String> {
    let trimmed = path.trim().trim_matches('/');
    let path = trimmed.strip_suffix(".git").unwrap_or(trimmed);
    (!path.is_empty()).then(|| path.to_owned())
}

/// `%XX` decoded as UTF-8 (JavaScript's `decodeURIComponent`): `None` for
/// a bad escape or bytes that are not UTF-8.
fn percent_decode(text: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(text.len());
    let mut rest = text.bytes();
    while let Some(byte) = rest.next() {
        if byte != b'%' {
            bytes.push(byte);
            continue;
        }
        let high = char::from(rest.next()?).to_digit(16)?;
        let low = char::from(rest.next()?).to_digit(16)?;
        bytes.push(u8::try_from(high.checked_mul(16)?.checked_add(low)?).ok()?);
    }
    String::from_utf8(bytes).ok()
}

fn scp(trimmed: &str) -> Option<Location> {
    let (login, rest) = trimmed.split_once('@')?;
    let (host, path) = rest.split_once(':')?;
    if login.is_empty() || host.is_empty() || path.is_empty() {
        return None;
    }
    let host = normalize_host(host);
    let path = normalize_path(path)?;
    host_is_valid(&host).then_some(Location {
        transport: Transport::Scp,
        host,
        port: None,
        path,
    })
}

fn url(trimmed: &str) -> Option<Location> {
    let (scheme, rest) = trimmed.split_once("://")?;
    let (transport, default_port) = Transport::from_scheme(scheme)?;
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (authority, after) = rest.split_at_checked(end)?;
    let path = after.split(['?', '#']).next().unwrap_or_default();
    let host_port = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    let (host, port) = match host_port.rsplit_once(':') {
        Some((host, "")) => (host, None),
        Some((host, port)) if port.bytes().all(|byte| byte.is_ascii_digit()) => {
            (host, Some(port.parse::<u16>().ok()?.to_string()))
        }
        Some(_) => return None,
        None => (host_port, None),
    };
    let host = normalize_host(&percent_decode(host)?);
    let path = normalize_path(&percent_decode(path)?)?;
    let port = port.filter(|port| port != default_port);
    host_is_valid(&host).then_some(Location {
        transport,
        host,
        port,
        path,
    })
}

/// `remote` as a remote's parts; `None` when it is not a complete remote.
#[must_use]
pub fn location(remote: &str) -> Option<Location> {
    let trimmed = remote.trim();
    if trimmed.is_empty() {
        return None;
    }
    // The scp form has no scheme; tested first it would read
    // `ssh://git@host:22/x` as a login `ssh://git`.
    if trimmed.contains("://") {
        url(trimmed)
    } else {
        scp(trimmed)
    }
}

/// Whether `text` is a complete remote rather than `owner/repo`.
#[must_use]
pub fn is_complete(text: &str) -> bool {
    location(text).is_some()
}

/// The `owner/name` a remote names on GitHub.
#[must_use]
pub fn github_repository(remote: &str) -> Option<String> {
    let location = location(remote)?;
    if !GITHUB_HOSTS.contains(&location.host.as_str()) {
        return None;
    }
    let mut segments = location
        .path
        .split('/')
        .filter(|segment| !segment.is_empty());
    let (Some(owner), Some(name), None) = (segments.next(), segments.next(), segments.next())
    else {
        return None;
    };
    Some(format!("{owner}/{name}"))
}
