//! The Add project sheet drawn (the page's `add-project-results.tsx` and
//! `add-project-flow.tsx`): under the field, in Paseo's order, the new
//! directory's path, the progress while a call runs (nothing else then),
//! the page's error, the search's error, "Loading...", the rows, and the
//! page's empty words.

use std::collections::HashMap;

use vessel_app_kit::Tokens;
use vessel_app_kit::agent_mark::{Forge, Size, forge_mark};
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::flow_sheet::{
    FlowHandlers, FlowHint, FlowRow, FlowSheetFrame, flow_error, flow_preview, flow_row,
    flow_sheet, flow_state,
};
use vessel_app_kit::glyph;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, IntoElement as _, SharedString, StatefulInteractiveElement as _, Window,
};
use vessel_app_view::workspace::add_flow::model::{PageId, PageKind};
use vessel_app_view::workspace::add_flow::options::join_directory_path;
use vessel_app_view::workspace::add_flow::paths::shorten_path;
use vessel_app_view::workspace::add_flow::rows::{
    Inputs, Row, RowMark, empty, rows, searches_directories,
};
use vessel_app_view::workspace::host_folders::offered;
use vessel_app_view::workspace::projects::ProjectsView;
use vessel_app_view::workspace::words::add_flow::{AddFlowWords, FileManager};

use super::{Open, reads_tilde};
use crate::shell::Shell;

/// What the field's folder search shows for the page as it is.
struct Search {
    paths: Vec<String>,
    shown: HashMap<String, String>,
    loading: bool,
    failed: bool,
}

/// The folders answer counts only for the machine and text it was asked
/// for (the page's `folders.ts`).
fn search(shell: &Shell, open: &Open) -> Search {
    let page = open.flow.page();
    let machine = page.kind.machine();
    let searches = searches_directories(page.kind.id())
        && machine.is_some_and(|machine| offered(shell.protocol_of(machine)));
    let answer = open.folders.as_ref().filter(|folders| {
        searches
            && Some(folders.machine.as_str()) == machine
            && folders.query.trim() == page.field.trim()
    });
    match answer {
        None => Search {
            paths: Vec::new(),
            shown: HashMap::new(),
            loading: searches,
            failed: false,
        },
        Some(folders) => {
            let found = folders.found.clone().unwrap_or_default();
            Search {
                paths: found.iter().map(|(path, _)| path.clone()).collect(),
                shown: found.into_iter().collect(),
                loading: false,
                failed: folders.found.is_none(),
            }
        }
    }
}

/// Whether the field's folders are still being asked for.
#[cfg(test)]
pub(super) fn loading(shell: &Shell, open: &Open) -> bool {
    search(shell, open).loading
}

fn recent(shell: &Shell, machine: Option<&str>) -> Vec<String> {
    machine
        .and_then(|machine| shell.areas.workspace.projects.get(machine))
        .and_then(ProjectsView::list)
        .map(|list| {
            list.projects
                .iter()
                .map(|project| project.root.display().to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn rows_with(shell: &Shell, open: &Open, found: &Search) -> Vec<Row> {
    let page = open.flow.page();
    let machine = page.kind.machine();
    let recent = recent(shell, machine);
    rows(&Inputs {
        page,
        machines: &open.flow.machines,
        machine: open.flow.machine(),
        recent: &recent,
        suggested: &found.paths,
        shown: &found.shown,
        last_clone_parent: machine.and_then(|machine| {
            shell
                .areas
                .workspace
                .add_flow
                .last_clone_parent
                .get(machine)
                .map(String::as_str)
        }),
        tilde: machine.is_some_and(|machine| reads_tilde(shell.protocol_of(machine))),
    })
}

/// The rows of the page shown (none while the sheet is closed).
pub(super) fn rows_of(shell: &Shell) -> Vec<Row> {
    shell
        .areas
        .workspace
        .add_flow
        .open
        .as_ref()
        .map(|open| rows_with(shell, open, &search(shell, open)))
        .unwrap_or_default()
}

const fn file_manager() -> FileManager {
    if cfg!(target_os = "macos") {
        FileManager::Finder
    } else if cfg!(target_os = "windows") {
        FileManager::FileExplorer
    } else {
        FileManager::Other
    }
}

fn mark(row: &Row, tokens: &Tokens) -> AnyElement {
    let color = tokens.neutral.foreground_muted;
    let which = match row.mark {
        RowMark::Github => {
            return forge_mark(Forge::GitHub, Size::Sixteen, tokens).into_any_element();
        }
        RowMark::Machine => Glyph::Monitor,
        RowMark::Plus => Glyph::Plus,
        RowMark::Search => Glyph::Search,
        RowMark::FolderPlus => Glyph::CommandAddProject,
        // No open-folder or hard-drive glyph: the folder (plan
        // difference 12).
        RowMark::FolderOpen | RowMark::Folder | RowMark::Drive => Glyph::Folder,
    };
    glyph(which, spacing::S4, color).into_any_element()
}

/// The footer's keys, as Paseo's `formatShortcut` writes them here.
fn hints(words: &AddFlowWords, depth: usize) -> Vec<FlowHint> {
    let arrows = if cfg!(target_os = "macos") {
        "\u{2191}\u{2193}"
    } else {
        "\u{2191}+\u{2193}"
    };
    vec![
        FlowHint {
            keys: arrows.into(),
            action: words.navigate.into(),
        },
        FlowHint {
            keys: "\u{23ce}".into(),
            action: words.select.into(),
        },
        FlowHint {
            keys: "Esc".into(),
            action: if depth > 1 { words.back } else { words.close }.into(),
        },
    ]
}

fn results(
    shell: &Shell,
    open: &Open,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Vec<AnyElement> {
    let words = shell.t.lang.add_flow();
    let page = open.flow.page();
    let found = search(shell, open);
    let mut out: Vec<AnyElement> = Vec::new();
    if let PageKind::NewDirectoryName { parent, .. } = &page.kind
        && !page.field.trim().is_empty()
    {
        let path = join_directory_path(parent, page.field.trim());
        out.push(flow_preview(shorten_path(&path), tokens).into_any_element());
    }
    if page.submitting {
        out.push(flow_state(words.progress(page.kind.id()), tokens).into_any_element());
        return out;
    }
    if let Some(error) = &page.error {
        out.push(flow_error(error.clone(), tokens).into_any_element());
    }
    if found.failed {
        out.push(flow_error(words.search_failed, tokens).into_any_element());
        return out;
    }
    if found.loading {
        out.push(flow_state(words.loading, tokens).into_any_element());
        return out;
    }
    let listed = rows_with(shell, open, &found);
    let active = page.active.min(listed.len().saturating_sub(1));
    if listed.is_empty() && page.kind.id() != PageId::NewDirectoryName {
        let said = words.empty(empty(page.kind.id(), open.flow.machine()));
        out.push(flow_state(said, tokens).into_any_element());
    }
    let manager = file_manager();
    for (index, row) in listed.into_iter().enumerate() {
        let drawn = flow_row(
            SharedString::from(format!("add-flow-row-{}", row.id)),
            FlowRow {
                mark: mark(&row, tokens),
                title: words.title(&row.title).into(),
                subtitle: row
                    .note
                    .as_ref()
                    .map(|note| words.note(note, manager).into()),
                active: index == active,
                disabled: row.disabled,
            },
            tokens,
        );
        let action = row.action.clone();
        let drawn = if row.disabled {
            drawn
        } else {
            drawn.on_click(cx.listener(move |shell, _event, window, cx| {
                shell.add_flow_choose(action.clone(), window, cx);
            }))
        };
        out.push(drawn.into_any_element());
    }
    out
}

/// The sheet over the window, while it is open; a success's move is
/// deferred here, where there is a window.
pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    if shell.areas.workspace.add_flow.after.is_some() {
        cx.defer_in(window, Shell::add_flow_after);
    }
    let sheet = &shell.areas.workspace.add_flow;
    let open = sheet.open.as_ref()?;
    let words = shell.t.lang.add_flow();
    let page = open.flow.page();
    let field_shown = words.placeholder(page.kind.id()).is_some();
    let field = sheet.field.bare(tokens, cx);
    let frame = FlowSheetFrame {
        title: words.page_title(page.kind.id()).into(),
        context: open
            .flow
            .machine()
            .map(|machine| machine.label.clone().into()),
        back: open.flow.depth() > 1,
        field,
        field_shown,
        results: results(shell, open, tokens, cx),
        scroll: sheet.scroll.clone(),
        hints: hints(words, open.flow.depth()),
    };
    Some(flow_sheet(
        frame,
        FlowHandlers {
            on_key: Shell::add_flow_key,
            back: Shell::add_flow_back,
            close: Shell::close_add_flow,
        },
        tokens,
        window,
        cx,
    ))
}
