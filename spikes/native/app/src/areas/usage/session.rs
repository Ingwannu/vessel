//! Session quota is tied to launch association, independently from context occupancy.
use super::{
    draw,
    rows::{Density, message, notice},
};
use crate::{routes::Route, shell::Shell};
use vessel_app_kit::ui::{
    AnyElement, Context, FluentBuilder as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, outline_button, theme::spacing};
use vessel_app_link::{Command, usage::Ask};
use vessel_app_view::usage::{
    LimitStatus, Phase, SessionSelection, limit_failure, limit_view, quota_facts, timestamp,
};
use vessel_proto::{EventRecord, SessionId, limits::LimitRefresh};

impl Shell {
    pub(super) fn settle_session_usage(&mut self) {
        let owner = self
            .areas
            .session
            .open
            .as_ref()
            .filter(|_| self.route == Route::Session)
            .map(|open| open.session);
        if self.areas.usage.session_owner != owner {
            self.areas.usage.session.close();
            self.areas.usage.session_epoch = None;
            self.areas.usage.session_owner = owner;
        }
        let selected = self
            .areas
            .session
            .open
            .as_ref()
            .filter(|_| self.route == Route::Session)
            .filter(|open| {
                self.fleet.machines.iter().any(|machine| {
                    machine.name == open.machine && machine.connection_context().is_some()
                })
            })
            .and_then(|open| {
                let machine = self
                    .fleet
                    .machines
                    .iter()
                    .find(|machine| machine.name == open.machine)?
                    .connection_context()?;
                Some(SessionSelection {
                    machine,
                    session: open.session,
                    binding: self.areas.usage.session_epoch,
                })
            });
        if selected.as_ref().is_some_and(|selected| {
            self.areas
                .usage
                .policy_pending
                .as_ref()
                .is_some_and(|pending| pending.machine == selected.machine)
                || self
                    .areas
                    .usage
                    .source_pending
                    .as_ref()
                    .is_some_and(|pending| pending.machine == selected.machine)
        }) {
            return;
        }
        let reader = &mut self.areas.usage.session;
        if selected.as_ref() == reader.selection() {
            return;
        }
        reader.close();
        if let Some(selected) = selected
            && let Some(request) = reader.request(selected)
        {
            self.command(Command::Usage(Ask::Session {
                request,
                refresh: LimitRefresh::Cached,
            }));
        }
    }
    pub(crate) fn usage_records(&mut self, session: SessionId, records: &[EventRecord]) {
        if self
            .areas
            .session
            .open
            .as_ref()
            .is_none_or(|open| open.session != session)
        {
            return;
        }
        if self.areas.usage.session_owner != Some(session) {
            self.areas.usage.session.close();
            self.areas.usage.session_epoch = None;
            self.areas.usage.session_owner = Some(session);
        }
        let epoch = vessel_app_view::usage::binding_epoch(records);
        if let Some(epoch) = epoch
            && self.areas.usage.session_epoch.is_none_or(|old| epoch > old)
        {
            self.areas.usage.session_epoch = Some(epoch);
            self.areas.usage.session.close();
        }
    }
    fn usage_refresh_session(&mut self, expected: SessionSelection) {
        if !self.usage_connection_current(&expected.machine) {
            return;
        }
        let reader = &mut self.areas.usage.session;
        if reader.selection() != Some(&expected) {
            return;
        }
        let Some(selected) = reader.selection().cloned() else {
            return;
        };
        if let Some(request) = reader.request(selected) {
            self.command(Command::Usage(Ask::Session {
                request,
                refresh: LimitRefresh::Refresh,
            }));
        }
    }
}

pub(crate) fn session_card(
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.usage;
    let text = shell.words.usage();
    if state.session.phase().observation().is_none() {
        let _selected = state.session.selection()?;
        return Some(
            div()
                .min_w_0()
                .max_w(px(300.0))
                .children(draw::read_status(state.session.phase(), shell, tokens))
                .into_any_element(),
        );
    }
    let limits = state.session.phase().observation()?;
    let view = limit_view(limits);
    let mut card = div()
        .min_w_0()
        .max_w(px(300.0))
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .border_t_1()
        .border_color(tokens.neutral.border_accent)
        .pt(px(spacing::S2))
        .child(message(text.accounts(), tokens));
    let status = match view.status {
        LimitStatus::Off => Some(text.lookup_off()),
        LimitStatus::NotRequested => Some(text.not_requested()),
        LimitStatus::Refreshing => Some(text.loading()),
        LimitStatus::Current => None,
        LimitStatus::Failed(reason) => Some(limit_failure(reason, shell.words)),
    };
    card = card.children(status.map(|status| message(status, tokens)));
    if let Some(snapshot) = view.snapshot {
        if view.status != LimitStatus::Current {
            card = card.child(message(text.retained(), tokens));
        }
        card = card
            .child(draw::facts(
                quota_facts(limits, snapshot, shell.words),
                Density::Compact,
                tokens,
            ))
            .children(
                snapshot
                    .windows
                    .iter()
                    .map(|window| super::windows::draw(shell, window, None, tokens, cx)),
            );
        let at = timestamp(snapshot.at, state.offset, shell.words);
        card = card.child(message(text.refreshed_at(&at).unwrap_or(at), tokens));
    }
    let selection = state.session.selection().cloned();
    let enabled = !matches!(state.session.phase(), Phase::Loading { .. });
    Some(
        card.child(
            outline_button("usage-session-refresh", text.refresh(), enabled, tokens).when(
                enabled,
                |button| {
                    button.on_click(cx.listener(move |shell, _, _, cx| {
                        if let Some(selection) = &selection {
                            shell.usage_refresh_session(selection.clone());
                        }
                        cx.notify();
                    }))
                },
            ),
        )
        .into_any_element(),
    )
}

pub(crate) fn limit_notice(
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.usage;
    let limits = state.session.phase().observation()?;
    let reported = limits.notice.as_ref()?;
    let text = shell.words.usage();
    let reset = reported
        .latest_reported_reset
        .filter(|at| *at > chrono::Utc::now());
    let mut banner = div()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .pb(px(spacing::S2))
        .child(notice(
            text.limit_notice(),
            vessel_app_kit::alert::Variant::Warning,
            tokens,
        ));
    if let Some(reset) = reset {
        banner = banner.child(message(timestamp(reset, state.offset, shell.words), tokens));
    }
    banner = banner
        .child(message(text.wait(), tokens))
        .child(message(text.wait_hint(), tokens));
    Some(
        banner
            .child(
                outline_button(
                    "usage-limit-switch-model",
                    text.switch_model(),
                    true,
                    tokens,
                )
                .on_click(
                    cx.listener(|shell, _, window, cx| shell.press_model_control(window, cx)),
                ),
            )
            .into_any_element(),
    )
}
