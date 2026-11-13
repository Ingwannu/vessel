//! Links in agent output (Markdown in the timeline, OSC 8 hyperlinks in
//! the terminal) are untrusted: one policy for both (the page's
//! `data/links.ts`, from its security review). Only absolute http(s)
//! URLs are links; anything else (`javascript:`, `mailto:`, `//host`, a
//! path into the app) is not. The app opens one only after a confirm that
//! shows the whole URL, in the system's browser.

/// Whether `href` may become a link: an absolute http(s) URL with a host,
/// nothing before its scheme, no space or control character in it.
#[must_use]
pub fn is_web_url(href: &str) -> bool {
    let lower = href
        .get(..8)
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    let rest = if lower.starts_with("https://") {
        href.get(8..)
    } else if lower.starts_with("http://") {
        href.get(7..)
    } else {
        None
    };
    let Some(rest) = rest else {
        return false;
    };
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    !host.is_empty()
        && !host.chars().any(char::is_whitespace)
        && !href.chars().any(char::is_control)
}
