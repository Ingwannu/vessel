//! Logical visibility and the last painted compact frame have different
//! lifetimes. Closing stops input ownership immediately; the sheet can keep
//! painting while its owner restores focus once before running an action.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use gpui::Pixels;

use crate::sheet::snap::motion::fade_in;
use crate::sheet::snap::{self, Movement};

#[derive(Debug, Clone, Copy)]
enum Phase {
    Hidden,
    Enter(Instant),
    Shown,
    Exit,
}

/// Persistent motion for one command palette. Its gesture handles share sheet state.
#[derive(Debug)]
pub struct CommandState {
    pub(super) sheet: Rc<RefCell<snap::State>>,
    phase: Phase,
}

impl Default for CommandState {
    fn default() -> Self {
        Self {
            sheet: Rc::new(RefCell::new(snap::State::default())),
            phase: Phase::Hidden,
        }
    }
}

/// A motion sample, before constructing the frame's elements.
#[derive(Debug, Clone, Copy)]
pub struct CommandMotion {
    /// Desktop panel and backdrop opacity (compact uses the sheet backdrop).
    pub opacity: f32,
    /// Compact geometry; None means desktop or dismissal completed.
    pub sheet: Option<snap::Geometry>,
    /// Keep requesting frames while true.
    pub animating: bool,
    /// Exit has finished; the host may drop its open flag.
    pub hidden: bool,
    /// During exit, painted rows must not run another action.
    pub closing: bool,
}

impl CommandState {
    /// Open or reopen, resetting the compact stop to 60%.
    pub fn present(&mut self) {
        let now = Instant::now();
        self.phase = Phase::Enter(now);
        self.sheet.borrow_mut().present(now);
    }

    /// Start exit once. The owner restores its previous focus now, not
    /// a second time when the final animation frame disappears.
    pub fn close(&mut self) {
        if matches!(self.phase, Phase::Hidden | Phase::Exit) {
            return;
        }
        let now = Instant::now();
        self.phase = Phase::Exit;
        self.sheet.borrow_mut().close(now);
    }

    /// Logical visibility for input and focus ownership. A closing compact
    /// sheet may still paint while this is false.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        matches!(self.phase, Phase::Enter(_) | Phase::Shown)
    }

    /// Whether an open command should reset a palette that is exiting.
    #[must_use]
    pub const fn closing(&self) -> bool {
        matches!(self.phase, Phase::Exit)
    }

    /// Sample before drawing. The existing App reduced-motion flag is
    /// supplied by the renderer; this control does not read OS settings.
    #[must_use]
    pub fn sample(&mut self, compact: bool, height: Pixels, reduced: bool) -> CommandMotion {
        self.sample_at(compact, height, reduced, Instant::now())
    }

    fn sample_at(
        &mut self,
        compact: bool,
        height: Pixels,
        reduced: bool,
        now: Instant,
    ) -> CommandMotion {
        let opacity = self.opacity(now, reduced);
        let closing = self.closing();
        let mut hidden = matches!(self.phase, Phase::Hidden) || (closing && !compact);
        let mut animating = matches!(self.phase, Phase::Enter(_) | Phase::Exit);
        let sheet = if compact {
            match self.sheet.borrow_mut().frame(height, now, reduced) {
                snap::Frame::PendingLayout => {
                    // A zero-size window needs a resize, not an animation
                    // loop that keeps requesting frames without geometry.
                    animating = false;
                    None
                }
                snap::Frame::Closed => {
                    hidden = true;
                    animating = false;
                    None
                }
                snap::Frame::Visible(geometry) => {
                    hidden = false;
                    animating = geometry.movement == Movement::Animating;
                    Some(geometry)
                }
            }
        } else {
            None
        };
        if hidden {
            self.phase = Phase::Hidden;
            animating = false;
        }
        CommandMotion {
            opacity,
            sheet,
            animating,
            hidden,
            closing,
        }
    }

    fn opacity(&mut self, now: Instant, reduced: bool) -> f32 {
        match self.phase {
            // Hidden, or closing: the desktop Command Center unmounts
            // before RNW can run its exit curve; compact dismissal is the
            // sheet's spring, not this fade.
            Phase::Hidden | Phase::Exit => 0.0,
            Phase::Shown => 1.0,
            Phase::Enter(started) => {
                let progress = fade_in(now.saturating_duration_since(started), reduced);
                if progress >= 1.0 {
                    self.phase = Phase::Shown;
                }
                progress
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::px;
    use std::time::Duration;

    #[test]
    fn desktop_fades_in_and_closes_immediately_then_reopens_compact() {
        let mut state = CommandState::default();
        assert!(!state.is_open());
        state.present();
        assert!(state.is_open());
        let now = Instant::now();
        state.phase = Phase::Enter(now.checked_sub(Duration::from_millis(125)).unwrap());
        let entering = state.sample_at(false, px(1000.0), false, now);
        assert!(entering.opacity > 0.5 && entering.opacity < 1.0);
        assert!(entering.animating && !entering.hidden);
        state.close();
        assert!(state.closing());
        assert!(!state.is_open());
        let ended = state.sample_at(false, px(1000.0), false, now);
        assert!(ended.hidden && !ended.animating);
        state.present();
        let reopened = state.sample(true, px(1000.0), true);
        assert!(!reopened.hidden && !reopened.closing && !reopened.animating);
        assert_eq!(reopened.sheet.unwrap().height, px(600.0));
        state.close();
        assert!(state.sample(true, px(1000.0), true).hidden);
    }

    #[test]
    fn zero_viewport_waits_for_layout_without_requesting_frames() {
        let mut state = CommandState::default();
        state.present();
        let now = Instant::now();
        let pending = state.sample_at(true, px(0.0), false, now);
        assert!(!pending.hidden && !pending.animating && pending.sheet.is_none());
        let resized = state.sample_at(true, px(1000.0), true, now);
        assert!(!resized.hidden && !resized.animating);
        assert_eq!(resized.sheet.unwrap().height, px(600.0));
    }
}
