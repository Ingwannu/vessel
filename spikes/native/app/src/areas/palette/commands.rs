//! Existing command result presentation; transcript results have their own scope.

use super::rows::{glyph_of, ids, keys_of, subtitle};
use crate::shell::Shell;
use vessel_app_kit::Tokens;
use vessel_app_kit::command::{
    CommandRowParts, CommandText, command_empty, command_heading, command_row_parts,
};
use vessel_app_kit::ui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    SharedString, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_view::palette::{PaletteTarget, marks, palette_groups};

pub(super) fn render(
    shell: &mut Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let compact = f32::from(window.viewport_size().width) < vessel_app_kit::picker::COMPACT;
    let all = shell.palette_all();
    let words = shell.t.lang.palette();
    let theme = shell.prefs.settings.appearance.theme;
    let query = shell.areas.palette.query.as_str();
    let groups = palette_groups(&all, query);
    let ids = ids(&groups);
    let active = shell.areas.palette.cursor.settle(&ids);
    let palette = &shell.areas.palette;
    if groups.is_empty() {
        command_empty(words.nothing, tokens).into_any_element()
    } else {
        let mut children: Vec<AnyElement> = Vec::new();
        for (at, (group, entries)) in groups.iter().enumerate() {
            children.push(command_heading(words.group(*group), at == 0, tokens).into_any_element());
            for entry in entries {
                let row = CommandRowParts {
                    glyph: Some(glyph_of(&entry.target)),
                    title: CommandText::new(entry.label.clone(), marks(query, &entry.label)),
                    subtitle: entry.detail.as_ref().map(|detail| subtitle(detail, query)),
                    keys: keys_of(&entry.target, shell),
                    checked: matches!(entry.target, PaletteTarget::Theme(shown) if shown == theme),
                    active: active == Some(entry.id.as_str()),
                };
                let target = entry.target.clone();
                let id = SharedString::from(format!("palette-{}", entry.id));
                let arrived = row.active;
                let drawn = command_row_parts(id.clone(), row, compact, tokens).on_click(
                    cx.listener(move |shell, _event, window, cx| {
                        if shell.areas.palette.motion.is_open()
                            && shell.areas.palette.mode
                                == vessel_app_view::palette::PaletteMode::Commands
                        {
                            shell.run_palette(target.clone(), window, cx);
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
        div()
            .id("palette-list")
            .flex_1()
            .min_h(px(vessel_app_kit::theme::spacing::S0))
            .overflow_y_scroll()
            .track_scroll(&palette.scroll)
            .children(children)
            .into_any_element()
    }
}
