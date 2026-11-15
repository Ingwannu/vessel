//! The saved book's answers. Screens receive safe reasons, never file contents.
//!
//! [Decision Log]
//! - Purpose: Settings › Overview (how the book reaches the machine) and
//!   Settings › Connections (every route) draw from one read of the book.
//! - Choice: one read kept here ([`BookRead`]), asked when either page is
//!   shown for a machine it was not read for, asked again after an edit
//!   succeeds (the name may have changed), dropped when both are left.
//! - Cost: a page shown while the read is on its way draws without the
//!   book's part for a frame or two.
use crate::routes::Route;
use crate::shell::Shell;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::Command;
use vessel_app_link::machines::{self, Answer};
use vessel_app_view::settings::Section;

/// The book as last read for the machine pages.
#[derive(Default)]
pub(super) struct BookRead {
    book: Option<Vec<machines::Machine>>,
    /// The machine the book was last read for.
    read_for: Option<String>,
}

impl BookRead {
    /// The book's entry for `name`, once read.
    pub(super) fn entry(&self, name: &str) -> Option<&machines::Machine> {
        self.book
            .as_ref()?
            .iter()
            .find(|entry| entry.name.as_str() == name)
    }
}

impl Shell {
    /// Before drawing: read the book when a page that shows it is shown for
    /// a machine it was not read for; forget it when those pages are left.
    pub(crate) fn settle_book(&mut self, window: &Window) {
        let settings = &self.areas.settings;
        let shown = self.route == Route::Settings
            && matches!(settings.section, Section::Overview | Section::Connections)
            && (!vessel_app_kit::breakpoint::phone(window) || settings.pushed);
        let chosen = super::choices::chosen_machine(self)
            .filter(|_| shown)
            .map(|machine| machine.name.clone());
        let read = &mut self.areas.settings.book;
        if chosen.is_none() {
            *read = BookRead::default();
            return;
        }
        if read.read_for != chosen {
            read.read_for = chosen;
            self.command(Command::Machines(machines::Ask::Read));
        }
    }

    pub(crate) fn machines_answer(&mut self, answer: Answer, cx: &mut Context<'_, Self>) {
        // Settings › Overview says the answer to its own dialog's edit
        // there, not again as a notice.
        let said = self.overview_answer(&answer);
        match answer {
            Answer::Read(Ok(book)) => {
                // Kept only while a machine page is shown (read for).
                if self.areas.settings.book.read_for.is_some() {
                    self.areas.settings.book.book = Some(book);
                }
            }
            Answer::Read(Err(failure)) => {
                self.notice = Some(self.t.lang.machine_book_failure(&failure));
            }
            Answer::Edited { edit, result } => {
                self.areas.settings.machine =
                    edit.selected_after(&result, self.areas.settings.machine.as_deref());
                if let Err(failure) = &result {
                    if !said {
                        self.notice = Some(self.t.lang.machine_book_failure(failure));
                    }
                } else {
                    // The name may have changed under the page: read again.
                    self.areas.settings.book.read_for = None;
                    // Track F's state keyed by the machine's name follows
                    // the edit (the hook agreed with F).
                    self.workspace_book_applied(&edit, cx);
                }
            }
        }
    }
}
