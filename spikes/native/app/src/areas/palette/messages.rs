//! Message-scope controls and activation over the conversation's existing search.
//! The kit owns drawing primitives; view owns selection, ordering and paging.

use vessel_app_kit::Tokens;
use vessel_app_kit::command::{CommandKey, command_empty};
#[cfg(test)]
use vessel_app_kit::segmented::{SegmentSize, segment, segmented};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    SharedString, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_view::palette::{Direction, PaletteMode};
use vessel_app_view::search::Row;

use crate::areas::conversation::search::{message_row, message_search_note};
use crate::shell::Shell;

/// Keep the failure typed so an open palette can change its language.
pub(super) struct Unavailable;

impl Shell {
    /// Changing scope retains text but cancels the previous search and selection.
    #[cfg(test)]
    pub(super) fn palette_mode(
        &mut self,
        mode: PaletteMode,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.areas.palette.is_open() || self.areas.palette.mode == mode {
            return;
        }
        let palette = &mut self.areas.palette;
        palette.mode = mode;
        palette.cursor.reset();
        palette.messages.reset();
        palette.unavailable = None;
        palette
            .scroll
            .set_offset(gpui::point(px(spacing::S0), px(spacing::S0)));
        self.refresh_palette_words(window, cx);
        self.areas.palette.search.focus(window, cx);
        let query = match mode {
            PaletteMode::Commands => String::new(),
            PaletteMode::Messages => self.areas.palette.query.clone(),
        };
        self.message_search_query(&query, cx);
    }

    /// Refresh only the retained placeholder; language changes preserve input.
    pub(crate) fn refresh_palette_words(&self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let placeholder = match self.areas.palette.mode {
            PaletteMode::Commands => self.t.lang.palette().placeholder,
            PaletteMode::Messages => self.words.message_search().placeholder(),
        };
        self.areas
            .palette
            .search
            .set_placeholder(placeholder, window, cx);
    }

    /// Keyboard and pointer share the same action, including More.
    pub(super) fn message_palette_key(
        &mut self,
        key: CommandKey,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let rows = self.message_search_rows();
        match key {
            CommandKey::Up | CommandKey::Down => {
                let direction = match key {
                    CommandKey::Down => Direction::Next,
                    CommandKey::Up => Direction::Previous,
                    CommandKey::Enter | CommandKey::Escape => return,
                };
                if let Some(at) = self.areas.palette.messages.advance(&rows, direction) {
                    self.areas.palette.scroll.scroll_to_item(at);
                }
                cx.notify();
            }
            CommandKey::Enter => {
                if let Some(row) = self.areas.palette.messages.resolved(&rows).cloned() {
                    self.choose_message(&row, window, cx);
                }
            }
            CommandKey::Escape => self.close_palette(window, cx),
        }
    }

    fn choose_message(&mut self, row: &Row, window: &mut Window, cx: &mut Context<'_, Self>) {
        if !self.areas.palette.is_open() || self.areas.palette.mode != PaletteMode::Messages {
            return;
        }
        match row {
            Row::More => self.message_search_more(cx),
            Row::Hit(hit) => {
                let Some(destination) = self.message_search_destination(hit) else {
                    self.areas.palette.unavailable = Some(Unavailable);
                    cx.notify();
                    return;
                };
                self.close_palette(window, cx);
                self.open_search_hit(destination, window, cx);
            }
        }
    }
}

/// Existing segmented controls make the new search scope discoverable on touch.
#[cfg(test)]
pub(super) fn scope(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let compact = f32::from(window.viewport_size().width) < vessel_app_kit::picker::COMPACT;
    let words = shell.words.message_search();
    let modes = [
        (PaletteMode::Commands, words.commands()),
        (PaletteMode::Messages, words.messages()),
    ];
    let segments = modes.into_iter().map(|(mode, label)| {
        segment(
            SharedString::from(format!("palette-mode-{mode:?}")),
            SharedString::from(label),
            shell.areas.palette.mode == mode,
            true,
            SegmentSize::Small,
            tokens,
        )
        .min_h(px(if compact {
            vessel_app_kit::breakpoint::TOUCH_TARGET
        } else {
            SegmentSize::Small.inner()
        }))
        .debug_selector(move || format!("palette-mode-{mode:?}"))
        .on_click(
            cx.listener(move |shell, _event, window, cx| shell.palette_mode(mode, window, cx)),
        )
    });
    segmented(SegmentSize::Small)
        .pb(px(spacing::S1))
        .children(segments)
        .into_any_element()
}

/// One result owns a query snapshot so a queued click cannot activate old data.
fn result(
    shell: &Shell,
    row: &Row,
    active: Option<&Row>,
    query: std::sync::Arc<str>,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let compact = f32::from(window.viewport_size().width) < vessel_app_kit::picker::COMPACT;
    let selected = row.clone();
    let id = match row {
        Row::Hit(hit) => format!("message-{}-{}-{}", hit.machine, hit.session, hit.seq),
        Row::More => "message-more".to_owned(),
    };
    let drawn = div()
        .id(SharedString::from(id.clone()))
        .cursor_pointer()
        .on_click(cx.listener(move |shell, _event, window, cx| {
            // Render callbacks may outlive an edit; they must not open an old hit.
            if shell.areas.palette.query == query.as_ref()
                && shell.message_search_rows().contains(&selected)
            {
                shell.choose_message(&selected, window, cx);
            }
        }))
        .child(message_row(
            shell,
            row,
            active == Some(row),
            compact,
            tokens,
        ));
    if active == Some(row) {
        vessel_app_kit::popup::fill_in(
            SharedString::from(format!("{id}-arrive")),
            drawn,
            tokens.neutral.surface1,
        )
        .into_any_element()
    } else {
        drawn.into_any_element()
    }
}

/// Replies update identities without taking over a person's scroll position.
pub(super) fn render(
    shell: &mut Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let rows = shell.message_search_rows();
    let active = shell.areas.palette.messages.settle(&rows).cloned();
    let query = std::sync::Arc::<str>::from(shell.areas.palette.query.as_str());
    let children = rows.iter().map(|row| {
        result(
            shell,
            row,
            active.as_ref(),
            std::sync::Arc::clone(&query),
            tokens,
            window,
            cx,
        )
    });
    div()
        .id("palette-messages")
        .flex_1()
        .min_h(px(spacing::S0))
        .overflow_y_scroll()
        .track_scroll(&shell.areas.palette.scroll)
        .children(children)
        .children(query.trim().is_empty().then(|| {
            command_empty(
                SharedString::from(shell.words.message_search().hint()),
                tokens,
            )
        }))
        .children(message_search_note(shell, tokens))
        .children(shell.areas.palette.unavailable.as_ref().map(|_| {
            vessel_app_kit::alert::alert(
                vessel_app_kit::alert::Content {
                    title: None,
                    description: Some(
                        div()
                            .child(SharedString::from(
                                shell.words.message_search().unavailable(),
                            ))
                            .into_any_element(),
                    ),
                    action: None,
                },
                vessel_app_kit::alert::Variant::Warning,
                vessel_app_kit::alert::Size::Small,
                tokens,
            )
        }))
        .into_any_element()
}
