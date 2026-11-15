//! Historical member reports, preserving every conflict path. The render
//! loop keeps each row, its hover anchor and its immediately following
//! notice together; `MemberRow` owns the presentation decisions.
use super::{Shell, frame};
use chrono::Utc;
use vessel_app_kit::ui::{
    AnyElement, InteractiveElement as _, IntoElement as _, ParentElement as _, Styled as _,
    StyledText, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, settings, theme::spacing};
use vessel_app_view::sync::{INCOMING_SUFFIX, MemberRow, Part, Snapshot};

pub(super) fn render(
    shell: &Shell,
    snapshot: &Snapshot,
    tokens: &Tokens,
    compact: bool,
) -> AnyElement {
    let words = shell.t.lang.sync();
    let state = &shell.areas.settings.sync;
    let mut card = settings::card(tokens).child(frame::message(words.members_hint, tokens));
    match &snapshot.members {
        Err(why) => card = card.child(frame::problem(shell, Part::Members, *why, tokens)),
        Ok(members) => {
            if members.is_empty() {
                card = card.child(frame::message(words.empty_members, tokens));
            }
            for member in members {
                let comparison = snapshot.member_state(member);
                let presentation =
                    MemberRow::of(shell.t.lang, member, comparison, Utc::now(), state.offset);
                let row = settings::row_at(
                    member.label.clone(),
                    presentation.hint.map(Into::into),
                    None,
                    Some(
                        frame::badge(
                            words.member_badge(presentation.badge),
                            presentation.badge.variant(),
                            tokens,
                        )
                        .into_any_element(),
                    ),
                    false,
                    compact,
                    tokens,
                )
                .id(format!("sync-member-{}", member.device));
                let row = match state.places.get(&member.device) {
                    Some(place) => state.tips.on(
                        row,
                        place,
                        words.exact_report(member.reported_at, state.offset),
                        compact,
                    ),
                    None => row,
                };
                card = card.child(row);
                if !member.conflicts.is_empty() {
                    let description = div()
                        .flex()
                        .flex_col()
                        .gap(px(spacing::S2))
                        .child(words.changes_hint)
                        .children(member.conflicts.iter().map(|path| {
                            div()
                                .type_(Type::Mono)
                                .whitespace_normal()
                                .child(path.as_str().to_owned())
                        }))
                        .child(copy_location(words.received_copies()))
                        .into_any_element();
                    card =
                        card.child(div().p(px(spacing::S4)).child(vessel_app_kit::alert::alert(
                            vessel_app_kit::alert::Content {
                                title: Some(words.changes.into()),
                                description: Some(description),
                                action: None,
                            },
                            crate::badge::alert_variant(vessel_app_view::sync::KEPT_EDITS_NOTICE),
                            vessel_app_kit::alert::Size::Small,
                            tokens,
                        )));
                }
            }
        }
    }
    settings::section(words.members, card, tokens).into_any_element()
}

// GPUI applies only the code family's override lazily, retaining the
// alert's inherited size/colour and Pretendard fallback. This is one
// wrapping sentence, not three flex children pretending to be inline.
fn copy_location(text: String) -> impl vessel_app_kit::ui::IntoElement {
    let code = text
        .match_indices(INCOMING_SUFFIX)
        .map(|(start, suffix)| {
            (
                start..start.saturating_add(suffix.len()),
                Type::Mono.family(),
            )
        })
        .collect::<Vec<_>>();
    div()
        .whitespace_normal()
        .child(StyledText::new(text).with_font_family_overrides(code))
}
