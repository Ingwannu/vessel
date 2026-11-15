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
//!   none shows; a plan scrolls inside the card past 320 (200 on a
//!   phone), and the card itself past 520 (440 on a phone), where Paseo's
//!   card scrolls with the conversation.

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, ElementId, FluentBuilder as _, InteractiveElement as _,
    IntoElement as _, ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window,
    div, px,
};
use vessel_app_kit::{
    CardLook, CardState, Start, Tokens, Type, TypeExt as _, breakpoint, card_button, chat_markdown,
    output_box, version_of,
};
use vessel_app_link::ActKind;
use vessel_app_view::ConversationWords;
use vessel_app_view::timeline::{
    AnswerMark, CardTitle, Permission, PermissionCard, ToolView, permission_card,
};
use vessel_proto::{PermissionId, PermissionOptionId};

use super::entries::Paint;
use super::look::Look;
use super::{Answer, ConversationArea, Place, Seat};
use crate::shell::Shell;

/// The tallest the card stands over the composer before its content
/// scrolls (Claude Code's plan exit stacks seven answers on a phone):
/// a phone's, and any wider window's.
const CARD_PHONE: f32 = 440.0;
const CARD: f32 = 520.0;

/// The tallest a plan stands in the card before it scrolls: a phone's,
/// and any wider window's.
const PLAN_PHONE: f32 = 200.0;
const PLAN: f32 = 320.0;

/// An answer being sent: its request and the option chosen.
pub(crate) type Sending = (PermissionId, PermissionOptionId);

impl Shell {
    /// Answer the waiting request `id` of the conversation at `P` with
    /// `option`, the place's way.
    fn permit<P: Place>(
        &mut self,
        id: PermissionId,
        option: PermissionOptionId,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(area) = P::SEAT.get_mut(self) else {
            return;
        };
        area.permit_sending = Some((id, option.clone()));
        area.refused = None;
        P::answer(self, Answer::Permit(id, option), cx);
        cx.notify();
    }

    /// Fold or unfold a plan request's plan.
    fn permit_plan_toggle(&mut self, seat: Seat, cx: &mut Context<'_, Self>) {
        if let Some(area) = seat.get_mut(self) {
            area.plan_folded = !area.plan_folded;
            cx.notify();
        }
    }
}

/// The card for the first request waiting in the conversation at `P`,
/// when one waits: the log's, else one the node says waits whose record
/// the log no longer holds (drawn from the node's word, with no entry
/// made for it).
pub(crate) fn card<P: Place>(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let seat = P::SEAT;
    let area = seat.get(shell)?;
    let unseen = area.timeline.unseen_permissions(&area.live);
    let live;
    let permission = if let Some((_entry, permission)) = area.timeline.pending() {
        permission
    } else {
        let request = *unseen.first()?;
        live = Permission {
            request: request.clone(),
            // Named as the fold names a request: its tool call's title.
            title: P::tool(shell, &request.tool_call).map(|tool| tool.title.clone()),
            outcome: None,
        };
        &live
    };
    let words = shell.t.lang.conversation();
    let tool = P::tool(shell, &permission.request.tool_call);
    let card = permission_card(
        permission,
        tool,
        area.timeline
            .waiting_permissions()
            .saturating_add(unseen.len()),
        |kind| words.permission_kind(kind),
    );
    let narrow = breakpoint::phone(window);
    let id = permission.request.id;
    let refused = area
        .refused
        .as_ref()
        .filter(|(kind, _)| *kind == ActKind::Permit)
        .map(|(_, why)| why.clone());
    // The answer going, unless it failed or another request now shows.
    let sending = area
        .permit_sending
        .as_ref()
        .filter(|(asked, _)| *asked == id && refused.is_none())
        .map(|(_, option)| option.clone());
    let title = match card.title {
        CardTitle::Agent(title) => title.to_owned(),
        CardTitle::Plan => words.permission_plan().to_owned(),
        CardTitle::Required => words.permission_required().to_owned(),
    };
    let head = head(title, card.count, *words, tokens);
    let holding = P::busy(shell);
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
    let body = body((shell, area, seat), &card, tool, head, tokens, window, cx);
    Some(frame(body, footer, narrow, tokens))
}

/// Under the body: the question, the answers, and a refused answer's
/// words in the danger colour.
fn footer(answers: Div, refused: Option<String>, words: ConversationWords, tokens: &Tokens) -> Div {
    let neutral = tokens.neutral;
    div()
        .flex()
        .flex_col()
        .gap(px(8.0))
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
                .child(words.question_refused(&why))
        }))
}

/// The card around the body and the footer: scrolling inside past its
/// height (lower on a phone).
fn frame(body: AnyElement, footer: Div, narrow: bool, tokens: &Tokens) -> AnyElement {
    let neutral = tokens.neutral;
    div()
        .id("permit-card")
        .max_h(px(if narrow { CARD_PHONE } else { CARD }))
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .gap(px(8.0))
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
        .gap(px(8.0))
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
    let mut row = div().flex().gap(px(8.0));
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

/// The card's body: a plan's head and Markdown, or the head and the
/// tool's detail (a shell call's `$ command`, an edit's diff).
fn body(
    (shell, area, seat): (&Shell, &ConversationArea, Seat),
    card: &PermissionCard<'_>,
    tool: Option<&ToolView>,
    head: Div,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let column = div().flex().flex_col().gap(px(8.0));
    if let Some(plan) = &card.plan {
        let folded = area.plan_folded;
        return column
            .child(head_with_chevron(head, (seat, folded), tokens, cx))
            .when(!folded, |body| {
                // Paseo's plan card scrolls with the conversation; over the
                // composer the plan scrolls inside the card instead, so the
                // card never stands taller than the window.
                let tall = if breakpoint::phone(window) {
                    PLAN_PHONE
                } else {
                    PLAN
                };
                body.child(
                    div()
                        .id("permit-plan-body")
                        .max_h(px(tall))
                        .overflow_y_scroll()
                        .child(chat_markdown(
                            "permit-plan",
                            plan,
                            version_of(plan),
                            None,
                            tokens,
                            window,
                            cx,
                        )),
                )
            })
            .into_any_element();
    }
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

/// A plan request's head: Paseo's chevron (16, turned down while open)
/// before the title, folding the plan.
fn head_with_chevron(
    head: Div,
    (seat, folded): (Seat, bool),
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let chevron = if folded {
        Glyph::ChevronRight
    } else {
        Glyph::ChevronDown
    };
    div()
        .id("permit-plan-head")
        .flex()
        .items_center()
        .gap(px(8.0))
        .min_h(px(24.0))
        .cursor_pointer()
        .on_click(cx.listener(move |shell, _event, _window, cx| {
            shell.permit_plan_toggle(seat.clone(), cx);
        }))
        .child(vessel_app_kit::glyph(
            chevron,
            16.0,
            tokens.neutral.foreground_muted,
        ))
        .child(head.flex_1())
        .into_any_element()
}
