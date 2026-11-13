//! The saved book's answers. Screens receive safe reasons, never file contents.
use crate::shell::Shell;
use vessel_app_kit::ui::Context;
use vessel_app_link::machines::Answer;
impl Shell {
    pub(crate) fn machines_answer(&mut self, answer: Answer, _cx: &mut Context<'_, Self>) {
        match answer {
            Answer::Read(result) => {
                if let Err(failure) = result {
                    self.notice = Some(self.t.lang.machine_book_failure(failure).to_owned());
                }
            }
            Answer::Edited { edit, result } => match result {
                Ok(()) => {
                    self.areas.settings.machine =
                        edit.selected_after(self.areas.settings.machine.as_deref());
                }
                Err(failure) => {
                    self.notice = Some(self.t.lang.machine_book_failure(failure).to_owned());
                }
            },
        }
    }
}
