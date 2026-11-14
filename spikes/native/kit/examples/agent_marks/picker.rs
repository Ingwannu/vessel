//! The real kit picker with `OpenCode` at both ends, so captures exercise
//! its existing clipping, padding, selection, hover and scrolling.
use super::Preview;
use gpui::{AnyElement, Context, ParentElement as _, StatefulInteractiveElement as _, Window};
use std::rc::Rc;
use vessel_app_kit::{
    ComboHandlers, ComboHeader, ComboRow, Leading, Side, Tokens, agent_mark, combobox,
    outline_button,
};

pub(super) fn trigger(
    preview: &Preview,
    tokens: &Tokens,
    cx: &mut Context<'_, Preview>,
) -> gpui::Stateful<gpui::Div> {
    outline_button("mark-picker", "Agent rows / 에이전트 행", true, tokens)
        .child(preview.picker.anchor())
        .on_click(cx.listener(|preview, _, window, cx| {
            preview.picker.open(None, window, cx);
            cx.notify();
        }))
}
pub(super) fn overlay(
    preview: &Preview,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Preview>,
) -> Option<AnyElement> {
    let mut rows = Vec::new();
    for index in 0..22 {
        let (label, art) = match index {
            0 | 21 => ("OpenCode", Some(agent_mark::OPENCODE)),
            1 => ("Claude Code", Some(agent_mark::CLAUDE)),
            2 => ("Codex", Some(agent_mark::CODEX)),
            3 => ("Gemini CLI", Some(agent_mark::GEMINI)),
            4 => ("사용자 추가 / unknown (disabled)", None),
            5 => ("기존 글리프 / glyph (disabled)", None),
            _ => ("한국어와 긴 에이전트 이름 / long custom agent name", None),
        };
        let mut row = ComboRow::new(label);
        row.leading = Some(Leading::AgentMark(art));
        row.checked = preview.selected == index;
        row.disabled = matches!(index, 3..=5);
        if index == 5 {
            row.leading = Some(Leading::Glyph(vessel_app_kit::assets::Glyph::Bot));
        }
        rows.push(row);
    }
    let header = ComboHeader {
        title: preview.header.map(|(_, title)| title.into()),
        back: preview.header.is_some(),
        search: false,
        empty: "".into(),
        note: None,
        side: Side::Above,
    };
    let handlers = ComboHandlers {
        state: |preview: &mut Preview| &mut preview.picker,
        chose: Rc::new(|preview: &mut Preview, index, _, cx| {
            preview.selected = index;
            cx.notify();
        }),
        back: Some(Rc::new(|preview: &mut Preview, _, cx| {
            preview.picker.close();
            cx.notify();
        })),
    };
    combobox(
        "mark-rows",
        &preview.picker,
        &header,
        &rows,
        &handlers,
        360.0,
        tokens,
        window,
        cx,
    )
}
