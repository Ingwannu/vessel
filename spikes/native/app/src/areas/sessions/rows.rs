//! Date buckets and session rows keep archived, active and failed sessions visible.
use super::lifecycle::HistoryAction;
use crate::shell::Shell;
use chrono::Local;
use vessel_app_kit::agent_mark::{Size, Space};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, ElementId, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, outline_button};
use vessel_app_link::sessions::Binding;
use vessel_app_view::machine_book::Name;
use vessel_app_view::sessions::{DateBucket, LifecycleTarget, SessionAddress};
use vessel_app_view::{Card, Reach};

pub(super) fn failures(shell: &Shell, tokens: &Tokens) -> Vec<AnyElement> {
    shell
        .fleet
        .machines
        .iter()
        .filter(|machine| shell.areas.sessions.scope.includes(machine))
        .filter_map(|machine| match &machine.reach {
            Reach::Online => None,
            Reach::Unreachable(_) => Some(
                div()
                    .type_(Type::Body)
                    .text_color(tokens.neutral.destructive)
                    .child(machine.name.clone())
                    .child(div().child(shell.words.sessions().host_failed()))
                    .into_any_element(),
            ),
        })
        .collect()
}
pub(super) fn list(
    shell: &Shell,
    found: &[&Card],
    shown: usize,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Vec<AnyElement> {
    let today = Local::now().date_naive();
    let mut previous = None;
    let mut rows = Vec::new();
    for card in found.iter().take(shown) {
        let bucket = DateBucket::of(
            card.detail.active_at.with_timezone(&Local).date_naive(),
            today,
        );
        if previous != Some(bucket) {
            rows.push(
                div()
                    .pt(px(spacing::S3))
                    .type_(Type::Label)
                    .text_color(tokens.neutral.foreground_muted)
                    .child(heading(shell, bucket))
                    .into_any_element(),
            );
            previous = Some(bucket);
        }
        rows.push(row(shell, card, tokens, cx));
    }
    rows
}
fn heading(shell: &Shell, bucket: DateBucket) -> &'static str {
    let words = shell.words.sessions();
    match bucket {
        DateBucket::Today => words.today(),
        DateBucket::Yesterday => words.yesterday(),
        DateBucket::ThisWeek => words.this_week(),
        DateBucket::ThisMonth => words.this_month(),
        DateBucket::Older => words.older(),
    }
}
fn row(shell: &Shell, card: &Card, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> AnyElement {
    let target = lifecycle_target(shell, card);
    let mut open = crate::areas::session::Open::of(card);
    open.binding = target.as_ref().and_then(|target| {
        target.node.map(|node| Binding {
            host: target.host.clone(),
            address: SessionAddress {
                node,
                session: target.session,
            },
        })
    });
    let id = format!("session-history-{}-{}", card.machine, card.session);
    div()
        .flex()
        .items_center()
        .gap(px(spacing::S2))
        .p(px(spacing::S2))
        .border_b_1()
        .border_color(tokens.neutral.border)
        .child(
            div()
                .id(ElementId::Name(id.clone().into()))
                .flex_1()
                .min_w_0()
                .cursor_pointer()
                .on_click(cx.listener(move |shell, _, window, cx| {
                    shell.open_session(open.clone(), window, cx);
                }))
                .min_h(px(vessel_app_kit::breakpoint::TOUCH_TARGET))
                .child(content(shell, card, tokens)),
        )
        .child(lifecycle_button(shell, card, target, &id, tokens, cx))
        .into_any_element()
}

fn lifecycle_button(
    shell: &Shell,
    card: &Card,
    target: Option<LifecycleTarget>,
    id: &str,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let archived = card.detail.marks.archived_at.is_some();
    let action = if archived {
        shell.words.sessions().unarchive()
    } else {
        shell.words.sessions().archive()
    };
    outline_button(
        ElementId::Name(format!("{id}-archive").into()),
        action,
        target.is_some() && !shell.sessions_lifecycle_busy(),
        tokens,
    )
    .min_h(px(vessel_app_kit::breakpoint::TOUCH_TARGET))
    .on_click(cx.listener(move |shell, _, window, cx| {
        if let Some(target) = target.clone() {
            let action = if archived {
                HistoryAction::Unarchive
            } else {
                HistoryAction::Archive
            };
            shell.history_lifecycle(target, action, window, cx);
        }
    }))
    .into_any_element()
}

fn lifecycle_target(shell: &Shell, card: &Card) -> Option<LifecycleTarget> {
    let node = shell
        .fleet
        .machines
        .iter()
        .find(|machine| machine.name == card.machine)?
        .node?;
    Some(LifecycleTarget {
        host: Name::parse(&card.machine).ok()?,
        session: card.session,
        node: Some(node),
    })
}

fn content(shell: &Shell, card: &Card, tokens: &Tokens) -> vessel_app_kit::ui::Div {
    let state = if card.detail.marks.archived_at.is_some() {
        shell.words.sessions().archived().to_owned()
    } else {
        shell.t.phrase(&card.phrase)
    };
    div()
        .min_w_0()
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(spacing::S2))
                .child(crate::agent_mark::mark_muted(
                    card.detail.harness.as_ref(),
                    Size::Sixteen,
                    Space::Compact,
                    tokens,
                ))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .type_(Type::Body)
                        .text_color(tokens.neutral.foreground_muted)
                        .child(shell.t.title(&card.title).to_owned()),
                ),
        )
        .child(
            div()
                .truncate()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(format!(
                    "{} · {} · {state} · {}",
                    card.machine,
                    card.folder(),
                    shell.t.since(card.minutes_since(chrono::Utc::now()))
                )),
        )
}
