//! The saved book's answers. Screens receive safe reasons, never file contents.
use crate::shell::Shell;
use vessel_app_kit::ui::Context;
use vessel_app_link::machines::Answer;
impl Shell {
    pub(crate) fn machines_answer(&mut self, answer: Answer, cx: &mut Context<'_, Self>) {
        match answer {
            Answer::Read(result) => {
                if let Err(failure) = result {
                    self.notice = Some(self.t.lang.machine_book_failure(&failure));
                }
            }
            Answer::Edited { edit, result } => {
                self.areas.settings.machine =
                    edit.selected_after(&result, self.areas.settings.machine.as_deref());
                if let Err(failure) = &result {
                    self.notice = Some(self.t.lang.machine_book_failure(failure));
                } else {
                    // Track F's state keyed by the machine's name follows
                    // the edit (the hook agreed with F).
                    self.workspace_book_applied(&edit, cx);
                }
            }
        }
    }
}
