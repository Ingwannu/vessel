//! A permission the agent waits for, answered over the composer: Paseo's
//! permission card (`agent-stream/view.tsx`, `PermissionRequestCard`,
//! `PermissionActionButton`, `permissionStyles`), a plan request as its
//! plan card (`components/plan-card.tsx`), GOAT's "1/N" when several
//! wait. The rules are the view's (`timeline::permission_card`).
//!
//! [Decision Log]
//! - Purpose: the owner on the card it replaces ("허가 여부 ui 디자인
//!   개떡같네 paseo나 goat 보고 배워"): a red-bordered tinted box, a bold
//!   sentence wrapped round the whole command, four filled pills.
//! - Constraint: the card stands in the conversation's slot over the
//!   composer (`dock`), above a question's card and the queue; the
//!   session's layout (the lead's) no longer draws one.
//! - The reference, value for value: `surface1`, a hairline of `border`,
//!   radius 8, 12 inside, 8 between; the title 14 on 22 in the
//!   foreground; the tool's detail (a shell call's `$ command` in the
//!   fixed-width box, an edit's diff), scrolling inside; "How would you
//!   like to proceed?" muted with 4 above and below; the answers in a
//!   wrapping row (a column on a phone) of the kit's card buttons, a check
//!   before an allow and a cross before a reject, the primary answer in the
//!   foreground and the rest muted. While an answer goes its button spins
//!   and none answers; a failed answer is said in the card. A plan is the
//!   plan card: a chevron of 16 that folds it, its title, its Markdown, the
//!   same footer.
//! - Differences, kept: the detail's box is the kit's (GOAT's cap of 192,
//!   where Paseo's is 200); the agent gives no description over ACP, so
//!   none shows; the card scrolls past 520 (440 on a phone), where Paseo's
//!   card scrolls with the conversation.

use std::collections::{HashMap, HashSet};
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, ElementId, FluentBuilder as _, InteractiveElement as _,
    IntoElement as _, ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window,
    div, px,
};
use vessel_app_kit::{
    CardLook, CardState, Start, Tokens, Type, TypeExt as _, breakpoint, card_button, output_box,
};
use vessel_app_link::ActKind;
use vessel_app_view::ConversationWords;
use vessel_app_view::timeline::{
    AnswerMark, CardTitle, PermissionCard, PlanExpansion, PlanOutcome, ToolView,
    permission_card_for,
};
use vessel_proto::{PermissionId, PermissionOptionId};

use super::entries::Paint;
use super::look::Look;
use super::plan_card::{PlanCard, card as plan_card};
use super::{Answer, Place, Seat};
use crate::shell::Shell;
use vessel_app_kit::theme::spacing;

/// The tallest the card stands over the composer before its content
/// scrolls (Claude Code's plan exit stacks seven answers on a phone):
/// a phone's, and any wider window's.
const CARD_PHONE: f32 = 440.0;
const CARD: f32 = 520.0;

/// An answer being sent: its request and the option chosen.
pub(crate) type Sending = HashMap<PermissionId, PermissionOptionId>;
/// Disclosure is independent for every simultaneously pending plan.
pub(crate) type Disclosures = HashSet<PermissionId>;

impl Shell {
    /// Answer the waiting request `id` of the conversation at `P` with
    /// `option`, the place's way.
    fn permit<P: Place>(
        &mut self,
        id: PermissionId,
        option: PermissionOptionId,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(area) = (P::SEAT.get_mut)(self) else {
            return;
        };
        if !area
            .response_failures
            .retryable(vessel_app_view::responses::Request::Permission(id))
            || !area.response_waiting(vessel_app_view::responses::Request::Permission(id))
            || area.permit_sending.contains_key(&id)
                && area
                    .response_failures
                    .get(vessel_app_view::responses::Request::Permission(id))
                    .is_none()
                && area
                    .refused
                    .as_ref()
                    .is_none_or(|(kind, _)| *kind != ActKind::Permit)
        {
            return;
        }
        let _previous = area.permit_sending.insert(id, option.clone());
        area.refused = None;
        P::answer(self, Answer::Permit(id, option), cx);
        cx.notify();
    }

    /// Fold or unfold a plan request's plan.
    fn permit_plan_toggle(&mut self, seat: Seat, id: PermissionId, cx: &mut Context<'_, Self>) {
        if let Some(area) = (seat.get_mut)(self) {
            if !area.response_waiting(vessel_app_view::responses::Request::Permission(id)) {
                return;
            }
            if !area.plan_folded.remove(&id) {
                let _inserted = area.plan_folded.insert(id);
            }
            cx.notify();
        }
    }
}

/// All pending permissions, including exact live requests with pruned records.
pub(crate) fn card<P: Place>(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let area = (P::SEAT.get)(shell)?;
    let ids: Vec<_> = area
        .timeline
        .pending_permissions()
        .map(|(_, permission)| permission.request.id)
        .chain(
            area.timeline
                .unseen_permissions(&area.live)
                .into_iter()
                .map(|request| request.id),
        )
        .collect();
    let cards: Vec<_> = ids
        .into_iter()
        .filter_map(|id| card_at::<P>(shell, id, tokens, window, cx))
        .collect();
    if cards.is_empty() {
        return None;
    }
    Some(
        div()
            .flex()
            .flex_col()
            .gap(px(spacing::S2))
            .children(cards)
            .into_any_element(),
    )
}

/// An exact Inbox request uses the same card, independently of list position.
pub(crate) fn card_at<P: Place>(
    shell: &Shell,
    id: PermissionId,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let seat = P::SEAT;
    let area = (seat.get)(shell)?;
    let logged = area
        .timeline
        .pending_permissions()
        .find(|(_, permission)| permission.request.id == id);
    let (request, title) = if let Some((_, permission)) = logged {
        (&permission.request, permission.title.as_deref())
    } else {
        let request = area
            .timeline
            .unseen_permissions(&area.live)
            .into_iter()
            .find(|request| request.id == id)?;
        let title = P::tool(shell, &request.tool_call).map(|tool| tool.title.as_str());
        (request, title)
    };
    let words = shell.t.lang.conversation();
    let tool = P::tool(shell, &request.tool_call);
    let card = permission_card_for(request, title, tool, 1, |kind| words.permission_kind(kind));
    let narrow = breakpoint::phone(window);
    let refused = area
        .response_failures
        .get(vessel_app_view::responses::Request::Permission(id))
        .map(|failure| crate::areas::responses::problem(shell.words, failure).to_owned())
        .or_else(|| {
            area.refused
                .as_ref()
                .filter(|(kind, _)| *kind == ActKind::Permit)
                .map(|(_, why)| why.clone())
        });
    // The answer going, unless it failed or another request now shows.
    let sending = area
        .permit_sending
        .get(&id)
        .filter(|_| refused.is_none())
        .cloned();
    let title = match card.title {
        CardTitle::Agent(title) => title.to_owned(),
        CardTitle::Plan => words.permission_plan().to_owned(),
        CardTitle::Required => words.permission_required().to_owned(),
    };
    let holding = P::busy(shell)
        || !area
            .response_failures
            .retryable(vessel_app_view::responses::Request::Permission(id));
    let answers = answers::<P>(
        &card,
        (sending.as_ref(), holding),
        id,
        narrow,
        *words,
        tokens,
        cx,
    );
    let footer = footer(answers, refused, *words, tokens);
    if let Some(plan) = card.plan.as_deref() {
        let shown = area.shown.as_ref()?;
        let key =
            ElementId::Name(format!("permit-plan:{}:{}:{id}", shown.machine, shown.session).into());
        let plan = plan_card(
            PlanCard {
                id: key,
                text: plan,
                title: Some(&title),
                description: None,
                outcome: PlanOutcome::Pending,
                expansion: if area.plan_folded.contains(&id) {
                    PlanExpansion::Collapsed
                } else {
                    PlanExpansion::Expanded
                },
                footer: Some(footer.into_any_element()),
            },
            move |shell, _window, cx| shell.permit_plan_toggle(seat, id, cx),
            shell.words.timeline_plan(),
            tokens,
            window,
            cx,
        );
        return Some(
            div()
                .id(card_id(id))
                .max_h(px(if narrow { CARD_PHONE } else { CARD }))
                .overflow_y_scroll()
                .child(plan)
                .into_any_element(),
        );
    }
    let head = head(title, card.count, *words, tokens);
    let body = body((shell, seat), &card, tool, head, tokens, window, cx);
    Some(frame(id, body, footer, narrow, tokens))
}

/// Under the body: the question, the answers, and a refused answer's
/// words in the danger colour.
fn footer(answers: Div, refused: Option<String>, words: ConversationWords, tokens: &Tokens) -> Div {
    let neutral = tokens.neutral;
    div()
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .child(
            div()
                .my(px(4.0))
                .text_color(neutral.foreground_muted)
                .child(words.permission_question()),
        )
        .child(answers)
        .children(refused.map(|why| {
            div()
                .text_size(px(13.0))
                .text_color(neutral.destructive)
                .child(why)
        }))
}

/// The card around the body and the footer: scrolling inside past its
/// height (lower on a phone).
fn frame(
    id: PermissionId,
    body: AnyElement,
    footer: Div,
    narrow: bool,
    tokens: &Tokens,
) -> AnyElement {
    let neutral = tokens.neutral;
    div()
        .id(card_id(id))
        .max_h(px(if narrow { CARD_PHONE } else { CARD }))
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .p(px(12.0))
        .rounded(px(8.0))
        .border_1()
        .border_color(neutral.border)
        .bg(neutral.surface1)
        .face(Type::Body)
        .text_size(px(14.0))
        .line_height(px(20.0))
        .child(body)
        .child(footer)
        .into_any_element()
}

// These cards are siblings in one pending stack. Their stateful descendants
// (scroll areas, buttons and detail boxes) must live below distinct identities.
fn card_id(id: PermissionId) -> ElementId {
    ElementId::Name(format!("permit-card-{id}").into())
}

/// The title (14 on 22, the foreground) and GOAT's count at its right.
fn head(
    title: String,
    count: Option<(usize, usize)>,
    words: ConversationWords,
    tokens: &Tokens,
) -> Div {
    let neutral = tokens.neutral;
    div()
        .flex()
        .items_start()
        .gap(px(spacing::S2))
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .line_height(px(22.0))
                .text_color(neutral.foreground)
                .child(title),
        )
        .children(count.map(|(at, total)| {
            div()
                .flex_none()
                .pt(px(4.0))
                .text_size(px(11.0))
                .line_height(px(14.0))
                .text_color(neutral.foreground_muted)
                .child(words.permission_count(at, total))
        }))
}

/// The answers: the kit's card buttons, in a wrapping row (a column on a
/// phone); while one goes, it spins and none answers.
fn answers<P: Place>(
    card: &PermissionCard<'_>,
    (sending, held): (Option<&PermissionOptionId>, bool),
    id: PermissionId,
    narrow: bool,
    words: ConversationWords,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let mut row = div().flex().gap(px(spacing::S2));
    row = if narrow {
        row.flex_col()
    } else {
        row.flex_row().flex_wrap().items_center()
    };
    for (index, answer) in card.answers.iter().enumerate() {
        let option = answer.option.id.clone();
        let state = match sending {
            Some(going) if *going == option => CardState::Sending,
            Some(_) => CardState::Waiting,
            None if held => CardState::Waiting,
            None => CardState::Ready,
        };
        let mark = match answer.mark {
            AnswerMark::Allow => Glyph::Check,
            AnswerMark::Reject => Glyph::Remove,
        };
        let look = if answer.primary {
            CardLook::Primary
        } else {
            CardLook::Muted
        };
        let button = card_button(
            ElementId::NamedInteger("permit".into(), u64::try_from(index).unwrap_or(u64::MAX)),
            mark,
            words.answer_words(answer.words),
            look,
            state,
            tokens,
        )
        .when(state == CardState::Ready, |button| {
            button.on_click(cx.listener(move |shell, _event, _window, cx| {
                shell.permit::<P>(id, option.clone(), cx);
            }))
        });
        row = row.child(button);
    }
    row
}

/// An ordinary card's head and tool detail; plans use the shared renderer.
fn body(
    (shell, seat): (&Shell, Seat),
    card: &PermissionCard<'_>,
    tool: Option<&ToolView>,
    head: Div,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let column = div().flex().flex_col().gap(px(spacing::S2));
    let paint = Paint {
        catalog: shell.words,
        words: shell.t.lang.conversation(),
        tokens,
        look: Look::of(tokens),
        find: None,
        seat,
        entries: &[],
        agent: None,
        rewinds: None,
        forks: None,
        fork_headers: &shell.areas.conversation.forking.headers,
        above: super::origin::Above::Nothing,
    };
    let detail = match (&card.command, tool) {
        (Some(command), _) => Some(output_box(
            ElementId::NamedInteger("permit-command".into(), 0),
            &format!("$ {command}"),
            Start::Top,
            tokens,
            window,
            cx,
        )),
        (None, Some(tool)) => super::tool::diff_detail(
            &paint,
            &ElementId::NamedInteger("permit-diff".into(), 0),
            tool,
            window,
            cx,
        ),
        (None, None) => None,
    };
    column.child(head).children(detail).into_any_element()
}

#[cfg(test)]
mod tests;
