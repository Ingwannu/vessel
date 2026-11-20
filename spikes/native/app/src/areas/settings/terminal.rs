//! Settings > Terminal: Paseo's `screens/settings/terminal/terminal-section.tsx`,
//! one row, "Terminal scrollback" (P3).
//!
//! [Decision Log]
//! - Purpose: the person sets how many lines a terminal keeps above its
//!   screen, 0 through 1,000,000 (Paseo's `terminalScrollbackLines`,
//!   default 10,000), and open terminals take it at once (Paseo's desktop
//!   sets `terminal.options.scrollback` live).
//! - Choice: a one-line number field that commits on Enter or when it loses
//!   the keyboard: its digits are read, clamped into range and stored; the
//!   field then shows the stored count. Nothing typed (no digits) keeps the
//!   count it had. While the field does not have the keyboard it shows the
//!   stored count.
//! - Stated difference: Paseo drops a non-digit as it is typed; here it is
//!   dropped when the field commits (the field's change callback has no
//!   window to rewrite the text in).

use gpui::Subscription;
use vessel_app_kit::ui::{AnyElement, Context, IntoElement as _, ParentElement as _, Styled as _, Window, div, px};
use vessel_app_kit::{TextField, Tokens, settings};
use vessel_app_view::prefs::PreferenceChange;
use vessel_app_view::terminal::ScrollbackLines;

use crate::shell::Shell;

/// The number field's width.
const FIELD_WIDTH: f32 = 120.0;

/// The page's state: its field and the commit when the field loses the
/// keyboard.
pub(crate) struct State {
    field: TextField,
    _blurred: Subscription,
}

impl State {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        let field = TextField::new(
            "",
            |shell: &mut Shell, text, window, cx| shell.scrollback_commit(&text, window, cx),
            window,
            cx,
        );
        let handle = field.focus_handle(cx);
        let blurred = cx.on_blur(&handle, window, |shell, window, cx| {
            let text = shell.areas.settings.terminal.field.text(cx);
            shell.scrollback_commit(&text, window, cx);
        });
        Self {
            field,
            _blurred: blurred,
        }
    }
}

/// The count a typed text asks for: its digits, clamped; `None` for none.
pub(crate) fn asked(text: &str) -> Option<ScrollbackLines> {
    let digits: String = text.chars().filter(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    // More digits than a u64 holds is past the maximum anyway.
    let lines = digits.parse::<u64>().unwrap_or(u64::MAX);
    Some(ScrollbackLines::clamped(lines))
}

impl Shell {
    /// Enter, or the field lost the keyboard: store the count it asks for
    /// and show the stored count.
    fn scrollback_commit(&mut self, text: &str, window: &mut Window, cx: &mut Context<'_, Self>) {
        let kept = self.prefs.settings.terminal.scrollback;
        if let Some(lines) = asked(text)
            && lines != kept
        {
            self.prefer(&PreferenceChange::TerminalScrollback(lines), window, cx);
        }
        let shown = self.prefs.settings.terminal.scrollback.get().to_string();
        let field = &self.areas.settings.terminal.field;
        if field.text(cx) != shown {
            field.set_text(&shown, window, cx);
        }
    }
}

/// The page.
pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.words.terminal_tabs();
    let field = &shell.areas.settings.terminal.field;
    let stored = shell.prefs.settings.terminal.scrollback.get().to_string();
    if !field.focus_handle(cx).is_focused(window) && field.text(cx) != stored {
        field.set_text(&stored, window, cx);
    }
    field.set_accessible_label(words.scrollback_label(), cx);
    let control = div()
        .w(px(FIELD_WIDTH))
        .child(field.input(tokens, cx))
        .into_any_element();
    settings::card(tokens)
        .child(settings::row_at(
            words.scrollback(),
            Some(words.scrollback_hint().into()),
            None,
            Some(control),
            false,
            compact,
            tokens,
        ))
        .into_any_element()
}
