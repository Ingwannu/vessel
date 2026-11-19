//! A session row's hover card (TRACKER 810), as Paseo's
//! `WorkspaceHoverCard` (`components/workspace-hover-card.tsx`) on the
//! kit's [`HoverCards`]: the session's name, its pull request, its machine,
//! its branch and its folder (each copyable), and its checks.
//!
//! [Decision Log]
//! - Purpose: what a row's one line leaves out, a hover away, as Paseo's
//!   sidebar shows it, with Paseo's rows and values: 260 wide; each row 12
//!   from the sides with 8 under it, its glyph 12 and 6 from its words,
//!   the words `sm` and muted on one line; the name `base` in the
//!   foreground.
//! - When: on a wide window with a pointer only (not on touch, not in the
//!   phone's sheet), never while the row's menu is open (the coordinator's
//!   ruling, as Paseo's: also not while a drag is under way, TRACKER 811).
//! - The copy rows (branch, folder): on hover the glyph becomes the copy
//!   mark and the words the foreground; a press puts the branch's name or
//!   the folder's path on the clipboard and shows the check for 2 s
//!   (Paseo's), through the kit's one "copied last" (`copy_for`).
//! - The pull request's badge and the checks row are links (Paseo's
//!   `accessibilityRole="link"`); on hover their mark becomes the outward
//!   arrow.
//! - Differences, named:
//!   - The card is a group named "Session details" (the coordinator's
//!     ruling); Paseo's role `menu` named "Workspace scripts" is left over.
//!   - No diff stat: Paseo's is a server field the node does not send yet
//!     (TRACKER 819 adds its row after the pull request's).
//!   - The checks row leads with the pull request's mark, not the forge's
//!     brand mark (the kit has none yet), and says the checks' one state as
//!     the row's line does, not a count per state (the session's summary
//!     holds one state).
//!   - The outward arrow is the kit's (`ArrowUpRight`, the inline link's),
//!     not Lucide's boxed `ExternalLink`.

use std::time::Duration;

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::picker::glyph;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, ElementId, InteractiveElement as _, IntoElement as _,
    ParentElement as _, SharedString, Stateful, StatefulInteractiveElement as _, Styled as _, div,
    px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, copied_within, copy_for};
use vessel_app_view::Card;
use vessel_app_view::sidebar::hover::{HoverChecks, HoverPull};
use vessel_app_view::workspace::pull::CardChecks;

use crate::shell::Shell;

/// The card's width (Paseo's `HOVER_CARD_WIDTH`).
const WIDTH: f32 = 260.0;
/// Every glyph on it.
const GLYPH: f32 = 12.0;
/// The checks row: its least height and the room above and below its
/// words (Paseo's `checksSummaryRow`).
const CHECKS: f32 = 28.0;
const CHECKS_Y: f32 = 6.0;
/// How long a copy row shows its check (Paseo's `setTimeout(..., 2000)`).
const COPIED_FOR: Duration = Duration::from_millis(2000);

/// The card's key for `card`'s row.
fn key(card: &Card) -> String {
    format!("{}/{}", card.machine, card.session)
}

/// `row` shows `card`'s hover card while the pointer is on it, when
/// `enabled` (a pointer on a wide window, the row's menu closed).
pub(super) fn on_row(row: Stateful<Div>, card: &Card, enabled: bool, shell: &Shell) -> Stateful<Div> {
    let key = key(card);
    let sidebar = &shell.areas.sidebar;
    sidebar
        .cards
        .on(row, &sidebar.place(&format!("card:{key}")), key, enabled)
}

/// The card shown, over everything.
pub(super) fn layer(
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let cards = &shell.areas.sidebar.cards;
    let shown = cards.shown()?;
    let card = shell
        .fleet
        .machines
        .iter()
        .flat_map(|machine| &machine.sessions)
        .find(|card| key(card) == shown.as_ref())?;
    let words = shell.t.lang.sidebar();
    let content = content(card, shell, tokens, cx);
    cards.render(content, words.session_details, WIDTH, tokens)
}

/// What the card holds, in Paseo's order.
fn content(card: &Card, shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let details = card.hover_details();
    let words = shell.t.lang.sidebar();
    let neutral = tokens.neutral;
    let title = div()
        .px(px(spacing::S3))
        .pb(px(spacing::S2))
        .type_(Type::Body)
        .text_color(neutral.foreground)
        .child(shell.t.title(&card.title).to_owned());
    let mut body = div().flex().flex_col().child(title);
    if let Some(pull) = details.pull {
        body = body.child(pull_row(pull, shell, tokens, cx));
    }
    // TRACKER 819: the diff stat's row goes here, after the pull request's.
    body = body.child(row().child(glyph(Glyph::Server, GLYPH, neutral.foreground_muted)).child(
        words_of(details.machine).text_color(neutral.foreground_muted),
    ));
    if let Some(branch) = details.branch {
        let copy = Copy {
            id: "hover-card-branch",
            mark: Glyph::Branch,
            label: branch.clone(),
            value: branch,
            said: words.copy_branch_name,
        };
        body = body.child(copy_row(copy, tokens, cx));
    }
    let copy = Copy {
        id: "hover-card-folder",
        mark: Glyph::Folder,
        label: details.folder.label,
        value: details.folder.path,
        said: words.copy_path,
    };
    body = body.child(copy_row(copy, tokens, cx));
    if let (Some(checks), Some(url)) = (details.checks, card.checks_url()) {
        // Under Paseo's 1 rule in `border`.
        body = body.child(
            checks_row(checks, url, shell, tokens, cx)
                .border_t_1()
                .border_color(neutral.border),
        );
    }
    body
}

/// A row's frame (Paseo's `cardInfoRow`).
fn row() -> Div {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(spacing::S1_5))
        .px(px(spacing::S3))
        .pb(px(spacing::S2))
}

/// Words on one line, cut when the card runs out.
fn words_of(text: impl Into<SharedString>) -> Div {
    div()
        .flex_1()
        .min_w_0()
        .truncate()
        .type_(Type::Meta)
        .child(text.into())
}

/// `mark`, giving way to `hovered` while the row (`group`) is hovered.
fn swapped(mark: AnyElement, hovered: Glyph, group: &SharedString, tokens: &Tokens) -> Div {
    div()
        .relative()
        .size(px(GLYPH))
        .flex_none()
        .child(
            div()
                .group_hover(group.clone(), |style| style.opacity(0.0))
                .child(mark),
        )
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .opacity(0.0)
                .group_hover(group.clone(), |style| style.opacity(1.0))
                .child(glyph(hovered, GLYPH, tokens.neutral.foreground)),
        )
}

/// The pull request's badge: its state's mark and its number, a link to
/// its page.
fn pull_row(pull: HoverPull, shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Stateful<Div> {
    let neutral = tokens.neutral;
    let number = pull.number.get();
    let group: SharedString = "hover-card-pull".into();
    let mark = crate::areas::pulls::state_glyph_sized(pull.phase, GLYPH, tokens);
    let url = pull.url;
    row()
        .id(ElementId::Name(group.clone()))
        .group(group.clone())
        .cursor_pointer()
        .role(gpui::Role::Link)
        .aria_label(shell.t.lang.sidebar().pull_request(number))
        .text_color(neutral.foreground_muted)
        .hover(move |style| style.text_color(neutral.foreground))
        .child(swapped(mark, Glyph::ArrowUpRight, &group, tokens))
        .child(words_of(format!("#{number}")))
        .on_click(cx.listener(move |_this, _event, _window, cx| {
            cx.stop_propagation();
            cx.open_url(&url);
        }))
}

/// A copy row's facts.
struct Copy {
    id: &'static str,
    mark: Glyph,
    label: String,
    value: String,
    said: &'static str,
}

/// A copyable fact (Paseo's `CopyableInfoRow`).
fn copy_row(copy: Copy, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Stateful<Div> {
    let neutral = tokens.neutral;
    let group: SharedString = copy.id.into();
    let copied = copied_within(&copy.value, COPIED_FOR, cx);
    let mark = if copied {
        div()
            .flex_none()
            .child(glyph(Glyph::Check, GLYPH, neutral.foreground))
    } else {
        swapped(
            glyph(copy.mark, GLYPH, neutral.foreground_muted).into_any_element(),
            Glyph::Copy,
            &group,
            tokens,
        )
    };
    let value = copy.value;
    let id = copy.id;
    row()
        .id(ElementId::Name(group.clone()))
        .debug_selector(move || id.to_owned())
        .group(group)
        .cursor_pointer()
        .role(gpui::Role::Button)
        .aria_label(copy.said)
        .text_color(if copied {
            neutral.foreground
        } else {
            neutral.foreground_muted
        })
        .hover(move |style| style.text_color(neutral.foreground))
        .child(mark)
        .child(words_of(copy.label))
        .on_click(cx.listener(move |_this, _event, window, cx| {
            cx.stop_propagation();
            copy_for(&value, COPIED_FOR, window, cx);
        }))
}

/// The checks' row under a rule: a link to their page, their state at the
/// end in its colour (Paseo's `ChecksSummaryPressable`).
fn checks_row(
    checks: HoverChecks,
    url: String,
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Stateful<Div> {
    let neutral = tokens.neutral;
    let words = shell.t.lang.sidebar();
    let status = tokens.status();
    let (which, colour) = match checks.state {
        CardChecks::Passed => (Glyph::CircleCheck, status.success.foreground),
        CardChecks::Failed => (Glyph::CircleX, status.error.foreground),
        CardChecks::Running => (Glyph::CircleDot, status.warning.foreground),
    };
    let state = checks.state.words(shell.words).to_owned();
    let group: SharedString = "hover-card-checks".into();
    let surface = neutral.surface2;
    div()
        .id(ElementId::Name(group.clone()))
        .group(group.clone())
        .min_h(px(CHECKS))
        .py(px(CHECKS_Y))
        .px(px(spacing::S3))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(spacing::S1_5))
        .cursor_pointer()
        .role(gpui::Role::Link)
        .aria_label(words.checks_said(&state))
        .text_color(neutral.foreground_muted)
        .hover(move |style| style.bg(surface).text_color(neutral.foreground))
        .child(swapped(
            glyph(Glyph::PullRequest, GLYPH, neutral.foreground_muted).into_any_element(),
            Glyph::ArrowUpRight,
            &group,
            tokens,
        ))
        .child(div().flex_none().type_(Type::Meta).child(words.checks))
        .child(
            div()
                .flex_1()
                .flex()
                .flex_row()
                .justify_end()
                .items_center()
                .gap(px(spacing::S1))
                .type_(Type::Meta)
                .text_color(colour)
                .child(glyph(which, GLYPH, colour))
                .child(state),
        )
        .on_click(cx.listener(move |_this, _event, _window, cx| {
            cx.stop_propagation();
            cx.open_url(&url);
        }))
}

#[cfg(test)]
#[path = "hover_card_tests.rs"]
mod tests;
