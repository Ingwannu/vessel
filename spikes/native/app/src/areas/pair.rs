//! Adding a machine by pairing (ADR 0010), in two steps a person cannot
//! mix up (the owner typed the key into the name field of build
//! 0.0.0.0.1, whose screen read as if it asked for it):
//!
//! 1. The link: pasted, decoded, cleared from its field (it is a
//!    single-use secret).
//! 2. The key shown large, with nothing to type, and one sentence: it
//!    must be the same as the key the machine printed with the link. Then
//!    the name, under a label that stays, filled in with a name the book
//!    does not have, so Pair works at once. A name that looks like the key
//!    or a link is called that, beside the field, as it is typed.
//!
//! Errors are shown beside the control they are about, not in the frame's
//! foot. The app reaches the machine afterwards as the CLI does, directly
//! or through its relay, with the device credential it stored.

use vessel_app_kit::ui::{
    Context, Div, ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{TextField, Tokens, Type, TypeExt as _, button, kicker};
use vessel_app_link::{Command, NameProblem, Offer, default_name, name_problem, read_link};

use super::common::title_bar;
use crate::routes::Route;
use crate::shell::Shell;

/// The pairing area's state.
pub(crate) struct Pairing {
    link: TextField,
    name: TextField,
    offer: Option<Offer>,
    link_error: Option<String>,
    name_problem: Option<NameProblem>,
    pair_error: Option<String>,
    busy: bool,
    /// The name the app filled in; kept as it is, the node's own name is
    /// used instead once pairing says it.
    prefilled: Option<String>,
}

impl Pairing {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            link: TextField::new(
                "",
                |shell: &mut Shell, text, window, cx| shell.read_offer(&text, window, cx),
                window,
                cx,
            ),
            name: TextField::with_change(
                "",
                |shell: &mut Shell, text, _window, cx| shell.pair(text, cx),
                |shell: &mut Shell, text, cx| {
                    shell.areas.pair.name_problem = name_problem(&text, &shell.book_names());
                    shell.areas.pair.pair_error = None;
                    cx.notify();
                },
                window,
                cx,
            ),
            offer: None,
            link_error: None,
            name_problem: None,
            pair_error: None,
            busy: false,
            prefilled: None,
        }
    }
}

/// Focus the link's field.
pub(crate) fn arrive(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    let t = shell.t;
    shell
        .areas
        .pair
        .link
        .set_placeholder(t.link_placeholder, window, cx);
    shell
        .areas
        .pair
        .name
        .set_placeholder(t.name_hint, window, cx);
    shell.areas.pair.link.focus(window, cx);
}

impl Shell {
    pub(crate) fn pairing_opened(&mut self, cx: &mut Context<'_, Self>) {
        self.areas.pair.pair_error = None;
        cx.notify();
    }

    fn book_names(&self) -> Vec<String> {
        self.fleet
            .machines
            .iter()
            .map(|machine| machine.name.clone())
            .collect()
    }

    fn read_offer(&mut self, text: &str, window: &mut Window, cx: &mut Context<'_, Self>) {
        match read_link(text) {
            Ok(offer) => {
                let pair = &mut self.areas.pair;
                pair.offer = Some(offer);
                pair.link_error = None;
                pair.pair_error = None;
                pair.name_problem = None;
                // The link is held only as the offer, not left on screen.
                pair.link.clear(window, cx);
                let name = default_name(&self.book_names());
                self.areas.pair.name.set_text(&name, window, cx);
                self.areas.pair.prefilled = Some(name);
                self.areas.pair.name.focus(window, cx);
            }
            Err(error) => self.areas.pair.link_error = Some(error.to_string()),
        }
        cx.notify();
    }

    fn pair(&mut self, name: String, cx: &mut Context<'_, Self>) {
        let pair = &mut self.areas.pair;
        if pair.busy {
            return;
        }
        let Some(offer) = pair.offer.clone() else {
            pair.link_error = Some(self.t.paste_first.to_owned());
            cx.notify();
            return;
        };
        if let Some(problem) = name_problem(&name, &self.book_names()) {
            self.areas.pair.name_problem = Some(problem);
            cx.notify();
            return;
        }
        let pair = &mut self.areas.pair;
        pair.busy = true;
        pair.pair_error = None;
        let prefer_node_name = pair.prefilled.as_deref() == Some(name.trim());
        self.command(Command::Pair {
            offer,
            name,
            prefer_node_name,
        });
        cx.notify();
    }

    fn pair_from_field(&mut self, cx: &mut Context<'_, Self>) {
        let name = self.areas.pair.name.text(cx);
        self.pair(name, cx);
    }

    /// Pairing worked: the draft next, to start working there.
    pub(crate) fn paired(&mut self, node: &str, route: &str, cx: &mut Context<'_, Self>) {
        let pair = &mut self.areas.pair;
        pair.busy = false;
        pair.offer = None;
        self.notice = Some(self.t.paired(node, route));
        self.route_to(Route::Draft);
        self.settle_draft();
        cx.notify();
    }

    pub(crate) fn pair_failed(&mut self, why: String) {
        let pair = &mut self.areas.pair;
        pair.busy = false;
        pair.pair_error = Some(why);
    }
}

pub(crate) fn render(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let t = shell.t;
    let pair = &shell.areas.pair;
    let mut form = div()
        .flex()
        .flex_col()
        .gap(px(14.0))
        .p(px(24.0))
        .max_w(px(640.0))
        .child(vessel_app_kit::mark(40.0, tokens))
        .child(
            div()
                .type_(Type::Body)
                .text_color(tokens.text_secondary)
                .child(t.pair_intro),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .child(kicker(t.pairing_link, tokens))
                .child(pair.link.element())
                .children(pair.link_error.clone().map(|error| beside(error, tokens))),
        );
    if let Some(offer) = &pair.offer {
        form = form.child(key_block(offer, shell, tokens)).child(
            div()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .child(kicker(t.name_label, tokens))
                .child(pair.name.element())
                .child(match pair.name_problem {
                    Some(problem) => beside(problem_words(problem, shell).to_owned(), tokens),
                    None => div()
                        .type_(Type::Meta)
                        .text_color(tokens.text_tertiary)
                        .child(t.name_hint),
                }),
        );
        form = form.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(12.0))
                .child(
                    button(
                        "pair",
                        if pair.busy { t.pairing } else { t.pair },
                        true,
                        tokens,
                    )
                    .on_click(cx.listener(|this, _event, _window, cx| this.pair_from_field(cx))),
                )
                .children(pair.pair_error.clone().map(|error| beside(error, tokens))),
        );
    }
    div()
        .size_full()
        .flex()
        .flex_col()
        .child(title_bar(t.add_machine.to_owned(), None, tokens))
        .child(form)
}

/// The key, large, in its groups, with the one sentence about it, and the
/// link's other facts small under it. Nothing to type here.
fn key_block(offer: &Offer, shell: &Shell, tokens: &Tokens) -> Div {
    let t = shell.t;
    div()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .p(px(14.0))
        .bg(tokens.bg_well)
        .rounded(px(vessel_app_kit::theme::radius::R4))
        .child(kicker(t.key, tokens))
        .child(
            div()
                .type_(Type::Mono)
                .text_size(px(20.0))
                .line_height(px(28.0))
                .text_color(tokens.text_primary)
                .child(offer.key.clone()),
        )
        .child(
            div()
                .type_(Type::Body)
                .text_color(tokens.text_secondary)
                .child(t.key_check),
        )
        .child(
            div()
                .type_(Type::Tiny)
                .text_color(tokens.text_tertiary)
                .child(format!(
                    "{} · {} · {}",
                    offer.node,
                    offer.routes.join(", "),
                    offer.expires
                )),
        )
}

const fn problem_words(problem: NameProblem, shell: &Shell) -> &'static str {
    let t = shell.t;
    match problem {
        NameProblem::Empty => t.name_empty,
        NameProblem::LooksLikeKey => t.name_is_key,
        NameProblem::LooksLikeLink => t.name_is_link,
        NameProblem::Invalid => t.name_invalid,
        NameProblem::Taken => t.name_taken,
    }
}

/// An error, beside the control it is about.
fn beside(text: String, tokens: &Tokens) -> Div {
    div()
        .type_(Type::Meta)
        .text_color(tokens.danger)
        .child(text)
}
