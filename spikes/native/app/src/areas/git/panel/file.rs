//! One file section: existing header, immutable document and lazy admission.
//! A viewport observation schedules work; it never substitutes for its ticket.
use super::Entry;
use super::bodies::{FileActivation, FileDemand, Reveal};
use super::document::{Document, Words, caption};
use crate::shell::Shell;
use vessel_app_kit::Tokens;
use vessel_app_kit::ui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, ParentElement as _, Pixels,
    StatefulInteractiveElement as _, Styled as _, Window, div,
};
use vessel_app_view::workspace::git::panel::PanelAddress;
use vessel_proto::workspace::RelPath;

struct Section<'a> {
    address: &'a PanelAddress,
    path: &'a RelPath,
    index: usize,
    width: Pixels,
}

pub(super) fn row(
    shell: &mut Shell,
    address: &PanelAddress,
    index: usize,
    width: Pixels,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let header = super::draw::file_row(shell, address, index, tokens, window);
    let Some(path) = shell
        .areas
        .git
        .panels
        .entries
        .get(address)
        .and_then(|entry| {
            entry
                .reads
                .as_ref()
                .and_then(|reads| reads.summary().shown())
        })
        .and_then(|summary| summary.files.get(index))
        .map(|file| file.path.clone())
    else {
        return header;
    };
    shell.areas.git.panels.touch_body(address, &path);
    let legacy = shell.words();
    let words = shell.words;
    let Some(entry) = shell.areas.git.panels.entries.get_mut(address) else {
        return header;
    };
    let captured = FileActivation::capture(entry, address, &path, FileDemand::Visible);
    let body = document(
        entry,
        Words {
            catalog: words,
            legacy,
        },
        &Section {
            address,
            path: &path,
            index,
            width,
        },
        tokens,
        window,
        cx,
    );
    let action = actions(
        entry,
        address,
        &path,
        Words {
            catalog: words,
            legacy,
        },
        tokens,
        cx,
    );
    let owner = cx.entity().downgrade();
    div()
        .w_full()
        .child(
            div()
                .w_full()
                .flex()
                .flex_col()
                .child(header)
                .child(body)
                .children(action),
        )
        .on_children_prepainted(move |children, window, cx| {
            let Some(captured) = captured.clone() else {
                return;
            };
            let Some(bounds) = children.first() else {
                return;
            };
            let visible = bounds.intersect(&window.content_mask().bounds);
            if visible.size.width <= Pixels::default() || visible.size.height <= Pixels::default() {
                return;
            }
            let owner = owner.clone();
            window.defer(cx, move |window, cx| {
                let _updated =
                    owner.update(cx, |shell, cx| shell.git_panel_file(&captured, window, cx));
            });
        })
        .id(("git-file-section", index))
        .into_any_element()
}

fn document(
    entry: &mut Entry,
    words: Words<'_>,
    section: &Section<'_>,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let Some(reads) = entry.reads.as_ref() else {
        return div().into_any_element();
    };
    if let Some(vessel_app_view::workspace::git::ReadState::Failed(why)) = reads.file(section.path)
    {
        // The same shared alert keeps the node's diagnostic beneath its words.
        return super::draw::failure(words.catalog, why, tokens, window, cx);
    }
    let Some(pages) = reads.file(section.path).and_then(|read| read.shown()) else {
        let text = if reads
            .file(section.path)
            .is_some_and(|read| read.outstanding().is_some())
        {
            words
                .legacy
                .file_note(&vessel_app_view::workspace::changes::FileNote::Loading)
        } else {
            words.catalog.git_panel().body_waiting().to_owned()
        };
        return caption(&text, tokens);
    };
    if !entry
        .documents
        .get(section.path)
        .is_some_and(|document| document.matches(pages))
    {
        let (Some(instance), Some(context)) = (entry.instance, entry.context.clone()) else {
            return caption(words.catalog.git_panel().body_waiting(), tokens);
        };
        let reveal = Reveal {
            address: section.address.clone(),
            target: entry.target.clone(),
            instance,
            context,
            ticket: pages.ticket().clone(),
            index: section.index,
        };
        let document = Document::new(pages, reveal, window, cx);
        let _previous = entry.documents.insert(section.path.clone(), document);
    }
    entry.documents.get(section.path).map_or_else(
        || div().into_any_element(),
        |document| document.element(words, tokens, section.width, window, cx),
    )
}

fn actions(
    entry: &Entry,
    address: &PanelAddress,
    path: &RelPath,
    words: Words<'_>,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let (captured, label) = if let Some(captured) =
        FileActivation::capture(entry, address, path, FileDemand::Continue)
    {
        // Keep this one legacy catalog value; no duplicate Show rest string.
        let label = words
            .legacy
            .file_note(&vessel_app_view::workspace::changes::FileNote::More);
        (captured, label)
    } else {
        (
            FileActivation::capture(entry, address, path, FileDemand::Retry)?,
            words.catalog.git_panel().refresh().to_owned(),
        )
    };
    Some(
        vessel_app_kit::button("git-file-read", &label, false, tokens)
            .on_click(cx.listener(move |shell, _, window, cx| {
                shell.git_panel_file(&captured, window, cx);
            }))
            .into_any_element(),
    )
}
