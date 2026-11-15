//! The parser's actions applied to the grid. Each handler says whether it
//! changed what is drawn, so the generation moves only then.

use super::Grid;
use super::modes::{
    AutoWrap, Bracketed, CursorKeys, Insert, LineFeed, Origin, Pending, Visibility,
};
use super::motion::Way;
use crate::terminal::parse::{Action, Control, Count, Csi, Esc, Mode, Osc, Switch, TabClear};

/// Whether an action changed what is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Effect {
    Drawn,
    Silent,
}

impl Grid {
    /// Apply one action.
    pub fn apply(&mut self, action: &Action) {
        let effect = match action {
            Action::Print(c) => {
                self.print(*c);
                Effect::Drawn
            }
            Action::Control(control) => self.control(*control),
            Action::Csi(csi) => self.csi(csi),
            Action::Esc(esc) => self.esc(*esc),
            Action::Osc(Osc::Title(title)) => {
                self.title = Some(title.clone());
                Effect::Silent
            }
            // Answers are the answered-query slice's; nothing here draws.
            Action::Osc(Osc::ColourQuery(_) | Osc::ClipboardRead | Osc::Other) | Action::Dcs(_) => {
                Effect::Silent
            }
        };
        if effect == Effect::Drawn {
            self.changed();
        }
    }

    fn control(&mut self, control: Control) -> Effect {
        match control {
            Control::Backspace => {
                self.at.col = self.at.col.saturating_sub(1);
                self.at.pending = Pending::Nothing;
            }
            Control::Tab => {
                self.at.col = self.tabs.next(self.at.col, self.size.last_col());
                self.at.pending = Pending::Nothing;
            }
            Control::LineFeed => self.line_feed(),
            Control::CarriageReturn => {
                self.at.col = 0;
                self.at.pending = Pending::Nothing;
            }
            Control::Bell | Control::Enquiry | Control::Other => return Effect::Silent,
        }
        Effect::Drawn
    }

    fn csi(&mut self, csi: &Csi) -> Effect {
        match csi {
            Csi::Up(n) => self.up(*n),
            Csi::Down(n) => self.down(*n),
            Csi::Forward(n) => self.sideways(*n, Way::Forward),
            Csi::Back(n) => self.sideways(*n, Way::Back),
            Csi::NextLine(n) => {
                self.down(*n);
                self.at.col = 0;
            }
            Csi::PreviousLine(n) => {
                self.up(*n);
                self.at.col = 0;
            }
            Csi::Column(col) => self.column(*col),
            Csi::Goto(row, col) => {
                self.row(*row);
                self.column(*col);
            }
            Csi::Row(row) => self.row(*row),
            Csi::TabForward(n) => self.tab_forward(*n),
            Csi::TabBack(n) => self.tab_back(*n),
            Csi::ScrollRegion(top, bottom) => self.set_region(*top, *bottom),
            Csi::SaveCursor => {
                self.save_cursor();
                return Effect::Silent;
            }
            Csi::RestoreCursor => self.restore_cursor(),
            Csi::Modes(switch, modes) => return self.modes(*switch, modes),
            Csi::ClearTabs(clear) => {
                match clear {
                    TabClear::Here => self.tabs.clear(self.at.col),
                    TabClear::All => self.tabs.clear_all(),
                }
                return Effect::Silent;
            }
            Csi::EraseDisplay(erase) => self.erase_display(*erase),
            Csi::EraseLine(erase) => self.erase_line(*erase),
            Csi::InsertLines(n) => self.insert_lines(*n),
            Csi::DeleteLines(n) => self.delete_lines(*n),
            Csi::InsertChars(n) => self.insert_chars(*n),
            Csi::DeleteChars(n) => self.delete_chars(*n),
            Csi::EraseChars(n) => self.erase_chars(*n),
            Csi::ScrollUp(n) => self.scroll_up(*n),
            Csi::ScrollDown(n) => self.scroll_down(*n),
            Csi::Graphic(groups) => {
                super::sgr::apply(&mut self.pen, groups);
                return Effect::Silent;
            }
            // Queries are the answered-query slice's; the cursor's shape
            // is the app's bar.
            Csi::Status(_)
            | Csi::Attributes(_)
            | Csi::CursorStyle
            | Csi::RefusedQuery
            | Csi::Unsupported => return Effect::Silent,
        }
        Effect::Drawn
    }

    fn tab_forward(&mut self, n: Count) {
        for _ in 0..n.get().min(self.size.cols()) {
            self.at.col = self.tabs.next(self.at.col, self.size.last_col());
        }
        self.at.pending = Pending::Nothing;
    }

    fn tab_back(&mut self, n: Count) {
        for _ in 0..n.get().min(self.size.cols()) {
            self.at.col = self.tabs.previous(self.at.col);
        }
        self.at.pending = Pending::Nothing;
    }

    fn modes(&mut self, switch: Switch, modes: &[Mode]) -> Effect {
        let mut effect = Effect::Silent;
        for mode in modes {
            if self.mode(switch, *mode) == Effect::Drawn {
                effect = Effect::Drawn;
            }
        }
        effect
    }

    fn mode(&mut self, switch: Switch, mode: Mode) -> Effect {
        let set = switch == Switch::Set;
        match mode {
            Mode::Insert => self.modes.insert = if set { Insert::Insert } else { Insert::Replace },
            Mode::NewLine => {
                self.modes.line_feed = if set {
                    LineFeed::DownAndReturn
                } else {
                    LineFeed::Down
                };
            }
            Mode::CursorKeys => {
                self.modes.cursor_keys = if set {
                    CursorKeys::Application
                } else {
                    CursorKeys::Normal
                };
            }
            Mode::BracketedPaste => {
                self.modes.bracketed = if set { Bracketed::On } else { Bracketed::Off };
            }
            Mode::Origin => {
                self.modes.origin = if set { Origin::Region } else { Origin::Screen };
                self.home();
                return Effect::Drawn;
            }
            Mode::AutoWrap => {
                self.modes.auto_wrap = if set { AutoWrap::Wrap } else { AutoWrap::Stay };
                self.at.pending = Pending::Nothing;
            }
            Mode::ShowCursor => {
                self.modes.visibility = if set {
                    Visibility::Shown
                } else {
                    Visibility::Hidden
                };
                return Effect::Drawn;
            }
            Mode::SaveCursor => match switch {
                Switch::Set => self.save_cursor(),
                Switch::Reset => {
                    self.restore_cursor();
                    return Effect::Drawn;
                }
            },
            Mode::AltScreen | Mode::AltScreenSaveCursor => {
                self.screen_switch(switch, mode);
                return Effect::Drawn;
            }
            Mode::Other => {}
        }
        Effect::Silent
    }

    /// `?47`, `?1047` and `?1049`: `?1049` also saves the cursor before
    /// entering and restores it after leaving.
    fn screen_switch(&mut self, switch: Switch, mode: Mode) {
        let saves = mode == Mode::AltScreenSaveCursor;
        match switch {
            Switch::Set => {
                if saves {
                    self.save_cursor();
                }
                self.enter_alternate();
            }
            Switch::Reset => {
                self.leave_alternate();
                if saves {
                    self.restore_cursor();
                }
            }
        }
    }

    fn esc(&mut self, esc: Esc) -> Effect {
        match esc {
            Esc::Index => self.index(),
            Esc::NextLine => {
                self.index();
                self.at.col = 0;
            }
            Esc::ReverseIndex => self.reverse_index(),
            Esc::RestoreCursor => self.restore_cursor(),
            Esc::Reset => self.reset(),
            Esc::SaveCursor => {
                self.save_cursor();
                return Effect::Silent;
            }
            Esc::SetTab => {
                self.tabs.set(self.at.col);
                return Effect::Silent;
            }
            Esc::Other => return Effect::Silent,
        }
        Effect::Drawn
    }

    /// RIS: everything as new, history too (xterm.js recreates its
    /// buffers); the generation keeps counting.
    fn reset(&mut self) {
        let generation = self.generation;
        let scrollback = self.history.limit();
        let history_first = self.history.first();
        *self = Self::new(self.size, scrollback);
        self.history.reset_after(history_first);
        self.generation = generation;
    }
}
