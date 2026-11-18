//! Settings rows use canonical binding identities and the engine's snapshot.
//! One recorder can be active; navigating away cancels it. All overlapping
//! bindings are reviewed before one atomic preference transaction is applied.

mod editing;
mod row;

use vessel_app_kit::dialog::{DialogContent, DialogSize, DialogState, dialog};
use vessel_app_kit::menu::{MenuHandlers, MenuItem, MenuState, menu};
use vessel_app_kit::ui::{
    AnyElement, App, Context, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div,
};
use vessel_app_kit::{ButtonKind, ButtonState, Tokens, Type, TypeExt as _, button_with_kind};
use vessel_app_view::prefs::ActionId;
use vessel_app_view::shortcuts::sequence_capture::SequenceCapture;
use vessel_app_view::shortcuts::{
    Platform, ShortcutGroup, default_context, group_label, visible_definitions,
};

use crate::shell::Shell;

pub(crate) struct ShortcutsPage {
    pub(super) capture: Option<SequenceCapture>,
    menu: MenuState,
    menu_for: Option<ActionId>,
    asking: Option<Asking>,
    dialog: DialogState,
}

struct Asking {
    capture: SequenceCapture,
    conflicts: Vec<ActionId>,
    /// Complete translated sentences prepared at the user event, not on render.
    sentences: Vec<String>,
}

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
    pub(super) fn leave(&mut self) {
        self.capture = None;
        self.menu.close();
        self.menu_for = None;
        self.asking = None;
    }
    pub(crate) const fn recording(&self) -> bool {
        self.capture.is_some()
    }
}

fn row_acts(shell: &Shell, id: &ActionId) -> Vec<(RowAct, MenuItem)> {
    let words = shell.words.shortcuts();
    let context = default_context(Platform::native());
    let Ok(bindings) =
        vessel_app_view::shortcuts::effective_bindings(&shell.prefs.settings.shortcuts, context)
    else {
        return Vec::new();
    };
    let bound = bindings.iter().any(|binding| &binding.id == id);
    let mut acts = vec![(
        RowAct::Record,
        MenuItem::new(if bound { words.rebind() } else { words.bind() }),
    )];
    if bound {
        acts.push((RowAct::Clear, MenuItem::new(words.clear())));
    }
    let migrated =
        vessel_app_view::shortcuts::migrated_overrides(&shell.prefs.settings.shortcuts, context);
    let has_default = vessel_app_view::shortcuts::definitions().is_ok_and(|definitions| {
        definitions
            .iter()
            .any(|definition| &definition.id == id && definition.default_keys.is_some())
    });
    if migrated.contains_key(id) && has_default {
        acts.push((RowAct::Reset, MenuItem::new(words.reset())));
    }
    acts
}

pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    _phone: bool,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.words.shortcuts();
    // Section renderer keeps the shared Settings signature; its list/detail
    // form does not decide keyboard availability (`available`, 600; the
    // phone form switches at the same width since TRACKER 798).
    if !super::available(window) {
        return div()
            .type_(Type::Body)
            .text_color(tokens.neutral.foreground_muted)
            .child(words.unavailable())
            .into_any_element();
    }
    let Ok(definitions) = visible_definitions(default_context(Platform::native())) else {
        return div().into_any_element();
    };
    let Ok(bindings) = vessel_app_view::shortcuts::effective_bindings(
        &shell.prefs.settings.shortcuts,
        default_context(Platform::native()),
    ) else {
        return div().into_any_element();
    };
    let mut trailing = (!shell.prefs.settings.shortcuts.is_empty()).then(|| {
        button_with_kind(
            "shortcuts-reset-all",
            words.reset_all(),
            ButtonKind::Ghost,
            ButtonState::Enabled,
            tokens,
        )
        .on_click(cx.listener(|shell, _event, window, cx| {
            shell.areas.shortcuts.page.leave();
            shell.prefer(
                &vessel_app_view::prefs::PreferenceChange::ResetShortcuts,
                window,
                cx,
            );
        }))
        .into_any_element()
    });
    let mut page = div().flex().flex_col();
    for group in ShortcutGroup::ALL {
        let mut card = vessel_app_kit::settings::card(tokens);
        for (at, definition) in definitions
            .iter()
            .filter(|definition| definition.action.group() == group)
            .enumerate()
        {
            card = card.child(row::row(shell, definition, &bindings, at == 0, tokens, cx));
        }
        page = page.child(vessel_app_kit::settings::section_with_trailing(
            group_label(group, shell.words),
            card,
            trailing.take(),
            tokens,
        ));
    }
    let state = &shell.areas.shortcuts.page;
    let items = state
        .menu_for
        .as_ref()
        .map(|id| row_acts(shell, id))
        .unwrap_or_default();
    let acts: Vec<_> = items.iter().map(|(act, _)| *act).collect();
    let items: Vec<_> = items.into_iter().map(|(_, item)| item).collect();
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
        &state.menu,
        &items,
        &handlers,
        tokens,
        false,
        cx,
    ))
    .children(asking(shell, tokens, window, cx))
    .into_any_element()
}

fn asking(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let page = &shell.areas.shortcuts.page;
    let asking = page.asking.as_ref()?;
    let words = shell.words.shortcuts();
    let content = DialogContent {
        title: words.in_use().into(),
        body: div()
            .flex()
            .flex_col()
            .type_(Type::Body)
            .text_color(tokens.neutral.foreground_muted)
            .children(
                asking
                    .sentences
                    .iter()
                    .map(|sentence| div().child(sentence.clone())),
            )
            .into_any_element(),
        footer: vec![
            button_with_kind(
                "shortcut-reassign-cancel",
                words.cancel(),
                ButtonKind::Secondary,
                ButtonState::Enabled,
                tokens,
            )
            .on_click(cx.listener(|shell, _event, window, cx| shell.shortcut_unask(window, cx)))
            .into_any_element(),
            button_with_kind(
                "shortcut-reassign",
                words.reassign(),
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
        false,
        Shell::shortcut_unask,
        cx,
    ))
}
