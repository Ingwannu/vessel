//! Structural release parsing preserves Markdown rather than guessing block kinds.

use std::sync::Arc;

/// Render only the newest release while a sheet enters.
pub const LATEST_ONLY: usize = 1;
/// The reference's initial settled page.
pub const FIRST_PAGE: usize = 5;
/// Additional releases revealed by each explicit request.
pub const PAGE_SIZE: usize = 10;

/// A release heading and its unchanged Markdown body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    version: Arc<str>,
    date: Arc<str>,
    markdown: Arc<str>,
}

impl Release {
    /// Authored version without link, bracket and leading-v decoration.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }
    /// Authored date remains text unless a locale formatter recognizes it.
    #[must_use]
    pub fn date(&self) -> &str {
        &self.date
    }
    /// Section headings and bodies are retained for the shared Markdown renderer.
    #[must_use]
    pub fn markdown(&self) -> &str {
        &self.markdown
    }
}

/// A sheet's release budget is independent of the document's length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Revealed(usize);

impl Default for Revealed {
    fn default() -> Self {
        Self(LATEST_ONLY)
    }
}

impl Revealed {
    /// Entering animations finish before the rest of the first page appears.
    pub fn settled(&mut self) {
        self.0 = self.0.max(FIRST_PAGE);
    }
    /// Saturation prevents an adversarial number of clicks wrapping the budget.
    pub fn show_more(&mut self) {
        self.0 = self.0.saturating_add(PAGE_SIZE);
    }
    /// Bound the render slice at the document boundary.
    #[must_use]
    pub const fn count(self) -> usize {
        self.0
    }
}

/// Parse only release boundaries outside fenced code; all section markup survives.
#[must_use]
pub fn parse(markdown: &str) -> Vec<Release> {
    let mut releases = Vec::new();
    let mut heading: Option<(String, String)> = None;
    let mut body = Vec::new();
    let mut fence: Option<(char, usize)> = None;
    for line in markdown.lines() {
        if let Some((kind, count)) = fence_edge(line) {
            match fence {
                None => fence = Some((kind, count)),
                Some((opened, length)) if opened == kind && count >= length => fence = None,
                Some(_) => {}
            }
            body.push(line);
            continue;
        }
        if fence.is_none()
            && let Some(title) = release_heading(line)
        {
            finish(&mut releases, heading.take(), &body);
            body.clear();
            heading = Some(split_heading(title));
        } else if heading.is_some() {
            body.push(line);
        }
    }
    finish(&mut releases, heading, &body);
    releases
}

fn finish(out: &mut Vec<Release>, heading: Option<(String, String)>, body: &[&str]) {
    if let Some((version, date)) = heading {
        out.push(Release {
            version: version.into(),
            date: date.into(),
            markdown: body.join("\n").trim_matches('\n').into(),
        });
    }
}

fn fence_edge(line: &str) -> Option<(char, usize)> {
    const FENCE_MIN: usize = 3;
    let line = line.trim_start();
    let kind = line.chars().next()?;
    if kind != '`' && kind != '~' {
        return None;
    }
    let length = line.chars().take_while(|c| *c == kind).count();
    (length >= FENCE_MIN).then_some((kind, length))
}

fn release_heading(line: &str) -> Option<&str> {
    const INDENT_MAX: usize = 3;
    let indent = line.chars().take_while(|c| *c == ' ').count();
    if indent > INDENT_MAX {
        return None;
    }
    let title = line.trim_start_matches(' ').strip_prefix("##")?;
    if title.is_empty() || title.starts_with([' ', '\t']) {
        Some(title.trim().trim_end_matches('#').trim_end())
    } else {
        None
    }
}

fn split_heading(heading: &str) -> (String, String) {
    let heading = remove_links(heading);
    let parts = [" - ", " – ", " — "]
        .iter()
        .find_map(|separator| heading.split_once(separator));
    let (version, date) = if let Some(parts) = parts {
        parts
    } else if let Some((version, date)) = heading.strip_suffix(')').and_then(|s| s.rsplit_once('('))
    {
        (version, date)
    } else {
        (heading.as_str(), "")
    };
    let version = version
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim();
    let version = version
        .strip_prefix(['v', 'V'])
        .filter(|s| s.starts_with(|c: char| c.is_ascii_digit()))
        .unwrap_or(version);
    (version.to_owned(), date.trim().to_owned())
}

fn remove_links(text: &str) -> String {
    let mut output = String::new();
    let mut remaining = text;
    while let Some((before, after)) = remaining.split_once('[') {
        output.push_str(before);
        if let Some((label, tail)) = after.split_once("](")
            && let Some((_url, rest)) = tail.split_once(')')
        {
            output.push_str(label);
            remaining = rest;
        } else {
            output.push('[');
            remaining = after;
        }
    }
    output.push_str(remaining);
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn releases_preserve_sections_and_fenced_fake_headings() {
        let releases = parse(
            "# News\n## [v2.0](https://example.test) - 2026-11-15\n\n### Added\ntext\n```md\n## fake\n```\n\n## [1.0] (yesterday)\nolder",
        );
        assert_eq!(releases.len(), 2);
        let first = releases.first().unwrap();
        assert_eq!(first.version(), "2.0");
        assert_eq!(first.date(), "2026-11-15");
        assert!(
            first
                .markdown()
                .contains("### Added\ntext\n```md\n## fake\n```")
        );
        let last = releases.last().unwrap();
        assert_eq!(last.version(), "1.0");
        assert_eq!(last.date(), "yesterday");
    }

    #[test]
    fn indentation_fence_length_and_reveal_budget_follow_reference() {
        assert_eq!(
            parse("    ## indented\n## v1\n~~~~\n## inside\n~~~\n## still inside\n~~~~\n## v0")
                .len(),
            2
        );
        let mut revealed = Revealed::default();
        assert_eq!(revealed.count(), LATEST_ONLY);
        revealed.settled();
        assert_eq!(revealed.count(), FIRST_PAGE);
        revealed.show_more();
        assert_eq!(revealed.count(), FIRST_PAGE.saturating_add(PAGE_SIZE));
    }
}
