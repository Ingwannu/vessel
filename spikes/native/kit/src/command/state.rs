//! Frame lifetime is separate from the owner's open flag: an exit keeps
//! painting while the owner can restore focus once, before running an action.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use gpui::Pixels;

use crate::sheet::snap::motion::Fade;
use crate::sheet::snap::{self, Movement};

#[derive(Debug, Clone, Copy)]
enum Phase {
    Hidden,
    Enter(Instant),
    Shown,
    Exit { started: Instant },
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
        if matches!(self.phase, Phase::Hidden | Phase::Exit { .. }) {
            return;
        }
        let now = Instant::now();
        self.phase = Phase::Exit { started: now };
        self.sheet.borrow_mut().close(now);
    }

    /// Whether an open command should reset a palette that is exiting.
    #[must_use]
    pub const fn closing(&self) -> bool {
        matches!(self.phase, Phase::Exit { .. })
    }

    /// Sample before drawing. The existing App reduced-motion flag is
    /// supplied by the renderer; this control does not read OS settings.
    #[must_use]
    pub fn sample(&mut self, compact: bool, height: Pixels, reduced: bool) -> CommandMotion {
        let now = Instant::now();
        let opacity = self.opacity(now, reduced);
        let closing = self.closing();
        let mut hidden = matches!(self.phase, Phase::Hidden) || (closing && opacity <= 0.0);
        let mut animating = matches!(self.phase, Phase::Enter(_) | Phase::Exit { .. });
        let sheet = if compact {
            match self.sheet.borrow_mut().frame(height, now, reduced) {
                snap::Frame::PendingLayout => None,
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
            Phase::Hidden => 0.0,
            Phase::Shown => 1.0,
            Phase::Enter(started) => {
                let progress = Fade::In.progress(now.saturating_duration_since(started), reduced);
                if progress >= 1.0 {
                    self.phase = Phase::Shown;
                }
                progress
            }
            Phase::Exit { started } => {
                let progress = Fade::Out.progress(now.saturating_duration_since(started), reduced);
                progress.mul_add(-1.0, 1.0)
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
    fn desktop_fade_finishes_and_reopening_resets_the_compact_stop() {
        let mut state = CommandState::default();
        state.present();
        state.phase = Phase::Enter(
            Instant::now()
                .checked_sub(Duration::from_millis(125))
                .unwrap(),
        );
        let entering = state.sample(false, px(1000.0), false);
        assert!(entering.opacity > 0.5 && entering.opacity < 1.0);
        assert!(entering.animating && !entering.hidden);
        state.close();
        assert!(state.closing());
        state.phase = Phase::Exit {
            started: Instant::now()
                .checked_sub(Duration::from_millis(251))
                .unwrap(),
        };
        let ended = state.sample(false, px(1000.0), false);
        assert!(ended.hidden && !ended.animating);
        state.present();
        let reopened = state.sample(true, px(1000.0), true);
        assert!(!reopened.hidden && !reopened.closing && !reopened.animating);
        assert_eq!(reopened.sheet.unwrap().height, px(600.0));
        state.close();
        assert!(state.sample(true, px(1000.0), true).hidden);
    }
}
