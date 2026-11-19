//! Settings > Projects' add form (split from `projects.rs` for the file
//! budget) and Add project's folder suggestions under its folder field
//! (ADR 0112; Paseo's `add-project-flow.tsx` directory search): what is
//! typed is asked of the machine (`folder.suggest`) 250 ms after the last
//! key, and its folders come back as Paseo's rows (a folder mark, the path
//! with the home as `~`, the whole path under it); a row chosen adds that
//! folder in the Add a folder form, as Paseo's adds the folder chosen, and
//! fills the field in the others (the parent of a clone, a new folder's
//! place).
//!
//! [Decision Log]
//! - Purpose: Add project finds folders as Paseo's does, on any machine.
//! - Constraint: only a machine with `folder.suggest` is asked (an older one
//!   keeps the plain field); a slower answer for an older query must not
//!   replace a newer one's.
//! - Choice: a generation per key (the latest wins the pause) and the
//!   query kept with its rows: rows are drawn only while the field still
//!   holds the query they answer.
//! - Cost: one call per pause in typing.

use std::path::Path;

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, button, glyph};
use vessel_app_link::workspace::{Ask, ProjectAsk};
use vessel_app_view::workspace::host_folders::{FOLDERS, PAUSE, offered};
use vessel_app_view::workspace::problem::Problem;
use vessel_app_view::workspace::projects::repository_name;
use vessel_app_view::workspace::words::AddForm;
use vessel_proto::workspace::host_folders::{FolderSuggestions, SuggestFolders, SuggestedFolder};
use vessel_proto::workspace::suggestions::{SuggestionLimit, SuggestionQuery};

use crate::shell::Shell;

/// The card's gap between its parts (the form's own, kept from before the
/// split; the kit has no 10 step).
const CARD_GAP: f32 = 10.0;
/// Paseo's row icon slot (`iconSlot`: 18 wide).
const ICON_SLOT: f32 = 18.0;

/// The folder field's suggestions.
#[derive(Default)]
pub(in crate::areas::workspace) struct Suggestions {
    /// The latest key's generation: a pause that ends after another key
    /// asks nothing.
    wait: u64,
    /// The query the rows answer, and the rows.
    query: String,
    folders: Vec<SuggestedFolder>,
}

impl Shell {
    /// The folder field changed: ask after Paseo's pause.
    pub(in crate::areas::workspace) fn add_path_changed(
        &mut self,
        text: String,
        cx: &mut Context<'_, Self>,
    ) {
        let suggestions = &mut self.areas.workspace.page.suggestions;
        suggestions.wait = suggestions.wait.wrapping_add(1);
        let wait = suggestions.wait;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(PAUSE).await;
            let _gone = this.update(cx, |shell, _cx| {
                if shell.areas.workspace.page.suggestions.wait == wait {
                    shell.suggest_folders(text.trim());
                }
            });
        })
        .detach();
    }

    fn suggest_folders(&mut self, typed: &str) {
        let Some(machine) = self.projects_machine() else {
            return;
        };
        let speaks = self
            .fleet
            .machines
            .iter()
            .find(|known| known.name == machine)
            .and_then(|known| known.protocol);
        if !offered(speaks) || self.areas.workspace.page.adding.is_none() {
            return;
        }
        let Ok(query) = SuggestionQuery::try_from(typed.to_owned()) else {
            return;
        };
        if query.as_str().is_empty() {
            let suggestions = &mut self.areas.workspace.page.suggestions;
            suggestions.query.clear();
            suggestions.folders.clear();
            return;
        }
        self.ask(
            &machine,
            Ask::Project(ProjectAsk::SuggestFolders(SuggestFolders {
                query,
                limit: SuggestionLimit::try_from(FOLDERS).ok(),
            })),
        );
    }

    /// Folders for `query` arrived; kept when it is still the machine the
    /// page shows (a failure shows none: the field still takes a path).
    pub(in crate::areas::workspace) fn folders_suggested(
        &mut self,
        machine: &str,
        query: &str,
        result: Result<FolderSuggestions, Problem>,
    ) {
        if self.projects_machine().as_deref() != Some(machine) {
            return;
        }
        let suggestions = &mut self.areas.workspace.page.suggestions;
        query.clone_into(&mut suggestions.query);
        suggestions.folders = result.map(|found| found.folders).unwrap_or_default();
    }

    /// A row chosen: Add a folder adds it; the other forms take it.
    fn add_folder_chosen(&mut self, path: &Path, window: &mut Window, cx: &mut Context<'_, Self>) {
        let page = &mut self.areas.workspace.page;
        let text = path.display().to_string();
        page.path.set_text(&text, window, cx);
        page.suggestions.folders.clear();
        if page.adding == Some(AddForm::Folder) {
            self.submit_add(cx);
        }
        cx.notify();
    }
}

/// The add form, in place: a clone's URL, the folder with its suggestions,
/// the help line, the node's refusal, Cancel and the form's verb.
pub(super) fn add_form(
    shell: &Shell,
    machine: &str,
    form: AddForm,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.words();
    let labels = words.labels();
    let state = &shell.areas.workspace.page;
    let (heading, verb, folder_label) = match form {
        AddForm::Folder => (labels.title_add_folder, labels.add, labels.folder),
        AddForm::Clone => (labels.title_clone, labels.clone, labels.clone_into),
        AddForm::New => (labels.title_new, labels.create, labels.folder),
    };
    let url = state.url.text(cx);
    let path = state.path.text(cx);
    let folder = path.trim().trim_end_matches(['/', '\\']);
    let name = repository_name(&url);
    let target = name
        .filter(|_| !folder.is_empty())
        .map(|name| (folder, name));
    let ready = !state.busy && !folder.is_empty() && (form != AddForm::Clone || name.is_some());
    let mut card = div()
        .flex()
        .flex_col()
        .gap(px(CARD_GAP))
        .p(px(spacing::S3_5))
        .rounded(px(vessel_app_kit::theme::radius::R4))
        .border_1()
        .border_color(tokens.neutral.border)
        .bg(tokens.neutral.surface1)
        .child(div().type_(Type::Title).child(heading));
    if form == AddForm::Clone {
        let url_field = state.url.element(tokens, compact, cx);
        card = card.child(labelled(labels.repository_url, url_field, tokens));
    }
    let path_field = state.path.element(tokens, compact, cx);
    card = card
        .child(labelled(folder_label, path_field, tokens))
        .children(suggestions(shell, path.trim(), tokens, cx))
        .child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(words.add_hint(form, machine, target)),
        );
    if let Some(error) = &state.error {
        card = card.child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.destructive)
                .child(error.clone()),
        );
    }
    card.child(
        div()
            .flex()
            .flex_row()
            .justify_end()
            .gap(px(spacing::S2))
            .child(
                button("add-cancel", labels.cancel, false, tokens).on_click(cx.listener(
                    |this, _event, _window, cx| {
                        this.areas.workspace.page.adding = None;
                        cx.notify();
                    },
                )),
            )
            .child(
                button("add-submit", verb, true, tokens)
                    .when(!ready, |button| button.opacity(0.5))
                    .on_click(cx.listener(|this, _event, _window, cx| this.submit_add(cx))),
            ),
    )
}

/// Paseo's folder rows for what the field holds: a 16 folder mark in an
/// 18 slot, the path with the home as `~` (14, the ink), the whole path
/// under it when it differs (12, muted, 2 down); 16 in, 8 high, 12 apart;
/// the hovered row one surface above the card (Paseo's rows hover on
/// `surface1` over its `surface0` sheet; this card is `surface1`).
fn suggestions(
    shell: &Shell,
    typed: &str,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let found = &shell.areas.workspace.page.suggestions;
    if found.folders.is_empty() || found.query != typed {
        return None;
    }
    let neutral = tokens.neutral;
    let wash = neutral.surface2;
    let mut list = div().flex().flex_col().py(px(spacing::S2));
    for (index, folder) in found.folders.iter().enumerate() {
        let whole = folder.path.display().to_string();
        let path = folder.path.clone();
        let row = div()
            .id(("add-folder", index))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(spacing::S3))
            .px(px(spacing::S4))
            .py(px(spacing::S2))
            .cursor_pointer()
            .hover(move |style| style.bg(wash))
            .child(
                div()
                    .w(px(ICON_SLOT))
                    .flex_none()
                    .flex()
                    .justify_center()
                    .child(glyph(Glyph::Folder, spacing::S4, neutral.foreground_muted)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .truncate()
                            .type_(Type::Body)
                            .text_color(neutral.foreground)
                            .child(folder.shown.clone()),
                    )
                    .when(whole != folder.shown, |text| {
                        text.child(
                            div()
                                .mt(px(spacing::S0_5))
                                .truncate()
                                .type_(Type::Meta)
                                .text_color(neutral.foreground_muted)
                                .child(whole),
                        )
                    }),
            )
            .on_click(cx.listener(move |this, _event, window, cx| {
                this.add_folder_chosen(&path, window, cx);
            }));
        list = list.child(row);
    }
    Some(list.into_any_element())
}

/// A field with its label above it.
fn labelled(
    label: &'static str,
    field: impl vessel_app_kit::ui::IntoElement,
    tokens: &Tokens,
) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(spacing::S1))
        .child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(label),
        )
        .child(field)
}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests;
