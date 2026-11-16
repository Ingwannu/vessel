//! One Plan Markdown body serves the stream and the approval dock. The caller
//! owns disclosure and actions so displaying a plan can never approve it.

use vessel_app_kit::chat_markdown::actions::CopyStyle;
use vessel_app_kit::chat_markdown::{Options, chat_markdown_with};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, ElementId, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, breakpoint, version_of};
use vessel_app_view::localization::TimelinePlan;
use vessel_app_view::timeline::{PlanExpansion, PlanOutcome};

use super::entries::disclosure;
use crate::shell::Shell;

/// A phone keeps a full touch target even though the reference's pointer header is 24.
const TOUCH_HEADER: f32 = 44.0;

/// The caller supplies identity and actions; all plan bodies share the same surface.
pub(crate) struct PlanCard<'a> {
    pub(crate) id: ElementId,
    pub(crate) text: &'a str,
    pub(crate) title: Option<&'a str>,
    pub(crate) description: Option<&'a str>,
    pub(crate) outcome: PlanOutcome,
    pub(crate) expansion: PlanExpansion,
    pub(crate) footer: Option<AnyElement>,
}

/// Render a plan with an optional approval footer, without owning any network action.
pub(crate) fn card(
    card: PlanCard<'_>,
    on_toggle: impl Fn(&mut Shell, &mut Window, &mut Context<'_, Shell>) + 'static,
    words: TimelinePlan<'_>,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let open = card.expansion == PlanExpansion::Expanded;
    let title = match card.outcome {
        PlanOutcome::Proposed | PlanOutcome::Pending => {
            card.title.unwrap_or(words.title(card.outcome))
        }
        PlanOutcome::Approved | PlanOutcome::Rejected | PlanOutcome::Canceled => {
            words.title(card.outcome)
        }
    };
    let header = div()
        .id(card.id.clone())
        .flex()
        .items_center()
        .gap(px(spacing::S2))
        .min_h(px(if breakpoint::phone(window) {
            TOUCH_HEADER
        } else {
            spacing::S6
        }))
        .cursor_pointer()
        .on_click(cx.listener(move |shell, _event, window, cx| on_toggle(shell, window, cx)))
        .child(disclosure(open, tokens.neutral.foreground_muted))
        .child(
            div()
                .min_w(px(spacing::S0))
                .flex_1()
                .child(title.to_owned()),
        );
    div()
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .p(px(spacing::S3))
        .rounded(px(spacing::S2))
        .border_1()
        .border_color(tokens.neutral.border)
        .bg(tokens.neutral.surface1)
        .type_(Type::Body)
        .text_color(tokens.neutral.foreground)
        .child(header)
        .when(open, |body| {
            body.children(card.description.map(|description| {
                div()
                    .text_color(tokens.neutral.foreground_muted)
                    .child(description.to_owned())
            }))
            .child(chat_markdown_with(
                card.id,
                card.text,
                version_of(card.text),
                Options {
                    marks: None,
                    actions: Some(super::markdown_actions::unavailable(
                        words.image_unavailable(),
                    )),
                    copy: CopyStyle::Plain,
                },
                tokens,
                window,
                cx,
            ))
        })
        .children(card.footer)
        .into_any_element()
}
