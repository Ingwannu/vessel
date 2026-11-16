//! Language, after Paseo's settings-screen.tsx `GeneralSection`.
//!
//! [Decision Log]
//! - Purpose: a stored choice applies at once; a checking override still wins.
//! - Constraint: E2 owns General's sending/storage rows and their shared list.
//! - Choice: this row is the first of General's card, under Paseo's "General"
//!   heading (General draws the card and the heading); the kit's existing
//!   select and combobox provide its own picker state. System first, then
//!   native names.
//! - Why: one isolated registration keeps simultaneous settings work separate.
//! - Cost: one small picker state; no second control implementation or preview.

use crate::shell::Shell;
use std::rc::Rc;
use vessel_app_kit::select::Anchor;
use vessel_app_kit::ui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, SharedString,
    StatefulInteractiveElement as _, Window,
};
use vessel_app_kit::{ComboHandlers, ComboRow, ComboboxState, Tokens, combobox, select, settings};
use vessel_app_view::localization::LanguagePreference;
use vessel_app_view::prefs::PreferenceChange;

// Paseo's language menu explicitly uses 300, wider than its ordinary selects.
const WIDTH: f32 = 300.0;

pub(crate) struct State {
    list: ComboboxState,
    anchor: Anchor,
}

impl State {
    pub(crate) fn close(&mut self) {
        self.list.close();
    }

    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            list: ComboboxState::new(window, cx),
            anchor: Anchor::default(),
        }
    }
}

/// The Language row.
pub(crate) fn row(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.words.meta();
    let state = &shell.areas.settings.language;
    let value = shell.prefs.settings.language;
    let trigger = select::trigger(
        "settings-language",
        SharedString::new_static(value.label(shell.words)),
        &state.anchor,
        state.list.is_open(),
        true,
        compact,
        tokens,
    )
    .debug_selector(|| "settings-language-trigger".to_owned())
    .on_click(cx.listener(|shell, _event, window, cx| {
        shell.areas.settings.list.close();
        let state = &mut shell.areas.settings.language;
        let bounds = state
            .anchor
            .get()
            .map(|bounds| select::end_aligned(bounds, WIDTH));
        state.list.set_anchor(bounds);
        state.list.open(None, window, cx);
        cx.notify();
    }));
    // The first row of General's card (Paseo's `GeneralSection`).
    settings::row_at(
        SharedString::new_static(words.language()),
        Some(SharedString::new_static(words.language_note())),
        None,
        Some(trigger.into_any_element()),
        true,
        compact,
        tokens,
    )
    .into_any_element()
}

pub(crate) fn picker(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.settings.language;
    if !state.list.is_open() {
        return None;
    }
    let rows: Vec<_> = LanguagePreference::choices()
        .map(|value| ComboRow {
            checked: value == shell.prefs.settings.language,
            ..ComboRow::new(SharedString::new_static(value.label(shell.words)))
        })
        .collect();
    let handlers = ComboHandlers {
        state: |shell: &mut Shell| &mut shell.areas.settings.language.list,
        chose: Rc::new(|shell: &mut Shell, index, window, cx| {
            shell.areas.settings.language.list.close();
            if let Some(language) = LanguagePreference::choices().nth(index) {
                shell.prefer(&PreferenceChange::Language(language), window, cx);
            }
        }),
        back: None,
    };
    combobox(
        "settings-language-list",
        &state.list,
        &select::header(),
        &rows,
        &handlers,
        WIDTH,
        tokens,
        window,
        cx,
    )
}
