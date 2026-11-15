//! Source-owned and built-in results use the kit's existing command row renderer.
use super::{
    rows::{action_keys, subtitle},
    snapshot,
};
use crate::shell::Shell;
use vessel_app_kit::Tokens;
use vessel_app_kit::command::{
    CommandRowParts, CommandText, command_empty, command_heading, command_row_parts,
};
use vessel_app_kit::ui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    SharedString, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_view::palette::marks;

pub(super) fn render(
    shell: &mut Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let compact = f32::from(window.viewport_size().width) < vessel_app_kit::picker::COMPACT;
    let snapshot = snapshot::project(shell);
    let active = shell.areas.palette.cursor.settle_identity(&snapshot.ids());
    let query = &shell.areas.palette.query;
    let mut children = Vec::new();
    for section in &snapshot.sections {
        if let Some(title) = &section.title {
            children.push(
                command_heading(
                    SharedString::from(title.clone()),
                    children.is_empty(),
                    tokens,
                )
                .into_any_element(),
            );
        }
        for row in &section.rows {
            let parts = CommandRowParts {
                glyph: row.glyph,
                title: CommandText::new(row.title.clone(), marks(query, &row.title)),
                subtitle: row.subtitle.as_ref().map(|detail| subtitle(detail, query)),
                keys: if super::super::shortcuts::available(window) {
                    row.shortcut
                        .map(|action| action_keys(action, shell))
                        .unwrap_or_default()
                } else {
                    Vec::new()
                },
                checked: row.selected,
                active: active == Some(&row.id),
            };
            let arrived = parts.active;
            let target = row.activate.clone();
            let id = SharedString::from(format!("palette-{:?}", row.id));
            let drawn = command_row_parts(id.clone(), parts, compact, tokens).on_click(
                cx.listener(move |shell, _event, window, cx| {
                    if shell.areas.palette.motion.is_open()
                        && shell.areas.palette.mode
                            == vessel_app_view::palette::PaletteMode::Commands
                    {
                        shell.activate_command(target.clone(), window, cx);
                    }
                }),
            );
            children.push(if arrived {
                vessel_app_kit::popup::fill_in(
                    SharedString::from(format!("{id}-arrive")),
                    drawn,
                    tokens.neutral.surface1,
                )
                .into_any_element()
            } else {
                drawn.into_any_element()
            });
        }
    }
    if children.is_empty() {
        return command_empty(shell.t.lang.palette().nothing, tokens).into_any_element();
    }
    div()
        .id("palette-list")
        .flex_1()
        .min_h(px(vessel_app_kit::theme::spacing::S0))
        .overflow_y_scroll()
        .track_scroll(&shell.areas.palette.scroll)
        .children(children)
        .into_any_element()
}
