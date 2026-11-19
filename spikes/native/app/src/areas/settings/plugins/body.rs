//! Settings › Plugins' page, as Paseo's `HostPluginsPage`: its offline and
//! too-old notices (a medium warning alone), or its section: the trust
//! notice (a small warning), the "Enable plugins" card, the install card
//! (16 around, 12 between the field and its full-width button), the note
//! under it (a small success or error alert) and the installations
//! (`rows.rs`), 12 apart (`SettingsSection`'s content gap).

use vessel_app_kit::alert::{Content, Size, Variant, alert};
use vessel_app_kit::settings;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, InteractiveElement as _, IntoElement as _, ParentElement as _,
    SharedString, StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, ControlSizing, Tokens, Type, TypeExt as _, button_with_kind_at,
};
use vessel_app_view::Reach;
use vessel_app_view::plugins::{
    Feedback, Listed, Page, PageState, Pending, Reached, change_words, install_source, page_state,
    problem_words, speaks,
};

use crate::shell::Shell;

/// What the page reads before its first list: loading.
const UNREAD: Listed = Listed {
    loading: true,
    error: false,
    count: 0,
};

/// The Plugins page for the chosen machine.
pub(super) fn render(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.words.plugins();
    let machine = super::super::choices::chosen_machine(shell);
    let reached = Reached {
        connected: machine.is_some_and(|machine| machine.reach == Reach::Online),
        supported: machine.is_some_and(|machine| speaks(machine.protocol)),
    };
    let page = shell.areas.settings.plugins.page();
    let state = if reached.connected && reached.supported {
        page.map_or(PageState::Loading, Page::state)
    } else {
        page_state(reached, UNREAD)
    };
    match state {
        PageState::Offline => notice(
            words.offline_title(),
            Some(words.offline_description()),
            tokens,
        ),
        PageState::Unsupported => notice(words.update_title(), None, tokens),
        PageState::Loading | PageState::Error | PageState::Empty | PageState::Ready => {
            let content = div()
                .flex()
                .flex_col()
                .gap(px(spacing::S3))
                .child(alert(
                    Content {
                        title: Some(SharedString::new_static(words.trusted_title())),
                        description: Some(
                            div()
                                .child(SharedString::new_static(words.trusted_description()))
                                .into_any_element(),
                        ),
                        action: None,
                    },
                    Variant::Warning,
                    Size::Small,
                    tokens,
                ))
                .child(switch_card(shell, page, tokens, compact, cx))
                .child(install_card(shell, page, tokens, compact, cx))
                .children(feedback(shell, page, tokens))
                .child(super::rows::catalog(
                    shell, page, state, tokens, compact, cx,
                ))
                .children(super::rows::menu(shell, page, tokens, compact, cx));
            settings::section(SharedString::new_static(words.title()), content, tokens)
                .into_any_element()
        }
    }
}

/// Paseo's page-wide warning for a machine it cannot manage.
fn notice(title: &'static str, description: Option<&'static str>, tokens: &Tokens) -> AnyElement {
    alert(
        Content {
            title: Some(SharedString::new_static(title)),
            description: description.map(|description| {
                div()
                    .child(SharedString::new_static(description))
                    .into_any_element()
            }),
            action: None,
        },
        Variant::Warning,
        Size::Medium,
        tokens,
    )
    .into_any_element()
}

/// "Enable plugins": the machine's switch, waiting until it is read and
/// while a change is made.
fn switch_card(
    shell: &Shell,
    page: Option<&Page>,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.words.plugins();
    let on = page.and_then(Page::enabled) == Some(true);
    let ready = page.is_some_and(|page| page.enabled().is_some() && !page.busy());
    let title = SharedString::new_static(words.global_title());
    let mut control = settings::switch(
        "plugins-global",
        on,
        ready,
        compact,
        shell.areas.settings.plugins.turns,
        tokens,
    )
    .role(gpui::Role::CheckBox)
    .aria_toggled(if on {
        gpui::Toggled::True
    } else {
        gpui::Toggled::False
    })
    .aria_label(title.clone());
    if ready {
        control = control.on_click(cx.listener(move |shell, _event, _window, cx| {
            shell.plugins_toggle(!on, cx);
        }));
    }
    settings::card(tokens).child(settings::row_at(
        title,
        Some(SharedString::new_static(words.global_hint())),
        None,
        Some(control.into_any_element()),
        true,
        compact,
        tokens,
    ))
}

/// The install card: "Plugin source" over its field, then "Install
/// plugin", which waits for a source and for any change in flight.
fn install_card(
    shell: &Shell,
    page: Option<&Page>,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.words.plugins();
    let field = &shell.areas.settings.plugins.field;
    let busy = page.is_none_or(Page::busy);
    let ready = !busy && install_source(&field.text(cx)).is_some();
    let installing = page.and_then(Page::pending) == Some(&Pending::Install);
    let label = if installing {
        words.installing()
    } else {
        words.install()
    };
    let sizing = if compact {
        ControlSizing::Touch
    } else {
        ControlSizing::Pointer
    };
    let mut button = button_with_kind_at(
        "plugins-install",
        SharedString::new_static(label),
        ButtonKind::Secondary,
        if ready {
            ButtonState::Enabled
        } else {
            ButtonState::Disabled
        },
        sizing,
        tokens,
    )
    .w_full();
    if ready {
        button = button.on_click(cx.listener(|shell, _event, _window, cx| {
            shell.plugins_install(cx);
        }));
    }
    // Paseo's `Field`: its label 14, muted, 8 above the input.
    let labelled = div()
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .child(
            div()
                .type_(Type::Body)
                .text_color(tokens.neutral.foreground_muted)
                .child(SharedString::new_static(words.source_label())),
        )
        .child(field.element(tokens, compact, cx));
    settings::card(tokens)
        .p(px(spacing::S4))
        .gap(px(spacing::S3))
        .child(labelled)
        .child(button)
}

/// What the last change said (Paseo's feedback alert).
fn feedback(shell: &Shell, page: Option<&Page>, tokens: &Tokens) -> Option<AnyElement> {
    let words = shell.words.plugins();
    let (variant, said) = match page?.feedback()? {
        Feedback::Done(change) => (Variant::Success, change_words(words, change)),
        Feedback::Failed(problem) => (Variant::Error, problem_words(words, problem)),
    };
    Some(
        alert(
            Content {
                title: Some(said.into()),
                description: None,
                action: None,
            },
            variant,
            Size::Small,
            tokens,
        )
        .into_any_element(),
    )
}
