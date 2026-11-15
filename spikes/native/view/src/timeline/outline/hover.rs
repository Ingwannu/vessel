//! Hover intent owns neither timers nor drawing. Stable slots prevent magnification
//! from moving the hit target; tickets make cancelled timers harmless.

use std::time::Duration;

/// Paseo's first activation delay; moving between slots keeps this deadline.
pub const OUTLINE_INTENT: Duration = Duration::from_millis(150);
const HORIZONTAL_TRANSIT: f32 = 4.0;

/// A position in logical screen pixels, admitted only when finite.
#[derive(Debug, Clone, Copy)]
pub struct OutlinePoint {
    x: f32,
    y: f32,
}

impl OutlinePoint {
    /// Reject non-finite coordinates at the UI boundary.
    #[must_use]
    pub fn new(x: f32, y: f32) -> Option<Self> {
        (x.is_finite() && y.is_finite()).then_some(Self { x, y })
    }
}

/// A slot belonging to the current outline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutlineSlot(usize);

impl OutlineSlot {
    /// Only an existing prompt can become a candidate.
    #[must_use]
    pub fn new(index: usize, count: usize) -> Option<Self> {
        (index < count).then_some(Self(index))
    }

    /// Its position in the outline.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }
}

/// Identity of one delayed activation, invalidated by transit or leave.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HoverTicket(u64);

/// The only effects the screen must perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoverChange {
    /// No redraw or new timer is needed.
    None,
    /// Start one delay and later present this ticket to `activate`.
    Schedule(HoverTicket),
    /// The attention changed; redraw the rail.
    Changed,
}

#[derive(Debug, Default)]
enum State {
    #[default]
    Idle,
    Waiting(OutlineSlot),
    Active(OutlineSlot),
}

/// Paseo's hover-intent state machine, independent of the UI clock.
#[derive(Debug, Default)]
pub struct OutlineHover {
    state: State,
    anchor: Option<OutlinePoint>,
    ticket: u64,
}

impl OutlineHover {
    /// Remember where the pointer entered the rail.
    pub const fn enter(&mut self, point: OutlinePoint) {
        self.anchor = Some(point);
    }

    /// Vertical movement selects a new candidate without restarting the delay.
    #[must_use]
    pub fn point_at(&mut self, slot: OutlineSlot) -> HoverChange {
        match self.state {
            State::Idle => {
                self.state = State::Waiting(slot);
                self.schedule()
            }
            State::Waiting(_) => {
                self.state = State::Waiting(slot);
                HoverChange::None
            }
            State::Active(current) => {
                self.state = State::Active(slot);
                if slot == current {
                    HoverChange::None
                } else {
                    HoverChange::Changed
                }
            }
        }
    }

    /// A sideways sweep postpones activation; vertical motion updates the anchor.
    #[must_use]
    #[expect(
        clippy::float_arithmetic,
        reason = "finite logical pointer coordinates; differences are used only for direction and a distance threshold"
    )]
    pub fn moved(&mut self, point: OutlinePoint) -> HoverChange {
        if !matches!(self.state, State::Waiting(_)) {
            return HoverChange::None;
        }
        let Some(anchor) = self.anchor else {
            return HoverChange::None;
        };
        let dx = (point.x - anchor.x).abs();
        let dy = (point.y - anchor.y).abs();
        if dy >= dx {
            self.anchor = Some(point);
            return HoverChange::None;
        }
        if dx < HORIZONTAL_TRANSIT {
            return HoverChange::None;
        }
        self.anchor = Some(point);
        self.schedule()
    }

    /// An expired timer can activate only its own still-waiting candidate.
    #[must_use]
    pub fn activate(&mut self, ticket: HoverTicket) -> HoverChange {
        if ticket.0 != self.ticket {
            return HoverChange::None;
        }
        match self.state {
            State::Waiting(slot) => {
                self.state = State::Active(slot);
                HoverChange::Changed
            }
            State::Idle | State::Active(_) => HoverChange::None,
        }
    }

    /// Leaving or hiding the rail cancels activation and clears attention.
    #[must_use]
    pub fn leave(&mut self) -> HoverChange {
        let changed = matches!(self.state, State::Active(_));
        self.state = State::Idle;
        self.anchor = None;
        let _cancelled = self.schedule();
        if changed {
            HoverChange::Changed
        } else {
            HoverChange::None
        }
    }

    /// The pill that currently owns the magnified band and preview.
    #[must_use]
    pub const fn attention(&self) -> Option<OutlineSlot> {
        match self.state {
            State::Active(slot) => Some(slot),
            State::Idle | State::Waiting(_) => None,
        }
    }

    /// Whether a delayed activation still references this controller.
    #[must_use]
    pub const fn waiting(&self) -> bool {
        matches!(self.state, State::Waiting(_))
    }

    fn schedule(&mut self) -> HoverChange {
        // A UI lifetime cannot exhaust the counter; wrapping also avoids a panic.
        self.ticket = self.ticket.wrapping_add(1);
        HoverChange::Schedule(HoverTicket(self.ticket))
    }
}

#[cfg(test)]
mod tests;
