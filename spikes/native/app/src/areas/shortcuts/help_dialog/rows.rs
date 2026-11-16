//! Help lists the same effective sequences as the engine. Query matching uses
//! the pure aliases projection; there is no parallel list of default key text.

use crate::shell::Shell;
use vessel_app_kit::command::{command_empty, command_heading, key_badges};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, InteractiveElement as _, IntoElement as _, ParentElement as _, SharedString,
    StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _};
use vessel_app_view::shortcuts::help::{HelpRow, HelpSection, filter_help};
use vessel_app_view::shortcuts::{
    Action, Platform, ShortcutGroup, action_label, binding_id, default_context, effective_bindings,
    group_label,
};

pub(super) fn render(shell: &Shell, tokens: &Tokens) -> AnyElement {
    let platform = Platform::native();
    let context = default_context(platform);
    let Ok(bindings) = effective_bindings(&shell.prefs.settings.shortcuts, context) else {
        return command_empty(shell.words.shortcuts().nothing(), tokens).into_any_element();
    };
    let sections: Vec<_> = ShortcutGroup::ALL
        .into_iter()
        .map(|group| HelpSection {
            group,
            title: group_label(group, shell.words),
            rows: Action::in_group(group)
                .filter_map(|action| {
                    let id = binding_id(action, context)?;
                    Some(HelpRow {
                        action,
                        label: action_label(action, shell.words),
                        note: (action == Action::Help).then(|| shell.words.shortcuts().note_help()),
                        keys: bindings
                            .iter()
                            .find(|binding| binding.id == id)
                            .map(|binding| &binding.keys),
                    })
                })
                .collect(),
        })
        .collect();
    let visible = filter_help(&sections, &shell.areas.shortcuts.help.query, platform);
    let mut list = div()
        .id("shortcut-help-results")
        .flex_1()
        .min_h(px(spacing::S0))
        .overflow_y_scroll()
        .track_scroll(&shell.areas.shortcuts.help.scroll)
        .flex()
        .flex_col();
    if visible.is_empty() {
        return list
            .child(command_empty(shell.words.shortcuts().nothing(), tokens))
            .into_any_element();
    }
    for (at, section) in visible.into_iter().enumerate() {
        list = list.child(command_heading(section.title, at == 0, tokens));
        for row in section.rows {
            let keys = row.keys.map_or_else(
                || {
                    div()
                        .type_(Type::Meta)
                        .text_color(tokens.neutral.foreground_muted)
                        .child(shell.words.shortcuts().unassigned())
                        .into_any_element()
                },
                |sequence| {
                    div()
                        .flex_wrap()
                        .flex()
                        .items_center()
                        .gap(px(spacing::S2))
                        .children(
                            vessel_app_view::shortcuts::help::display_caps(
                                row.action,
                                sequence,
                                vessel_app_view::shortcuts::modified(
                                    row.action,
                                    &shell.prefs.settings.shortcuts,
                                ),
                                platform,
                            )
                            .into_iter()
                            .map(|caps| {
                                let caps: Vec<SharedString> =
                                    caps.into_iter().map(SharedString::from).collect();
                                key_badges(&caps, tokens).into_any_element()
                            }),
                        )
                        .into_any_element()
                },
            );
            list = list.child(vessel_app_kit::settings::row(
                row.label,
                row.note.map(SharedString::from),
                Some(keys),
                true,
                tokens,
            ));
        }
    }
    list.into_any_element()
}
