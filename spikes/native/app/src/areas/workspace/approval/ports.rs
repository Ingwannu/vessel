//! Approved allocation declarations are not observed ports or runtime grants.
use super::{code, line};
use vessel_app_kit::{
    Tokens, Type, TypeExt as _,
    ui::{Div, ParentElement as _, Styled as _, div},
};
use vessel_app_view::localization::Translator;
use vessel_app_view::workspace::allocation_changes::{AllocationFields, AllocationLine};
use vessel_app_view::workspace::command_changes::Change;
use vessel_app_view::workspace::words::WorkspaceWords;
use vessel_proto::workspace::ServicePortsView;

pub(super) fn render(
    translator: &Translator,
    words: &dyn WorkspaceWords,
    allocation: &AllocationLine,
    tokens: &Tokens,
) -> Div {
    let (change, current, previous) = match allocation {
        AllocationLine::None => return div(),
        AllocationLine::Same(fields) => (Some(Change::Same), Some(fields), None),
        AllocationLine::Added(fields) => (Some(Change::Added), Some(fields), None),
        AllocationLine::Removed(fields) => (Some(Change::Removed), None, Some(fields)),
        AllocationLine::Changed { now, was } => (None, Some(now), Some(was)),
    };
    let text = translator.approval();
    let mut body = definition(
        translator,
        text.allocation_current(),
        current,
        false,
        tokens,
    );
    if let Some(previous) = previous {
        body = body.child(definition(
            translator,
            text.allocation_previous(),
            Some(previous),
            true,
            tokens,
        ));
    }
    div()
        .flex()
        .flex_col()
        .child(
            div()
                .type_(Type::Kicker)
                .text_color(tokens.neutral.foreground_muted)
                .child(text.service_ports()),
        )
        .child(line(words, change, body, tokens))
        .child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(text.allocation_declaration()),
        )
}

fn definition(
    translator: &Translator,
    title: &'static str,
    fields: Option<&AllocationFields>,
    removed: bool,
    tokens: &Tokens,
) -> Div {
    let text = translator.approval();
    let mut body = div()
        .flex()
        .flex_col()
        .child(div().type_(Type::Label).child(title));
    let Some(fields) = fields else {
        return body.child(div().type_(Type::Body).child(text.allocation_absent()));
    };
    // Every declared choice must be visible even when another choice takes precedence.
    let ServicePortsView { range, port_script } = fields.view();
    let range = range.map_or_else(
        || text.allocation_not_declared().to_owned(),
        |range| range.to_string(),
    );
    let program = port_script.map_or(text.allocation_not_declared(), |program| program.as_str());
    body = body
        .child(div().type_(Type::Meta).child(text.allocation_range()))
        .child(code(range, removed, tokens));
    body.child(div().type_(Type::Meta).child(text.allocation_program()))
        .child(code(program.to_owned(), removed, tokens))
}
