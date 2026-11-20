//! Literal find in the grid. Case is folded per scalar. A continuation
//! cell is not a character, so a wide scalar is one step of the match.
//! Next and previous stop at the ends: whether Paseo's search wraps was
//! not read.

use super::super::types::{Cell, Glyph, Place};
use super::{Line, Shown, TerminalModel};

/// Paseo's highlight cap (`FIND_HIGHLIGHT_LIMIT`). At this count the
/// result is [`FindBound::Limited`], matching `resultCount >=` that cap.
const LIMIT: u32 = 20_000;

/// Whether every match was kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindBound {
    /// The count is every match.
    All,
    /// The count stopped at [`LIMIT`]. There may be more.
    Limited,
}

/// What a find is showing. `current` is absent when nothing matched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FindView {
    count: u32,
    current: Option<Place>,
    bound: FindBound,
}

impl FindView {
    /// How many highlights were kept, at most 20,000.
    #[must_use]
    pub const fn count(self) -> u32 {
        self.count
    }

    /// The highlight next and previous move. Absent when nothing matched.
    #[must_use]
    pub const fn current(self) -> Option<Place> {
        self.current
    }

    /// Whether the cap was reached.
    #[must_use]
    pub const fn bound(self) -> FindBound {
        self.bound
    }
}

#[derive(Debug, Default)]
pub(super) struct FindState {
    places: Vec<Place>,
    at: Option<u32>,
    bound: FindBound,
}

impl Default for FindBound {
    fn default() -> Self {
        Self::All
    }
}

struct Unit {
    ch: char,
    place: Place,
    width: u16,
    /// The line this scalar sits on wrapped, so the next line continues it.
    wrapped_after: bool,
}

impl TerminalModel {
    /// Find `query` from the start of the buffer. An empty query matches
    /// nothing. The match is literal and case-insensitive, and it does not
    /// cross a gap of empty cells.
    #[must_use]
    pub fn find(&mut self, query: &str) -> FindView {
        let (places, bound) = self.matches(query);
        let at = if places.is_empty() { None } else { Some(0) };
        self.find = FindState { places, at, bound };
        self.find_view()
    }

    /// The next highlight, or the current one when this is the last.
    #[must_use]
    pub fn find_next(&mut self) -> FindView {
        if let Some(at) = self.find.at {
            let last = u32::try_from(self.find.places.len().saturating_sub(1)).unwrap_or(0);
            if at < last {
                self.find.at = Some(at.saturating_add(1));
            }
        }
        self.find_view()
    }

    /// The previous highlight, or the current one when this is the first.
    #[must_use]
    pub fn find_previous(&mut self) -> FindView {
        if let Some(at) = self.find.at {
            if at > 0 {
                self.find.at = Some(at.saturating_sub(1));
            }
        }
        self.find_view()
    }

    /// Forget the find.
    pub fn clear_find(&mut self) {
        self.find = FindState::default();
    }

    fn find_view(&self) -> FindView {
        let current = self.find.at.and_then(|at| {
            let index = usize::try_from(at).ok()?;
            self.find.places.get(index).copied()
        });
        let count = u32::try_from(self.find.places.len()).unwrap_or(u32::MAX);
        FindView {
            count,
            current,
            bound: self.find.bound,
        }
    }

    fn matches(&self, query: &str) -> (Vec<Place>, FindBound) {
        let needle: Vec<char> = query.chars().collect();
        if needle.is_empty() {
            return (Vec::new(), FindBound::All);
        }
        let units = self.units();
        let mut places = Vec::new();
        let mut bound = FindBound::All;
        let mut start = 0usize;
        while start < units.len() {
            if hit(&units, start, &needle, self.cols()) {
                if u32::try_from(places.len()).unwrap_or(u32::MAX) >= LIMIT {
                    bound = FindBound::Limited;
                    break;
                }
                if let Some(unit) = units.get(start) {
                    places.push(unit.place);
                }
            }
            start = start.saturating_add(1);
        }
        if u32::try_from(places.len()).unwrap_or(u32::MAX) >= LIMIT {
            bound = FindBound::Limited;
        }
        (places, bound)
    }

    fn units(&self) -> Vec<Unit> {
        let mut lines = Vec::new();
        if self.shown == Shown::Primary {
            for line in &self.history {
                lines.push(line);
            }
        }
        for line in self.screen() {
            lines.push(line);
        }
        let mut units = Vec::new();
        let mut line_no = 0u32;
        for line in lines {
            push_line(&mut units, line, line_no);
            line_no = line_no.saturating_add(1);
        }
        units
    }
}

fn push_line(units: &mut Vec<Unit>, line: &Line, line_no: u32) {
    let mut index = 0usize;
    while index < line.cells.len() {
        let Some(cell) = line.cells.get(index) else {
            break;
        };
        match cell.glyph() {
            Glyph::Scalar(ch) => {
                let width = width_of(&line.cells, index);
                let col = u16::try_from(index).unwrap_or(u16::MAX);
                units.push(Unit {
                    ch,
                    place: Place { line: line_no, col },
                    width,
                    wrapped_after: line.wrapped,
                });
                index = index.saturating_add(usize::from(width));
            }
            Glyph::Continuation | Glyph::Empty => index = index.saturating_add(1),
        }
    }
}

fn width_of(cells: &[Cell], index: usize) -> u16 {
    if matches!(
        cells.get(index.saturating_add(1)).copied().map(Cell::glyph),
        Some(Glyph::Continuation)
    ) {
        2
    } else {
        1
    }
}

fn hit(units: &[Unit], start: usize, needle: &[char], cols: u16) -> bool {
    let mut previous: Option<&Unit> = None;
    for (offset, expected) in needle.iter().enumerate() {
        let Some(unit) = units.get(start.saturating_add(offset)) else {
            return false;
        };
        if let Some(previous) = previous {
            if !follows(previous, unit, cols) {
                return false;
            }
        }
        if !same_char(unit.ch, *expected) {
            return false;
        }
        previous = Some(unit);
    }
    true
}

fn follows(previous: &Unit, next: &Unit, cols: u16) -> bool {
    let next_col = previous.place.col.saturating_add(previous.width);
    if next.place.line == previous.place.line {
        return next.place.col == next_col;
    }
    previous.wrapped_after
        && next.place.line == previous.place.line.saturating_add(1)
        && next.place.col == 0
        && next_col >= cols
}

fn same_char(left: char, right: char) -> bool {
    let mut left = left.to_lowercase();
    let mut right = right.to_lowercase();
    loop {
        match (left.next(), right.next()) {
            (None, None) => return true,
            (Some(a), Some(b)) if a == b => {}
            (None, Some(_)) | (Some(_), None) | (Some(_), Some(_)) => return false,
        }
    }
}
