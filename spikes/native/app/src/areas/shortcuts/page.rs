//! Settings › Shortcuts, as Paseo's page
//! (`screens/settings/keyboard-shortcuts-section.tsx`): one section per
//! group, the first with "Reset all" at its head while any key was
//! changed; a row is the action's name at the left and, at the right, its
//! keys (or "Not set") and a 32-point menu slot (Bind or Rebind, Clear,
//! Reset to default). Rebind records in the row: the row fills with
//! `surface2`, the keys' place says "Press shortcut…" and then shows the
//! key held, with Done and Cancel. What a key does is the view's
//! (`view::shortcuts::Capture`): the page's refusals show under the keys,
//! and a key another action has is moved only after the page's question.
//!
//! [Decision Log]
//! - Purpose: the person changes any app-wide key, as Paseo and the page
//!   both allow, from the app.
//! - Reference: Paseo's row geometry (12 inside, a hairline between rows,
//!   the menu slot kept empty for a fixed row so the keys keep one rail),
//!   its menu items and words; the page's refusals and its "Shortcut
//!   already in use" question (Paseo saves whatever is pressed: one key
//!   held by two actions would silently fire only the first).
//! - Differences, named: Escape cancels a recording (Paseo records it as a
//!   key; the page refuses a key without a modifier anyway); modifiers held
//!   alone are not shown before the key (the toolkit's interceptor hears
//!   keys, not modifier changes); the menu rows have no leading glyphs (the
//!   kit's menu has none yet); the page is shown at a phone's width too
//!   (Paseo hides it without a hardware keyboard, which the app cannot
//!   detect; a phone with a keyboard attached uses it).

use vessel_app_kit::dialog::{DialogContent, DialogSize, DialogState, dialog};
use vessel_app_kit::menu::{MenuHandlers, MenuItem, MenuState, menu};
use vessel_app_kit::ui::{
    AnyElement, App, Context, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div,
};
use vessel_app_kit::{ButtonKind, ButtonState, Tokens, Type, TypeExt as _, button_with_kind};
use vessel_app_view::prefs::{PreferenceChange, ShortcutEdit, ShortcutEdits};
use vessel_app_view::shortcuts::{
    Action, Capture, Chord, Done, Platform, ShortcutGroup, binding_words, bindings_of, clear,
    modified, modified_count, reset,
};

use crate::shell::Shell;

/// The page's own state, kept by the shortcut listener.
pub(crate) struct ShortcutsPage {
    /// The row recording, if one is.
    pub(super) capture: Option<Capture>,
    /// A row's menu.
    menu: MenuState,
    /// Whose menu is open.
    menu_for: Option<Action>,
    /// The question before a key moves from another action.
    asking: Option<Asking>,
    dialog: DialogState,
}

/// A recorded key another action has, waiting for "Reassign".
struct Asking {
    from: Action,
    to: Action,
    /// The key in words ("Ctrl+K").
    words: String,
    edits: Vec<ShortcutEdit>,
}

/// What a row's menu item does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowAct {
    Record,
    Clear,
    Reset,
}

impl ShortcutsPage {
    pub(crate) fn new(cx: &mut App) -> Self {
        Self {
            capture: None,
            menu: MenuState::new(cx),
            menu_for: None,
            asking: None,
            dialog: DialogState::new(cx),
        }
    }

    /// Whether a row records: then no shortcut fires (Paseo's
    /// `setCapturingShortcut`).
    pub(crate) const fn recording(&self) -> bool {
        self.capture.is_some()
    }
}

/// The menu's items for `action`, with what each does (Paseo shows Clear
/// only with a key and Reset only when changed and a default exists).
fn row_acts(shell: &Shell, action: Action) -> Vec<(RowAct, MenuItem)> {
    let words = shell.t.lang.shortcuts();
    let overrides = &shell.prefs.settings.shortcuts;
    let bound = !bindings_of(action, overrides).is_empty();
    let mut acts = vec![(
        RowAct::Record,
        MenuItem::new(if bound { words.rebind } else { words.bind }),
    )];
    if bound {
        acts.push((RowAct::Clear, MenuItem::new(words.clear)));
    }
    if modified(action, overrides) && !action.defaults().is_empty() {
        acts.push((RowAct::Reset, MenuItem::new(words.reset)));
    }
    acts
}

impl Shell {
    /// A key pressed while a row records (the listener hands it here).
    pub(super) fn shortcut_recorded(&mut self, chord: Chord, cx: &mut Context<'_, Self>) {
        let bare = !(chord.primary || chord.ctrl || chord.alt || chord.shift || chord.meta);
        if bare && chord.key == "Escape" {
            self.shortcut_cancel(cx);
            return;
        }
        if let Some(capture) = &mut self.areas.shortcuts.page.capture {
            if bare && chord.key == "Backspace" {
                capture.back();
            } else {
                capture.press(chord);
            }
        }
        cx.notify();
    }

    fn shortcut_menu(
        &mut self,
        action: Action,
        at: vessel_app_kit::ui::Point<vessel_app_kit::ui::Pixels>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let page = &mut self.areas.shortcuts.page;
        page.menu_for = Some(action);
        page.menu.open(at, window, cx);
        cx.notify();
    }

    fn shortcut_act(&mut self, act: RowAct, window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some(action) = self.areas.shortcuts.page.menu_for.take() else {
            return;
        };
        match act {
            RowAct::Record => {
                // Recording another row drops the first (Paseo's one
                // `capturingBindingId`).
                self.areas.shortcuts.page.capture = Capture::start(action);
                window.blur(cx);
            }
            RowAct::Clear => self.shortcut_edits(clear(action), window, cx),
            RowAct::Reset => self.shortcut_edits(reset(action), window, cx),
        }
        cx.notify();
    }

    fn shortcut_done(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let platform = Platform::native();
        let Some(capture) = self.areas.shortcuts.page.capture.clone() else {
            return;
        };
        match capture.done(&self.prefs.settings.shortcuts, platform) {
            Done::Nothing => {}
            Done::Save(edits) => {
                self.areas.shortcuts.page.capture = None;
                self.shortcut_edits(edits, window, cx);
            }
            Done::Ask { from, edits } => {
                let words = capture
                    .chord
                    .as_ref()
                    .map(|chord| {
                        binding_words(
                            &chord_binding(chord),
                            platform,
                            self.t.lang.shortcuts().then,
                        )
                    })
                    .unwrap_or_default();
                let page = &mut self.areas.shortcuts.page;
                page.capture = None;
                page.asking = Some(Asking {
                    from,
                    to: capture.action,
                    words,
                    edits,
                });
                page.dialog.focus(window, cx);
            }
        }
        cx.notify();
    }

    fn shortcut_cancel(&mut self, cx: &mut Context<'_, Self>) {
        self.areas.shortcuts.page.capture = None;
        cx.notify();
    }

    fn shortcut_reassign(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        if let Some(asking) = self.areas.shortcuts.page.asking.take() {
            self.shortcut_edits(asking.edits, window, cx);
        }
        cx.notify();
    }

    fn shortcut_unask(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) {
        self.areas.shortcuts.page.asking = None;
        cx.notify();
    }

    fn shortcut_edits(
        &mut self,
        edits: Vec<ShortcutEdit>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if edits.is_empty() {
            return;
        }
        match ShortcutEdits::try_from(edits) {
            Ok(edits) => self.prefer(&PreferenceChange::Shortcuts { edits }, window, cx),
            Err(error) => tracing::warn!(%error, "too many shortcut edits at once"),
        }
    }
}

/// One chord as a binding, for its words.
fn chord_binding(chord: &Chord) -> vessel_app_view::shortcuts::Binding {
    vessel_app_view::shortcuts::Binding {
        first: chord.clone(),
        then: None,
    }
}

/// The page.
pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    phone: bool,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.t.lang.shortcuts();
    let overrides = &shell.prefs.settings.shortcuts;
    let reset_all = (modified_count(overrides) > 0).then(|| {
        button_with_kind(
            "shortcuts-reset-all",
            words.reset_all,
            ButtonKind::Ghost,
            ButtonState::Enabled,
            tokens,
        )
        .on_click(cx.listener(|shell, _event, window, cx| {
            shell.prefer(&PreferenceChange::ResetShortcuts, window, cx);
        }))
        .into_any_element()
    });
    let mut trailing = reset_all;
    let mut page = div().flex().flex_col();
    for group in ShortcutGroup::ALL {
        let mut card = vessel_app_kit::settings::card(tokens);
        for (at, action) in Action::in_group(group).enumerate() {
            card = card.child(row::row(shell, action, at == 0, tokens, cx));
        }
        page = page.child(vessel_app_kit::settings::section_with_trailing(
            words.group(group),
            card,
            trailing.take(),
            tokens,
        ));
    }
    let page_state = &shell.areas.shortcuts.page;
    let menu_items: Vec<(RowAct, MenuItem)> = page_state
        .menu_for
        .map(|action| row_acts(shell, action))
        .unwrap_or_default();
    let acts: Vec<RowAct> = menu_items.iter().map(|(act, _)| *act).collect();
    let items: Vec<MenuItem> = menu_items.into_iter().map(|(_, item)| item).collect();
    let handlers = MenuHandlers {
        state: |shell: &mut Shell| &mut shell.areas.shortcuts.page.menu,
        chose: std::rc::Rc::new(move |shell: &mut Shell, index, window, cx| {
            if let Some(act) = acts.get(index).copied() {
                shell.shortcut_act(act, window, cx);
            }
        }),
    };
    page.children(menu(
        "shortcut-row-menu",
        &page_state.menu,
        &items,
        &handlers,
        tokens,
        phone,
        cx,
    ))
    .children(asking(shell, tokens, phone, window, cx))
    .into_any_element()
}

/// The question before a key moves (the page's dialog).
fn asking(
    shell: &Shell,
    tokens: &Tokens,
    phone: bool,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let page = &shell.areas.shortcuts.page;
    let asking = page.asking.as_ref()?;
    let words = shell.t.lang.shortcuts();
    let sentence = words.clash(
        vessel_app_view::shortcuts::Clash::Taken(asking.from),
        &asking.words,
        asking.to,
    );
    let content = DialogContent {
        title: words.in_use.into(),
        body: div()
            .type_(Type::Body)
            .text_color(tokens.neutral.foreground_muted)
            .child(sentence)
            .into_any_element(),
        footer: vec![
            button_with_kind(
                "shortcut-reassign-cancel",
                words.cancel,
                ButtonKind::Secondary,
                ButtonState::Enabled,
                tokens,
            )
            .on_click(cx.listener(|shell, _event, window, cx| shell.shortcut_unask(window, cx)))
            .into_any_element(),
            button_with_kind(
                "shortcut-reassign",
                words.reassign,
                ButtonKind::Primary,
                ButtonState::Enabled,
                tokens,
            )
            .on_click(cx.listener(|shell, _event, window, cx| shell.shortcut_reassign(window, cx)))
            .into_any_element(),
        ],
        size: DialogSize::Normal,
    };
    Some(dialog(
        &page.dialog,
        content,
        tokens,
        window,
        phone,
        Shell::shortcut_unask,
        cx,
    ))
}

mod row;
