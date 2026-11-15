//! Compact details reuse the kit dialog's bottom-sheet path and the same tool
//! bodies. Local generations fence queued dismissals; no node action lives here.

use super::entries::{Paint, id};
use super::{ConversationArea, Seat};
use crate::shell::Shell;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, ElementId, IntoElement as _, ParentElement as _, Styled as _, Window, div,
    px,
};
use vessel_app_kit::{
    DialogContent, DialogSize, Start, Tokens, Type, TypeExt as _, breakpoint, dialog, output_box,
};
use vessel_app_view::prefs::ReasoningExpansion;
use vessel_app_view::timeline::{EntryKey, EntryKind, Folds, Timeline, plan_text};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Generation(u64);

#[derive(Debug, Clone)]
enum Selection {
    /// Its former identity vanished; the existing overlay dismisses normally.
    Unavailable,
    Entry {
        index: usize,
        key: EntryKey,
    },
    Run {
        lead: usize,
        key: EntryKey,
    },
}

#[derive(Debug)]
struct Shown {
    generation: Generation,
    selection: Selection,
}

/// Preserve the counter on clear/dehydration, just like other async UI identities.
#[derive(Debug, Default)]
pub(super) struct SheetState {
    next: u64,
    shown: Option<Shown>,
}

#[cfg(test)]
impl SheetState {
    /// The resident snapshot cannot discard this owner's live work/UI state.
    pub(super) fn snapshot_busy(&self) -> bool {
        self.shown.is_some()
    }
}

impl SheetState {
    pub(super) fn clear(&mut self) {
        self.shown = None;
    }

    pub(super) fn group_open(&self) -> bool {
        self.shown
            .as_ref()
            .is_some_and(|shown| matches!(shown.selection, Selection::Run { .. }))
    }

    /// Preserve an open detail by key across a rebuilt prefix. A vanished target
    /// remains a pending dismissal so the shared dialog restores focus normally.
    pub(super) fn rebase(
        &mut self,
        timeline: &Timeline,
        folds: Option<&Folds>,
        reasoning: ReasoningExpansion,
    ) {
        let next = self
            .shown
            .as_ref()
            .and_then(|shown| match &shown.selection {
                Selection::Entry { key, .. } => {
                    entry_selection(timeline, reasoning, timeline.index_of(key)?)
                }
                Selection::Run { key, .. } => {
                    let index = timeline.index_of(key)?;
                    if timeline.rewound(index) {
                        return None;
                    }
                    let run = folds?.run_of(index)?;
                    Some(Selection::Run {
                        lead: run.lead,
                        key: timeline.entries().get(run.lead)?.key.clone(),
                    })
                }
                Selection::Unavailable => None,
            });
        if let Some(shown) = &mut self.shown {
            shown.selection = next.unwrap_or(Selection::Unavailable);
        }
    }

    fn take(&mut self, generation: Generation) -> Option<Shown> {
        if self.shown.as_ref()?.generation != generation {
            return None;
        }
        self.shown.take()
    }

    fn show(&mut self, selection: Selection) -> bool {
        let Some(next) = self.next.checked_add(1) else {
            return false;
        };
        self.next = next;
        self.shown = Some(Shown {
            generation: Generation(next),
            selection,
        });
        true
    }
}

fn entry_selection(
    timeline: &Timeline,
    reasoning: ReasoningExpansion,
    index: usize,
) -> Option<Selection> {
    if timeline.rewound(index) {
        return None;
    }
    let entry = timeline.entries().get(index)?;
    let eligible = match &entry.kind {
        EntryKind::Tool(tool) => plan_text(tool).is_none(),
        EntryKind::Thought { .. } => !reasoning.expanded(),
        EntryKind::Ready { .. }
        | EntryKind::Prompt(_)
        | EntryKind::Steer { .. }
        | EntryKind::Question { .. }
        | EntryKind::Message { .. }
        | EntryKind::Plan { .. }
        | EntryKind::Permission(_)
        | EntryKind::TurnEnded { .. }
        | EntryKind::TurnFailed { .. }
        | EntryKind::Log { .. }
        | EntryKind::Status { .. }
        | EntryKind::Parking(_)
        | EntryKind::Forked { .. }
        | EntryKind::Profile { .. }
        | EntryKind::Notification(_)
        | EntryKind::Compaction(_)
        | EntryKind::Plugin(_)
        | EntryKind::Model(_) => false,
    };
    if !eligible {
        return None;
    }
    Some(Selection::Entry {
        index,
        key: entry.key.clone(),
    })
}

impl Shell {
    /// Only tools and non-forced reasoning use compact details; plans stay inline.
    pub(crate) fn entry_sheet(
        &mut self,
        seat: Seat,
        index: usize,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        let Some(area) = seat.get_mut(self) else {
            return false;
        };
        let Some(selection) = entry_selection(&area.timeline, area.reasoning, index) else {
            return false;
        };
        if !area.tool_sheet.show(selection) {
            return false;
        }
        area.dialog.focus(window, cx);
        cx.notify();
        true
    }

    pub(crate) fn run_sheet(
        &mut self,
        seat: Seat,
        lead: usize,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        let Some(area) = seat.get_mut(self) else {
            return false;
        };
        if area
            .folds
            .as_ref()
            .and_then(|folds| folds.run_of(lead))
            .is_none()
        {
            return false;
        }
        let Some(entry) = area.timeline.entries().get(lead) else {
            return false;
        };
        if !area.tool_sheet.show(Selection::Run {
            lead,
            key: entry.key.clone(),
        }) {
            return false;
        }
        area.dialog.focus(window, cx);
        cx.notify();
        true
    }

    fn dismiss_tool_sheet(
        &mut self,
        seat: Seat,
        generation: Generation,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(area) = seat.get_mut(self) else {
            return;
        };
        let Some(_shown) = area.tool_sheet.take(generation) else {
            return;
        };
        area.dialog.dismissed(window, cx);
        cx.notify();
    }
}

/// Render outside the virtualized rows so new output cannot unmount an open sheet.
pub(super) fn overlay(
    shell: &Shell,
    seat: Seat,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let area = seat.get(shell)?;
    let shown = area.tool_sheet.shown.as_ref()?;
    let generation = shown.generation;
    let paint = super::timeline::paint(shell, area, seat, tokens, window);
    let Some((title, body)) = content(&paint, area, &shown.selection, window, cx) else {
        cx.defer_in(window, move |shell, window, cx| {
            shell.dismiss_tool_sheet(seat, generation, window, cx);
        });
        return None;
    };
    Some(dialog(
        &area.dialog,
        DialogContent {
            title: title.into(),
            body,
            footer: Vec::new(),
            size: DialogSize::Wide,
        },
        tokens,
        window,
        breakpoint::phone(window),
        move |shell: &mut Shell, window, cx| shell.dismiss_tool_sheet(seat, generation, window, cx),
        cx,
    ))
}

fn content(
    paint: &Paint<'_>,
    area: &ConversationArea,
    selection: &Selection,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<(String, AnyElement)> {
    match selection {
        Selection::Unavailable => None,
        Selection::Entry { index, key } => entry_content(paint, area, *index, key, window, cx),
        Selection::Run { lead, key } => run_content(paint, area, *lead, key, window, cx),
    }
}

fn entry_content(
    paint: &Paint<'_>,
    area: &ConversationArea,
    index: usize,
    key: &EntryKey,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<(String, AnyElement)> {
    let entry = area
        .timeline
        .entries()
        .get(index)
        .filter(|entry| &entry.key == key)?;
    match &entry.kind {
        EntryKind::Tool(tool) => Some((
            if tool.title.is_empty() {
                paint.words.tool_call().to_owned()
            } else {
                tool.title.clone()
            },
            super::tool::body(paint, index, tool, window, cx).into_any_element(),
        )),
        EntryKind::Thought { text } => Some((
            paint.words.thought().to_owned(),
            output_box(
                ElementId::NamedInteger("thought-sheet".into(), id(index)),
                text,
                Start::Top,
                paint.tokens,
                window,
                cx,
            ),
        )),
        EntryKind::Ready { .. }
        | EntryKind::Prompt(_)
        | EntryKind::Steer { .. }
        | EntryKind::Question { .. }
        | EntryKind::Message { .. }
        | EntryKind::Plan { .. }
        | EntryKind::Permission(_)
        | EntryKind::TurnEnded { .. }
        | EntryKind::TurnFailed { .. }
        | EntryKind::Log { .. }
        | EntryKind::Status { .. }
        | EntryKind::Parking(_)
        | EntryKind::Forked { .. }
        | EntryKind::Profile { .. }
        | EntryKind::Notification(_)
        | EntryKind::Compaction(_)
        | EntryKind::Plugin(_)
        | EntryKind::Model(_) => None,
    }
}

fn run_content(
    paint: &Paint<'_>,
    area: &ConversationArea,
    lead: usize,
    key: &EntryKey,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<(String, AnyElement)> {
    let _host = area
        .timeline
        .entries()
        .get(lead)
        .filter(|entry| &entry.key == key)?;
    let run = area.folds.as_ref()?.run_of(lead)?;
    let mut rows = div().flex().flex_col().gap(px(spacing::S1));
    for index in run.lead..run.end {
        let entry = area.timeline.entries().get(index)?;
        rows = rows.child(super::entries::entry(
            paint,
            index,
            entry,
            area.entry_expanded(index),
            super::pacing::phase(area, entry),
            window,
            cx,
        ));
    }
    Some((
        paint.words.run_summary(&run.summary),
        rows.into_any_element(),
    ))
}

/// Empty legacy details follow the same loading/empty contract as canonical bodies.
pub(super) fn empty(paint: &Paint<'_>, index: usize, working: bool) -> AnyElement {
    super::tool_body::empty(
        &super::tool_body::BodyPaint::from(paint),
        ElementId::NamedInteger("tool-details-loading".into(), id(index)),
        working,
    )
}

#[cfg(test)]
mod generation_tests {
    use super::{Selection, SheetState};
    use vessel_app_view::timeline::EntryKey;
    use vessel_proto::EventSeq;

    fn entry() -> Selection {
        Selection::Entry {
            index: 0,
            key: EntryKey::Record(EventSeq::new(0)),
        }
    }

    #[test]
    fn an_old_dismissal_cannot_close_a_reopened_sheet_even_after_clear() {
        let mut state = SheetState::default();
        assert!(state.show(entry()));
        let old = state.shown.as_ref().unwrap().generation;
        state.clear();
        assert!(state.show(entry()));
        let current = state.shown.as_ref().unwrap().generation;
        assert!(state.take(old).is_none());
        assert_eq!(state.shown.as_ref().unwrap().generation, current);
        assert!(state.take(current).is_some());
        assert!(state.take(current).is_none());
    }
    #[test]
    fn a_sheet_rebases_its_key_and_keeps_normal_dismissal_when_that_key_disappears() {
        use super::ReasoningExpansion;
        use chrono::DateTime;
        use vessel_app_view::timeline::Timeline;
        use vessel_proto::{AgentEvent, ContentBlock, EventRecord, SessionEvent};
        let mut timeline = Timeline::default();
        let records = [2, 10].map(|seq| EventRecord {
            seq: EventSeq::new(seq),
            at: DateTime::UNIX_EPOCH,
            event: SessionEvent::Agent {
                event: AgentEvent::ThoughtChunk {
                    content: ContentBlock::Text {
                        text: "Thinking".to_owned(),
                    },
                },
            },
        });
        // Different logical reasoning rows need a real content boundary.
        let mut records = records.to_vec();
        records.insert(
            1,
            EventRecord {
                seq: EventSeq::new(3),
                at: DateTime::UNIX_EPOCH,
                event: SessionEvent::Agent {
                    event: AgentEvent::MessageChunk {
                        content: ContentBlock::Text {
                            text: "Boundary".to_owned(),
                        },
                    },
                },
            },
        );
        let _change = timeline.apply(&records);
        let mut state = SheetState::default();
        assert!(state.show(Selection::Entry {
            index: 0,
            key: EntryKey::Record(EventSeq::new(10))
        }));
        let generation = state.shown.as_ref().unwrap().generation;
        state.rebase(&timeline, None, ReasoningExpansion::Collapsed);
        assert!(matches!(
            &state.shown.as_ref().unwrap().selection,
            Selection::Entry { index: 2, .. }
        ));
        assert_eq!(state.shown.as_ref().unwrap().generation, generation);
        state.rebase(&Timeline::default(), None, ReasoningExpansion::Collapsed);
        assert!(matches!(
            state.shown.as_ref().unwrap().selection,
            Selection::Unavailable
        ));
        assert!(
            state.take(generation).is_some(),
            "normal actual-close path still owns focus restoration"
        );
    }
}
