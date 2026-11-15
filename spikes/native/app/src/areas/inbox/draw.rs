//! Attention uses the kit's shared screen header, buttons and scroll frame.
//! Narrow windows stack the selected form above the list so an answer never
//! ends up below hundreds of rows; wide windows keep both visible together.
use super::place::InboxPlace;
use crate::shell::Shell;
use vessel_app_kit::theme::spacing::{S0, S2, S3, S4};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, ElementId, FluentBuilder as _, InteractiveElement as _,
    IntoElement as _, ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window,
    div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, breakpoint, outline_button, screen_header};
use vessel_app_view::inbox::{Entry, Listing, Operation, Read, words};

pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let narrow = breakpoint::phone(window);
    let labels = shell.words.inbox();
    let refresh = outline_button("inbox-refresh", labels.refresh(), true, tokens)
        .on_click(cx.listener(|shell, _event, _window, cx| {
            shell.inbox_refresh();
            cx.notify();
        }))
        .into_any_element();
    let header = screen_header(labels.title().to_owned(), vec![refresh], tokens, narrow);
    let list = list(shell, tokens, cx);
    let detail = detail(shell, tokens, window, cx);
    let content = div().flex().flex_1().min_h(px(S0)).min_w(px(S0));
    let content = if narrow {
        content.flex_col().child(detail).child(list)
    } else {
        content.flex_row().child(list).child(detail)
    };
    div()
        .size_full()
        .flex()
        .flex_col()
        .min_w(px(S0))
        .bg(tokens.neutral.surface0)
        .child(header)
        .child(content)
        .into_any_element()
}

fn list(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> AnyElement {
    let area = &shell.areas.inbox;
    let labels = shell.words.inbox();
    let mut rows: Vec<_> = area
        .store
        .nodes()
        .iter()
        .flat_map(|node| {
            node.snapshot.iter().flat_map(move |snapshot| {
                snapshot
                    .entries
                    .iter()
                    .map(move |entry| (&node.source, entry))
            })
        })
        .collect();
    rows.sort_by(|(_, a), (_, b)| {
        b.content
            .waiting()
            .cmp(&a.content.waiting())
            .then_with(|| a.at.cmp(&b.at))
            .then_with(|| a.key.seq.cmp(&b.key.seq))
    });
    let mut body = div()
        .id("inbox-list")
        .flex()
        .flex_col()
        .gap(px(S2))
        .flex_1()
        .min_h(px(S0))
        .min_w(px(S0))
        .p(px(S3))
        .overflow_y_scroll();
    if let Some(problem) = area.problem {
        body = body.child(note(words::problem(problem, shell.words), tokens));
    }
    body = machine_notes(body, shell, tokens);
    if area.coalescing == vessel_app_view::inbox::notices::Coalescing::Combined {
        body = body.child(note(labels.combined(), tokens));
    }
    if let Some(problem) = area.read_problem {
        body = body.child(note(words::problem(problem, shell.words), tokens));
    }
    if let Some((_, problem)) = area.presence_problems.first() {
        body = body
            .child(note(labels.presence_failed(), tokens))
            .child(note(words::problem(*problem, shell.words), tokens));
    }
    if rows.is_empty() {
        let loading = area
            .store
            .nodes()
            .iter()
            .any(|node| node.listing == Listing::Loading);
        body = body.child(note(
            if loading {
                labels.loading()
            } else {
                labels.empty()
            },
            tokens,
        ));
    }
    for (source, entry) in rows {
        body = body.child(row(source.name().as_str(), entry, shell, tokens, cx));
    }
    body = body.child(more_buttons(shell, tokens, cx));
    body = body.child(super::terminal_rows::render(shell, tokens, cx));
    body.into_any_element()
}

fn row(
    machine: &str,
    entry: &Entry,
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let key = entry.key;
    let title = entry
        .title
        .as_ref()
        .map_or_else(|| shell.words.inbox().untitled(), |title| title.as_str());
    let background = if shell.areas.inbox.detail.key() == Some(key) {
        tokens.neutral.interaction_highlight
    } else {
        tokens.neutral.surface1
    };
    let wash = tokens.neutral.interaction_highlight;
    let selected = div()
        .id(ElementId::Name(
            format!("inbox-{}-{}-{}", key.node, key.session, key.seq).into(),
        ))
        .flex()
        .flex_col()
        .gap(px(S2))
        .min_w(px(S0))
        .p(px(S3))
        .rounded(px(vessel_app_kit::theme::radius::R2))
        .bg(background)
        .border_1()
        .border_color(tokens.neutral.border)
        .cursor_pointer()
        .hover(move |style| style.bg(wash))
        .child(
            div()
                .type_(Type::Label)
                .text_color(tokens.neutral.foreground)
                .child(title.to_owned()),
        )
        .child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(format!(
                    "{machine} · {}",
                    words::kind(&entry.content, shell.words)
                )),
        )
        .children(
            entry
                .project
                .as_ref()
                .map(|project| note(project.as_str(), tokens)),
        )
        .child(note(
            &shell
                .t
                .lang
                .conversation()
                .when(entry.at, chrono::Local::now().fixed_offset()),
            tokens,
        ))
        .when(entry.read == Read::Unread, |row| {
            row.border_color(tokens.accent)
        })
        .on_click(cx.listener(move |shell, _event, _window, cx| {
            shell.inbox_select(key, cx);
            cx.notify();
        }));
    selected.into_any_element()
}

fn detail(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let area = &shell.areas.inbox;
    let labels = shell.words.inbox();
    let mut body = div()
        .id("inbox-detail")
        .flex()
        .flex_col()
        .gap(px(S3))
        .flex_1()
        .min_w(px(S0))
        .min_h(px(S0))
        .p(px(S4))
        .overflow_y_scroll();
    let Some(key) = area.detail.key() else {
        return body.child(note(labels.select(), tokens)).into_any_element();
    };
    let pending = area.detail.pending();
    if let Some(operation) = pending {
        let label = match operation {
            Operation::Answer => labels.answering(),
            Operation::Load | Operation::Mark => labels.confirming(),
        };
        body = body.child(note(label, tokens));
    }
    if let Some(problem) = area.detail.problem() {
        body = body.child(note(words::problem(problem, shell.words), tokens));
    }
    let open = outline_button("inbox-open", labels.open(), true, tokens)
        .on_click(cx.listener(move |shell, _event, window, cx| shell.inbox_open(key, window, cx)));
    body = body.child(open);
    let Some(entry) = area.detail.entry() else {
        return body
            .when(
                pending.is_none() && area.detail.problem().is_none(),
                |body| body.child(note(labels.resolved(), tokens)),
            )
            .into_any_element();
    };
    let next = match entry.read {
        Read::Read => Read::Unread,
        Read::Unread => Read::Read,
    };
    let label = match next {
        Read::Read => labels.read(),
        Read::Unread => labels.unread(),
    };
    let may_mark = pending.is_none() && area.detail.confirmed() && area.store.may_mark(key);
    body = body.child(
        outline_button("inbox-mark", label, may_mark, tokens).on_click(cx.listener(
            move |shell, _event, _window, cx| {
                shell.inbox_mark(next);
                cx.notify();
            },
        )),
    );
    if !area.store.may_answer(key) && entry.content.waiting() {
        body = body.child(note(labels.read_only(), tokens));
    }
    body = body.child(note(words::kind(&entry.content, shell.words), tokens));
    if matches!(
        &entry.content,
        vessel_app_view::inbox::Content::InSession
            | vessel_app_view::inbox::Content::Permission {
                detail: vessel_proto::inbox::InboxDetail::InSession,
                ..
            }
    ) {
        body = body.child(note(labels.in_session(), tokens));
    }
    body.children(crate::areas::conversation::dock::requests::<InboxPlace>(
        shell, tokens, window, cx,
    ))
    .into_any_element()
}

fn note(text: &str, tokens: &Tokens) -> Div {
    div()
        .type_(Type::Body)
        .text_color(tokens.neutral.foreground_muted)
        .child(text.to_owned())
}

fn machine_notes(
    mut body: vessel_app_kit::ui::Stateful<Div>,
    shell: &Shell,
    tokens: &Tokens,
) -> vessel_app_kit::ui::Stateful<Div> {
    let area = &shell.areas.inbox;
    let labels = shell.words.inbox();
    for target in &area.targets {
        if target.node().is_none() {
            body = body.child(note(
                &format!("{} · {}", target.route().as_str(), labels.offline()),
                tokens,
            ));
        }
    }
    for node in area.store.nodes() {
        if node
            .snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.attention.is_none())
        {
            body = body.child(note(
                &format!(
                    "{} · {}",
                    node.source.name().as_str(),
                    labels.badges_legacy()
                ),
                tokens,
            ));
        }
        if let Listing::Failed(problem) = node.listing {
            body = body.child(note(
                &format!(
                    "{} · {}",
                    node.source.name().as_str(),
                    words::problem(problem, shell.words)
                ),
                tokens,
            ));
        }
    }
    body
}

fn more_buttons(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let mut buttons = div().flex().flex_col().gap(px(S2));
    for node in shell.areas.inbox.store.nodes() {
        if node.pages.before().is_some() {
            let id = node.source.node();
            buttons = buttons.child(
                outline_button(
                    ElementId::Name(format!("inbox-newest-{id}").into()),
                    shell.words.inbox().newest(),
                    true,
                    tokens,
                )
                .on_click(cx.listener(move |shell, _event, _window, cx| {
                    if shell.areas.inbox.store.newest(id).is_some() {
                        shell.inbox_refresh();
                    }
                    cx.notify();
                })),
            );
        }
        if node
            .snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.extent == vessel_app_view::inbox::Extent::More)
        {
            let id = node.source.node();
            buttons = buttons.child(
                outline_button(
                    ElementId::Name(format!("inbox-more-{id}").into()),
                    shell.words.inbox().more(),
                    true,
                    tokens,
                )
                .on_click(cx.listener(move |shell, _event, _window, cx| {
                    shell.inbox_more(id);
                    cx.notify();
                })),
            );
        }
    }
    buttons
}
