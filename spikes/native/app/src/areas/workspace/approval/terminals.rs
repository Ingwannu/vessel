//! Complete terminal definitions; display support alone does not enable approval.
use super::{code, line};
use vessel_app_kit::{
    Tokens, Type, TypeExt as _,
    ui::{Div, ParentElement as _, Styled as _, div},
};
use vessel_app_view::localization::Translator;
use vessel_app_view::workspace::command_changes::Change;
use vessel_app_view::workspace::terminal_changes::TerminalLine;
use vessel_app_view::workspace::words::WorkspaceWords;
use vessel_proto::workspace::FileTerminal;

pub(super) fn render(
    translator: &Translator,
    words: &dyn WorkspaceWords,
    lines: &[TerminalLine],
    tokens: &Tokens,
) -> Div {
    let mut list = div().flex().flex_col().child(
        div()
            .type_(Type::Kicker)
            .text_color(tokens.neutral.foreground_muted)
            .child(translator.scripts().automatic_terminals().to_owned()),
    );
    for entry in lines {
        let (change, removed) = match entry {
            TerminalLine::Line {
                position: _,
                change,
                terminal: _,
            } => (Some(*change), *change == Change::Removed),
            TerminalLine::Changed {
                position: _,
                terminal: _,
                was: _,
            } => (None, false),
        };
        let mut text = definition(translator, entry.terminal(), removed, tokens);
        if let TerminalLine::Changed {
            position: _,
            terminal: _,
            was,
        } = entry
        {
            text = text.child(definition(translator, was, true, tokens));
        }
        list = list.child(line(
            words,
            change,
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .type_(Type::Meta)
                        .child(entry.position().ordinal().to_string()),
                )
                .child(text),
            tokens,
        ));
    }
    list
}

fn definition(
    translator: &Translator,
    terminal: &FileTerminal,
    removed: bool,
    tokens: &Tokens,
) -> Div {
    // Every owner field named: a newly approved terminal field breaks the display.
    let FileTerminal { name, command } = terminal;
    let name = name
        .as_ref()
        .map_or(translator.scripts().terminal_name_absent(), |name| {
            name.as_str()
        });
    div()
        .flex()
        .flex_col()
        .child(code(name.to_owned(), removed, tokens))
        .child(code(command.to_string(), removed, tokens))
}
