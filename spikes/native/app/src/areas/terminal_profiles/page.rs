//! Settings › Terminals for the chosen machine: Paseo's
//! `TerminalProfilesSection` (`screens/settings/host-page.tsx` 1311-1665).
//!
//! [Decision Log]
//! - Reference: the section "Terminal profiles" with a plus at its heading's
//!   end; a card of rows in the list's order, each with the profile's mark
//!   (else the terminal glyph), its name, and its command and arguments as
//!   a hint, then Move up and Move down (off at the ends), Edit and Remove
//!   (its trash in the destructive colour); "No profiles yet…" when the
//!   list is empty; one muted card saying to connect while the machine is
//!   not connected. Every change saves the whole list.
//! - Choice: the kit's section, card, row frame and ghost icon buttons
//!   (Paseo's `Button variant="ghost"` with a 14 icon), each named for
//!   assistive technology as Paseo's `accessibilityLabel`.
//! - Differences:
//!   - while a save waits, the row buttons and the plus wait too, so two
//!     changes never race (Paseo leaves them on, and a second save can
//!     overtake the first);
//!   - while the list is read the card is not drawn (Paseo shows the empty
//!     sentence until its config arrives);
//!   - a node too old for profiles says so (Paseo shows nothing for a
//!     missing feature);
//!   - a refused move or removal is said over the list (Paseo's
//!     `Alert.alert`, a system popup);
//!   - Paseo's "Terminal agents" card above the list is ADR 0089's hooks
//!     switch, not drawn yet.

use vessel_app_kit::agent_mark::{Size, Space};
use vessel_app_kit::alert::{self, Content};
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::settings;
use vessel_app_kit::theme::icon;
use vessel_app_kit::theme::spacing::{S0, S1, S2, S4};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, ElementId, FluentBuilder as _, InteractiveElement as _,
    IntoElement as _, ParentElement as _, SharedString, StatefulInteractiveElement as _,
    Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ButtonContent, ButtonIcon, ButtonKind, ButtonState, ControlSizing, Tokens, Type, TypeExt as _,
    button_with_content, glyph,
};
use vessel_app_view::Reach;
use vessel_app_view::agent_mark::AgentMark;
use vessel_app_view::terminal_profiles::{Problem, Step, command_line, moved};
use vessel_proto::terminal_profiles::{Profile, Profiles};

use super::dialog;
use crate::shell::Shell;

/// Paseo's `terminalProfileStyles.row.minHeight`.
const ROW_MIN: f32 = 56.0;

impl Shell {
    /// The list Settings › Terminals shows, once read.
    pub(super) fn terminal_profiles_page_list(&self) -> Option<&Profiles> {
        let state = &self.areas.terminal_profiles;
        state.listings.get(state.page.as_ref()?)?.profiles()
    }

    /// A row's Move up or Move down: the list with it moved is sent.
    pub(super) fn terminal_profile_move(
        &mut self,
        index: usize,
        step: Step,
        cx: &mut Context<'_, Self>,
    ) {
        let machine = self.areas.terminal_profiles.page.clone();
        let next = self
            .terminal_profiles_page_list()
            .and_then(|list| moved(list, index, step));
        if let (Some(machine), Some(next)) = (machine, next) {
            let _sent = self.terminal_profiles_save(machine, next);
        }
        cx.notify();
    }
}

/// What the page shows for its machine.
enum Shown<'a> {
    /// Not connected (or no machine): Paseo's one card.
    Unavailable,
    /// The node is below protocol 45.
    TooOld,
    /// Being read.
    Reading,
    /// The read was refused.
    Refused(Problem),
    /// The list.
    List(&'a Profiles),
}

fn shown(shell: &Shell) -> Shown<'_> {
    let online = crate::areas::settings::chosen_machine(shell)
        .is_some_and(|machine| machine.reach == Reach::Online);
    if !online {
        return Shown::Unavailable;
    }
    let state = &shell.areas.terminal_profiles;
    let Some(listing) = state
        .page
        .as_ref()
        .and_then(|page| state.listings.get(page))
    else {
        return Shown::Reading;
    };
    match (listing.profiles(), listing.problem()) {
        (Some(list), _) => Shown::List(list),
        (None, Some(Problem::TooOld)) => Shown::TooOld,
        (None, Some(Problem::Offline)) => Shown::Unavailable,
        (None, Some(problem @ Problem::Refused(_))) => Shown::Refused(problem),
        (None, None) => Shown::Reading,
    }
}

/// The Terminals page.
pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.words.terminal_profiles();
    let state = &shell.areas.terminal_profiles;
    let saving = state
        .page
        .as_ref()
        .and_then(|page| state.listings.get(page))
        .is_some_and(vessel_app_view::terminal_profiles::Listing::saving);
    let body = match shown(shell) {
        Shown::Unavailable => muted_card(words.unavailable(), tokens),
        Shown::TooOld => muted_card(words.too_old(), tokens),
        Shown::Reading => section(shell, None, false, tokens, cx),
        Shown::Refused(problem) => section(
            shell,
            Some(muted_card(dialog::refusal(problem, shell), tokens)),
            false,
            tokens,
            cx,
        ),
        Shown::List(list) => {
            let card = if list.entries().is_empty() {
                muted_card(words.empty(), tokens)
            } else {
                rows(shell, list, saving, compact, tokens, cx)
            };
            section(shell, Some(card), !saving, tokens, cx)
        }
    };
    let refused = state.refused.map(|problem| {
        div().mb(px(S4)).child(alert::alert(
            Content {
                title: Some(words.unable_to_save().into()),
                description: Some(dialog::refusal(problem, shell).into_any_element()),
                action: None,
            },
            alert::Variant::Error,
            alert::Size::Small,
            tokens,
        ))
    });
    div()
        .debug_selector(|| "terminal-profiles".to_owned())
        .flex()
        .flex_col()
        .children(refused)
        .child(body)
        .children(dialog::dialogs(shell, tokens, window, cx))
        .into_any_element()
}

/// The section with its plus, around `card` (none while it is read).
fn section(
    shell: &Shell,
    card: Option<Div>,
    can_add: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.words.terminal_profiles();
    let add = icon_button(
        "terminal-profiles-add".to_owned(),
        ButtonIcon::muted(Glyph::Plus),
        words.add(),
        can_add,
        tokens,
    )
    .when(can_add, |button| {
        button.on_click(cx.listener(|shell, _event, window, cx| {
            shell.terminal_profile_add(window, cx);
        }))
    });
    settings::section_with_trailing(
        words.title(),
        card.unwrap_or_else(div),
        Some(add.into_any_element()),
        tokens,
    )
}

/// Paseo's ghost icon-only `Button` (a 14 glyph), named; `id` is also its
/// test selector (Paseo's `testID`).
fn icon_button(
    id: String,
    icon: ButtonIcon,
    name: &'static str,
    enabled: bool,
    tokens: &Tokens,
) -> gpui::Stateful<Div> {
    let selector = id.clone();
    button_with_content(
        ElementId::Name(id.into()),
        ButtonContent::Icon(icon),
        ButtonKind::Ghost,
        if enabled {
            ButtonState::Enabled
        } else {
            ButtonState::Disabled
        },
        ControlSizing::Pointer,
        tokens,
    )
    .aria_label(name)
    .debug_selector(move || selector.clone())
}

/// Paseo's `emptyCard`: one muted sentence, centred, 16 around.
fn muted_card(text: &'static str, tokens: &Tokens) -> Div {
    settings::card(tokens).child(
        div()
            .p(px(S4))
            .flex()
            .justify_center()
            .text_center()
            .type_(Type::Body)
            .text_color(tokens.neutral.foreground_muted)
            .child(text),
    )
}

fn rows(
    shell: &Shell,
    list: &Profiles,
    saving: bool,
    compact: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let final_index = list.entries().len().saturating_sub(1);
    list.entries()
        .iter()
        .enumerate()
        .fold(settings::card(tokens), |card, (index, profile)| {
            card.child(row(
                shell,
                profile,
                index,
                final_index,
                saving,
                compact,
                tokens,
                cx,
            ))
        })
}

#[expect(
    clippy::too_many_arguments,
    reason = "a row's place in the list and the page's state, as its one caller has them"
)]
fn row(
    shell: &Shell,
    profile: &Profile,
    index: usize,
    last: usize,
    saving: bool,
    compact: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.words.terminal_profiles();
    let id = profile.id().clone();
    let key = id.as_str().to_owned();
    let content = row_content(profile, tokens);
    let open = !saving;
    let up = icon_button(
        format!("terminal-profile-move-up-{key}"),
        ButtonIcon::muted(Glyph::ArrowUp),
        words.move_up(),
        open && index > 0,
        tokens,
    )
    .when(open && index > 0, |button| {
        button.on_click(cx.listener(move |shell, _event, _window, cx| {
            shell.terminal_profile_move(index, Step::Up, cx);
        }))
    });
    let down = icon_button(
        format!("terminal-profile-move-down-{key}"),
        ButtonIcon::muted(Glyph::ArrowDown),
        words.move_down(),
        open && index < last,
        tokens,
    )
    .when(open && index < last, |button| {
        button.on_click(cx.listener(move |shell, _event, _window, cx| {
            shell.terminal_profile_move(index, Step::Down, cx);
        }))
    });
    let edit_id = id.clone();
    let edit = icon_button(
        format!("terminal-profile-edit-{key}"),
        ButtonIcon::muted(Glyph::Pencil),
        words.edit(),
        open,
        tokens,
    )
    .when(open, |button| {
        button.on_click(cx.listener(move |shell, _event, window, cx| {
            shell.terminal_profile_edit(&edit_id, window, cx);
        }))
    });
    let remove = icon_button(
        format!("terminal-profile-remove-{key}"),
        ButtonIcon::destructive(Glyph::Trash2),
        words.remove(),
        open,
        tokens,
    )
    .when(open, |button| {
        button.on_click(cx.listener(move |shell, _event, window, cx| {
            shell.terminal_profile_remove_ask(id.clone(), window, cx);
        }))
    });
    let actions = div()
        .flex_none()
        .flex()
        .flex_row()
        .items_center()
        .child(up)
        .child(down)
        .child(edit)
        .child(remove);
    settings::row_with_content(
        content.into_any_element(),
        Some(actions.into_any_element()),
        index == 0,
        compact,
        tokens,
    )
    .min_h(px(ROW_MIN))
    .debug_selector(move || format!("terminal-profile-row-{key}"))
}

/// A row's mark (else the terminal glyph), name and command line.
fn row_content(profile: &Profile, tokens: &Tokens) -> Div {
    let neutral = tokens.neutral;
    let mark = AgentMark::of_profile(profile).map_or_else(
        || {
            div().child(glyph(
                Glyph::SquareTerminal,
                icon::MD,
                neutral.foreground_muted,
            ))
        },
        |mark| crate::agent_mark::mark_muted_of(mark, Size::Sixteen, Space::Compact, tokens),
    );
    div()
        .flex_1()
        .min_w(px(S0))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(S2))
        .child(
            div()
                .flex_none()
                .w(px(icon::MD))
                .flex()
                .justify_center()
                .child(mark),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(S0))
                .flex()
                .flex_col()
                .child(
                    div()
                        .truncate()
                        .type_(Type::Body)
                        .text_color(neutral.foreground)
                        .child(SharedString::from(profile.name().as_str().to_owned())),
                )
                .child(
                    div()
                        .mt(px(S1))
                        .truncate()
                        .type_(Type::Meta)
                        .text_color(neutral.foreground_muted)
                        .child(command_line(profile)),
                ),
        )
}
