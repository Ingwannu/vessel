//! Historical member reports, preserving every conflict path.
use super::{Shell, frame};
use chrono::Utc;
use vessel_app_kit::ui::{
    AnyElement, InteractiveElement as _, IntoElement as _, ParentElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, settings, theme::spacing};
use vessel_app_view::sync::{Part, Snapshot, member_details, member_variant};

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
                let row = settings::row_at(
                    member.label.clone(),
                    Some(
                        member_details(shell.t.lang, member, comparison, Utc::now(), state.offset)
                            .into(),
                    ),
                    None,
                    Some(
                        frame::badge(
                            words.member_state(comparison),
                            member_variant(comparison),
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
                        .child(
                            div()
                                .type_(Type::Mono)
                                .child(vessel_app_view::sync::INCOMING_SUFFIX),
                        )
                        .children(member.conflicts.iter().map(|path| {
                            div()
                                .type_(Type::Mono)
                                .whitespace_normal()
                                .child(path.as_str().to_owned())
                        }))
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
