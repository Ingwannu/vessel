//! Date buckets and session rows keep archived, active and failed sessions visible.
use crate::shell::Shell;
use chrono::Local;
use vessel_app_kit::agent_mark::{Size, Space};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, ElementId, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, outline_button};
use vessel_app_view::sessions::{DateBucket, HostScope};
use vessel_app_view::{Card, Reach};

pub(super) fn failures(shell: &Shell, tokens: &Tokens) -> Vec<AnyElement> {
    shell
        .fleet
        .machines
        .iter()
        .filter(|machine| match &shell.areas.sessions.scope {
            HostScope::All => true,
            HostScope::Host(name) => machine.name == name.as_str(),
        })
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
    let open = crate::areas::session::Open::of(card);
    let target = (*card).clone();
    let archived = card.detail.marks.archived_at.is_some();
    let action = if archived {
        shell.words.sessions().unarchive()
    } else {
        shell.words.sessions().archive()
    };
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
        .child(
            outline_button(
                ElementId::Name(format!("{id}-archive").into()),
                action,
                true,
                tokens,
            )
            .min_h(px(vessel_app_kit::breakpoint::TOUCH_TARGET))
            .on_click(cx.listener(move |shell, _, window, cx| {
                if archived {
                    shell.act_on_card(
                        &target,
                        vessel_app_link::Act::Mark(vessel_proto::Mark::Unarchive),
                    );
                    cx.notify();
                } else {
                    shell.ask_archive(&target, window, cx);
                }
            })),
        )
        .into_any_element()
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
