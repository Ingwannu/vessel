//! The first paint: the appearance the page keeps, written into the index
//! it is served, so the page is drawn right before its script runs.
//!
//! [Decision Log]
//! - Purpose: the page's origin is new on every launch (ADR 0017), so a
//!   theme kept in the browser is gone each time, and the content
//!   security policy allows no inline script to set one before the
//!   stylesheet paints. A dark theme read over the socket after the first
//!   paint flashes light first.
//! - Choice: the server writes `data-theme` (absent for `system`, which
//!   the stylesheet follows by itself), `data-ui-size`, `data-code-size`
//!   and `data-content-width` on the index's `<html>`, and the stylesheet
//!   has a rule for each value: nothing inline. The values come from the
//!   preferences task's current state (`keep`, memory: no file is read per
//!   request), and each is from a closed set or a bounded integer written
//!   by this module, never text from the file or the page.
//! - Where: right after the one `<html lang="en"` of the built index (a
//!   test checks there is exactly one when the real bundle is embedded).
//!   The placeholder page, served when no bundle was built, is served as
//!   it is.
//! - Exposure: the index is served before a tab proves its key, to
//!   anything on this machine that can reach the port (by the launch's
//!   address or a literal loopback name) and to whatever a port forward
//!   exposes it to. These values say how the page looks and nothing else.
//! - Alternatives: a cookie (per origin, so gone with it); an inline
//!   script (the policy forbids it); a separate stylesheet generated per
//!   launch (one more request before the first paint, and a cache rule of
//!   its own).

use std::borrow::Cow;
use std::fmt::Write as _;

use crate::wire::{Appearance, CodeSize, ContentWidth, InterfaceSize, Theme};

/// Where the attributes go: the opening of the built index's `<html>`.
const MARK: &[u8] = b"<html lang=\"en\"";

/// What the index is painted with: the appearance, reduced to the values
/// the attributes take.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Paint {
    theme: Theme,
    interface_size: InterfaceSize,
    code_size: CodeSize,
    content_width: ContentWidth,
}

impl From<&Appearance> for Paint {
    fn from(appearance: &Appearance) -> Self {
        Self {
            theme: appearance.theme,
            interface_size: appearance.interface_size,
            code_size: appearance.code_size,
            content_width: appearance.content_width,
        }
    }
}

impl Paint {
    /// The attributes, each with a space before it.
    fn attributes(self) -> String {
        let mut text = String::with_capacity(96);
        match self.theme {
            Theme::System => {}
            Theme::Light => text.push_str(" data-theme=\"light\""),
            Theme::Dark => text.push_str(" data-theme=\"dark\""),
        }
        let width = match self.content_width {
            ContentWidth::Comfortable => "comfortable",
            ContentWidth::Wide => "wide",
            ContentWidth::Full => "full",
        };
        let _written = write!(
            text,
            " data-ui-size=\"{}\" data-code-size=\"{}\" data-content-width=\"{width}\"",
            self.interface_size.get(),
            self.code_size.get(),
        );
        text
    }
}

/// The index to serve: the built one painted, the placeholder as it is.
pub(crate) fn index(bytes: &'static [u8], paint: Paint) -> Cow<'static, [u8]> {
    if crate::files::bundled() {
        splice(bytes, paint)
    } else {
        Cow::Borrowed(bytes)
    }
}

/// `bytes` with the attributes after the first [`MARK`]; unchanged when
/// there is none.
fn splice(bytes: &'static [u8], paint: Paint) -> Cow<'static, [u8]> {
    let Some(end) = bytes
        .windows(MARK.len())
        .position(|window| window == MARK)
        .and_then(|at| at.checked_add(MARK.len()))
    else {
        return Cow::Borrowed(bytes);
    };
    let Some((head, tail)) = bytes.split_at_checked(end) else {
        return Cow::Borrowed(bytes);
    };
    let attributes = paint.attributes();
    let mut painted = Vec::with_capacity(bytes.len().saturating_add(attributes.len()));
    painted.extend_from_slice(head);
    painted.extend_from_slice(attributes.as_bytes());
    painted.extend_from_slice(tail);
    Cow::Owned(painted)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &[u8] = b"<!doctype html>\n<html lang=\"en\">\n<head></head></html>\n";

    #[test]
    fn the_appearance_is_written_on_html() {
        let painted = splice(PAGE, Paint::default());
        assert_eq!(
            String::from_utf8_lossy(&painted),
            "<!doctype html>\n<html lang=\"en\" data-ui-size=\"14\" data-code-size=\"12\" \
             data-content-width=\"comfortable\">\n<head></head></html>\n"
        );
        let dark = Appearance {
            theme: Theme::Dark,
            interface_size: InterfaceSize::try_from(16).unwrap(),
            code_size: CodeSize::try_from(13).unwrap(),
            content_width: ContentWidth::Full,
            ..Appearance::default()
        };
        let painted = splice(PAGE, Paint::from(&dark));
        assert!(
            String::from_utf8_lossy(&painted).contains(
                "<html lang=\"en\" data-theme=\"dark\" data-ui-size=\"16\" data-code-size=\"13\" \
                 data-content-width=\"full\">"
            ),
            "{}",
            String::from_utf8_lossy(&painted)
        );
        // A page without the mark is served as it is.
        let other: &'static [u8] = b"<html><body></body></html>";
        assert!(matches!(splice(other, Paint::default()), Cow::Borrowed(_)));
    }

    /// The built index has exactly one place for the paint; the
    /// placeholder is served unchanged. Which one is embedded depends on
    /// whether `web/dist` was built (CI's web job builds it).
    #[test]
    fn the_embedded_index_has_one_place_for_the_paint() {
        let built = crate::files::find("/index.html").unwrap();
        let marks = built
            .bytes
            .windows(MARK.len())
            .filter(|window| *window == MARK)
            .count();
        let dark = Paint {
            theme: Theme::Dark,
            ..Paint::default()
        };
        if crate::files::bundled() {
            assert_eq!(marks, 1, "the built index needs one `<html lang=\"en\"`");
            assert!(matches!(index(built.bytes, dark), Cow::Owned(_)));
        } else {
            assert_eq!(&*index(built.bytes, dark), built.bytes);
        }
    }
}
