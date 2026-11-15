//! Invitation field and its explicitly revealed credential. The render
//! path and clipboard both ask the pure model at the current time.

use crate::shell::Shell;
use chrono::Utc;
use std::time::Duration;
use vessel_app_kit::ui::{
    AnyElement, ClipboardItem, Context, FluentBuilder as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, button, outline_button, settings};
use vessel_app_kit::{breakpoint::TOUCH_TARGET, theme::spacing};
use vessel_app_link::devices::Ask;
use vessel_app_view::devices::{ConnectionContext, Inviting, Target, invite_label};

impl Shell {
    fn devices_invite(
        &mut self,
        context: &ConnectionContext,
        window: &Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.devices_press(context, window) {
            return;
        }
        let state = &mut self.areas.settings.devices;
        if state.page.check_context(context).is_err() {
            return;
        }

        if matches!(state.page.inviting, Inviting::Loading(_)) {
            return;
        }
        let label = invite_label(&state.name.text(cx));
        let ticket = match state.page.tickets.next_ticket() {
            Ok(ticket) => ticket,
            Err(why) => {
                state.page.inviting = Inviting::Failed(why);
                return;
            }
        };
        state.copied = None;
        state.page.inviting = Inviting::Loading(ticket);
        if let Err(why) = self.devices_send(context.clone(), ticket, Ask::Invite { label }) {
            let _accepted = self
                .areas
                .settings
                .devices
                .page
                .inviting
                .answer(ticket, Err(why));
        }
    }

    fn devices_reveal(
        &mut self,
        context: &ConnectionContext,
        window: &Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.devices_press(context, window) {
            return;
        }
        let state = &mut self.areas.settings.devices;
        if state.page.check_context(context).is_err() {
            return;
        }
        let Inviting::Ready(pairing) = &mut state.page.inviting else {
            return;
        };
        let now = Utc::now();
        if pairing.visible_link(now).is_some() {
            pairing.hide();
            return;
        }
        state.copied = None;
        let key = match state.page.tickets.next_ticket() {
            Ok(key) => key,
            Err(why) => {
                state.page.inviting = Inviting::Failed(why);
                return;
            }
        };
        pairing.show(now, key);
        let Some(limit) = pairing.display_limit() else {
            return;
        };
        let wait = limit
            .deadline()
            .signed_duration_since(now)
            .to_std()
            .unwrap_or(Duration::ZERO);
        // A one-shot wake-up, not polling. The model checks the deadline
        // itself; an older timer cannot mask a more recently revealed link.
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(wait).await;
            let _gone = this.update(cx, |shell, cx| {
                if let Inviting::Ready(pairing) = &mut shell.areas.settings.devices.page.inviting {
                    pairing.hide_limit(limit);
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn devices_copy(
        &mut self,
        context: &ConnectionContext,
        window: &Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.devices_press(context, window) {
            return;
        }
        let state = &mut self.areas.settings.devices;
        if state.page.check_context(context).is_err() {
            return;
        }
        let Inviting::Ready(pairing) = &mut state.page.inviting else {
            return;
        };
        let Some(link) = pairing.visible_link(Utc::now()) else {
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(link.expose().to_owned()));
        pairing.hide();
        state.copied = Some(self.t.lang.devices().copied);
    }
}

pub(super) fn start(shell: &Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    shell
        .areas
        .settings
        .devices
        .name
        .set_text(shell.t.lang.devices().phone, window, cx);
    cx.observe_window_activation(window, |shell, window, cx| {
        if !window.is_window_active() {
            shell.areas.settings.devices.page.inviting.hide();
            cx.notify();
        }
    })
    .detach();
    cx.observe_window_visibility(window, |shell, _visibility, _window, cx| {
        shell.areas.settings.devices.page.inviting.hide();
        cx.notify();
    })
    .detach();
}

pub(super) fn render(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.t.lang.devices();
    let state = &shell.areas.settings.devices;
    let Target::Online(machine) = &state.page.target else {
        return div().into_any_element();
    };
    let mut card = settings::card(tokens).child(settings::row_at(
        words.name,
        Some(words.name_hint(machine.as_str()).into()),
        None,
        Some(form(shell, tokens, compact, cx)),
        true,
        compact,
        tokens,
    ));
    if let Inviting::Failed(why) = state.page.inviting {
        card = card.child(
            div().p(px(spacing::S4)).child(vessel_app_kit::alert::alert(
                vessel_app_kit::alert::Content {
                    title: None,
                    description: Some(
                        div()
                            .child(words.invite_problem(why, shell.words))
                            .into_any_element(),
                    ),
                    action: None,
                },
                crate::badge::alert_variant(why.notice()),
                vessel_app_kit::alert::Size::Small,
                tokens,
            )),
        );
    }
    if let Inviting::Ready(pairing) = &state.page.inviting {
        card = card
            .child(settings::row_at(
                words.expires(vessel_app_view::devices::time::Remaining::of(
                    pairing.expires(),
                    Utc::now(),
                )),
                Some(words.verify.into()),
                None,
                None,
                false,
                compact,
                tokens,
            ))
            .child(
                div()
                    .px(px(spacing::S4))
                    .type_(Type::Mono)
                    .whitespace_normal()
                    .child(pairing.fingerprint().to_owned()),
            )
            .child(
                div()
                    .p(px(spacing::S4))
                    .child(credential(shell, pairing, tokens, compact, cx)),
            );
    }
    settings::section(words.pair_another, card, tokens).into_any_element()
}

fn form(shell: &Shell, tokens: &Tokens, compact: bool, cx: &mut Context<'_, Shell>) -> AnyElement {
    let words = shell.t.lang.devices();
    let state = &shell.areas.settings.devices;
    let Some(context) = state.page.context().cloned() else {
        return div().into_any_element();
    };
    let busy = matches!(state.page.inviting, Inviting::Loading(_));
    div()
        .flex()
        .flex_col()
        .gap(px(spacing::S3))
        .when(compact, Styled::w_full)
        .when(!compact, |form| form.w(px(vessel_app_kit::select::WIDTH)))
        .max_w_full()
        .when(busy, |form| {
            form.child(super::rows::busy(
                "devices-inviting",
                words.creating,
                state.page.inviting.variant(),
                tokens,
            ))
        })
        .child(state.name.element(tokens, compact, cx))
        .when(!busy, |form| {
            form.child(
                button(
                    format!("devices-new-link-{}", context.epoch()),
                    words.new_link,
                    true,
                    tokens,
                )
                .when(compact, |button| button.min_h(px(TOUCH_TARGET)))
                .on_click(cx.listener(move |shell, _event, window, cx| {
                    shell.devices_invite(&context, window, cx);
                    cx.notify();
                })),
            )
        })
        .into_any_element()
}

fn credential(
    shell: &Shell,
    pairing: &vessel_app_view::devices::Pairing,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> vessel_app_kit::ui::Div {
    let words = shell.t.lang.devices();
    let now = Utc::now();
    let link = pairing.visible_link(now);
    let mut controls = div()
        .flex()
        .flex_col()
        .gap(px(spacing::S3))
        .w_full()
        .min_w_0();
    match pairing.matrix(now) {
        Ok(Some(matrix)) => {
            controls = controls.child(vessel_app_kit::qr::qr(
                matrix.side(),
                matrix.dark(),
                px(vessel_app_kit::qr::PAIRING_SIZE),
            ));
        }
        Ok(None) => {}
        Err(_too_large) => {
            controls = controls.child(div().child(words.qr_unavailable));
        }
    }
    controls = match link {
        Some(link) => controls.child(
            div()
                .type_(Type::Mono)
                .whitespace_normal()
                .child(link.expose().to_owned()),
        ),
        None => controls.child(
            div()
                .type_(Type::Body)
                .text_color(tokens.neutral.foreground_muted)
                .child(words.hidden),
        ),
    };
    if now < pairing.expires() {
        controls = controls.child(actions(shell, link.is_some(), tokens, compact, cx));
    }
    controls = controls.child(vessel_app_kit::alert::alert(
        vessel_app_kit::alert::Content {
            title: None,
            description: Some(div().child(words.warning).into_any_element()),
            action: None,
        },
        crate::badge::alert_variant(vessel_app_view::devices::PAIRING_WARNING),
        vessel_app_kit::alert::Size::Small,
        tokens,
    ));
    if let Some(note) = shell.areas.settings.devices.copied {
        controls = controls.child(div().type_(Type::Meta).child(note));
    }
    controls
}

fn actions(
    shell: &Shell,
    showing: bool,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> vessel_app_kit::ui::Div {
    let words = shell.t.lang.devices();
    let Some(context) = shell.areas.settings.devices.page.context().cloned() else {
        return div();
    };
    let reveal_context = context.clone();
    div()
        .flex()
        .flex_wrap()
        .gap(px(spacing::S2))
        .child(
            outline_button(
                format!("devices-reveal-{}", context.epoch()),
                if showing { words.hide } else { words.show },
                true,
                tokens,
            )
            .when(compact, |button| button.min_h(px(TOUCH_TARGET)))
            .on_click(cx.listener(move |shell, _event, window, cx| {
                shell.devices_reveal(&reveal_context, window, cx);
                cx.notify();
            })),
        )
        .when(showing, |row| {
            row.child(
                outline_button(
                    format!("devices-copy-{}", context.epoch()),
                    words.copy,
                    true,
                    tokens,
                )
                .when(compact, |button| button.min_h(px(TOUCH_TARGET)))
                .on_click(cx.listener(move |shell, _event, window, cx| {
                    shell.devices_copy(&context, window, cx);
                    cx.notify();
                })),
            )
        })
}

/// A redraw clock holds only a public expiry, never the invitation. Each
/// wake checks that the page still has that expiry; navigation or a new
/// request ends it. It cannot reveal, copy or mask any credential.
pub(super) fn clock(expires: chrono::DateTime<Utc>, cx: &mut Context<'_, Shell>) {
    cx.spawn(async move |this, cx| {
        while let Some(wait) =
            vessel_app_view::devices::time::Remaining::next_change(expires, Utc::now())
        {
            cx.background_executor().timer(wait).await;
            let keep = this.update(cx, |shell, cx| {
                let current = matches!(&shell.areas.settings.devices.page.inviting,
                    Inviting::Ready(pairing) if pairing.expires() == expires);
                if current {
                    cx.notify();
                }
                current
            });
            if !matches!(keep, Ok(true)) {
                break;
            }
        }
    })
    .detach();
}
