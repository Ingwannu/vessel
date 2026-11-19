//! Add project's directory rows: the machine's project roots that match
//! the field, then the machine's `folder.suggest` answer, and a typed path
//! first. Paseo's `project-picker-options.ts`,
//! `working-directory-suggestions.ts`, `shorten-path.ts` and
//! `search/text-match.ts`'s `scorePathMatch` (pinned 0.15.4; the page's
//! `add-project/paths.ts` and `data/text-match.ts`).
//!
//! The roots are the machine's projects' (plan difference 4); a machine
//! below `folder.suggest` gives no answer, so its rows are those and a
//! typed path (difference 3). Only whether a root matches is used, so the
//! match is kept as a yes or no, not Paseo's tiers.

/// A directory row: a typed path ("Open this path") or a suggestion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryOption {
    /// Typed by hand rather than suggested.
    pub typed: bool,
    /// The path.
    pub path: String,
}

/// The home folder as `~`, for a macOS or Linux home (Paseo's rule:
/// `/Users/<name>` or `/home/<name>` at the start).
#[must_use]
pub fn shorten_path(path: &str) -> String {
    for home in ["/Users/", "/home/"] {
        let Some(rest) = path.strip_prefix(home) else {
            continue;
        };
        let name_end = rest.find('/').unwrap_or(rest.len());
        if name_end == 0 {
            continue;
        }
        let after = rest.get(name_end..).unwrap_or_default();
        return format!("~{after}");
    }
    path.to_owned()
}

/// Whether typed text is a path the node can open: absolute, `~`, a
/// Windows drive, or a UNC share.
#[must_use]
pub fn is_openable(query: &str) -> bool {
    let trimmed = query.trim();
    let mut chars = trimmed.chars();
    let drive = matches!(
        (chars.next(), chars.next(), chars.next()),
        (Some(letter), Some(':'), Some('\\' | '/')) if letter.is_ascii_alphabetic()
    );
    trimmed.starts_with('/') || trimmed.starts_with('~') || trimmed.starts_with("\\\\") || drive
}

fn unique(paths: impl IntoIterator<Item = impl AsRef<str>>) -> Vec<String> {
    let mut ordered: Vec<String> = Vec::new();
    for path in paths {
        let trimmed = path.as_ref().trim();
        if !trimmed.is_empty() && !ordered.iter().any(|seen| seen == trimmed) {
            ordered.push(trimmed.to_owned());
        }
    }
    ordered
}

fn normalize(value: &str) -> String {
    value.trim().replace('\\', "/").to_lowercase()
}

/// The query's characters in order within one word of `text` (no white
/// space between them): Paseo's subsequence tier.
fn in_one_word(query: &[char], text: &str) -> bool {
    let mut matched = 0_usize;
    for character in text.chars() {
        if matched == query.len() {
            break;
        }
        if character.is_whitespace() {
            matched = 0;
        } else if query.get(matched) == Some(&character) {
            matched = matched.saturating_add(1);
        }
    }
    matched == query.len()
}

/// Paseo's `scoreMatch` as a yes or no: equal, contained, or in order
/// within one word, ignoring case.
fn matches_text(query: &str, text: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let query = query.to_lowercase();
    let text = text.to_lowercase();
    let chars: Vec<char> = query.chars().collect();
    text.contains(&query) || in_one_word(&chars, &text)
}

fn compact(value: &str) -> String {
    value
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// Paseo's `scorePathMatch` as a yes or no: the query against the whole
/// path, separators included; else both without their separators
/// (`blank page` finds `blankpage/`).
#[must_use]
pub fn path_matches(query: &str, path: &str) -> bool {
    if matches_text(query, path) {
        return true;
    }
    let (query, path) = (compact(query), compact(path));
    !query.is_empty() && !path.is_empty() && matches_text(&query, &path)
}

fn recent_matches(path: &str, query: &str) -> bool {
    let wanted = normalize(query);
    wanted == "~" || wanted == "~/" || path_matches(&wanted, &normalize(path))
}

/// The roots matching the field (all of them for an empty field), then the
/// machine's answer as it ranked it, without repeats.
#[must_use]
pub fn suggestions(recent: &[String], machine: &[String], query: &str) -> Vec<String> {
    let query = query.trim();
    let recent = unique(recent);
    if query.is_empty() {
        return recent;
    }
    unique(
        recent
            .iter()
            .filter(|path| recent_matches(path, query))
            .chain(machine.iter()),
    )
}

/// The directory rows: a typed openable path not among the suggestions
/// first, then the suggestions.
#[must_use]
pub fn directory_options(
    recent: &[String],
    machine: &[String],
    query: &str,
) -> Vec<DirectoryOption> {
    let suggested = suggestions(recent, machine, query);
    let typed = query.trim();
    let first = (is_openable(typed) && !suggested.iter().any(|path| path == typed)).then(|| {
        DirectoryOption {
            typed: true,
            path: typed.to_owned(),
        }
    });
    first
        .into_iter()
        .chain(
            suggested
                .into_iter()
                .map(|path| DirectoryOption { typed: false, path }),
        )
        .collect()
}
