//! Input is confined to the owned page and its current CSS viewport.
use super::{Page, PageError};
use crate::cdp::requests::{InsertText, MouseEvent};
use vessel_proto::browser::{HumanInput, KeyModifiers};

impl Page {
    /// The node must first admit principal, resource and displayed-document revision.
    pub(crate) async fn human_input(&mut self, input: HumanInput) -> Result<(), PageError> {
        self.active()?;
        match input {
            HumanInput::Pointer {
                position,
                action,
                modifiers,
            } => {
                if !position.inside(self.viewport) {
                    return Err(PageError::Surface);
                }
                self.pointer_position = Some(position);
                let event = MouseEvent::pointer(position, action, modifiers, &mut self.pointer);
                self.execute(event).await.map(|_| ())
            }
            HumanInput::Wheel {
                position,
                delta_x,
                delta_y,
            } => {
                if !position.inside(self.viewport) {
                    return Err(PageError::Surface);
                }
                self.pointer_position = Some(position);
                let event = MouseEvent::wheel(
                    position,
                    delta_x,
                    delta_y,
                    KeyModifiers::default(),
                    &self.pointer,
                );
                self.execute(event).await.map(|_| ())
            }
            HumanInput::Text { text } => self.execute(InsertText { text }).await.map(|_| ()),
            HumanInput::Key {
                key,
                action,
                modifiers,
            } => {
                let event = self.keyboard.event(&key, action, modifiers)?;
                self.execute(event).await.map(|_| ())
            }
            HumanInput::ReleaseAll => {
                if let Some(position) = self.pointer_position {
                    for event in self.pointer.release_all(position).into_iter().flatten() {
                        let _released = self.execute(event).await?;
                    }
                }
                for event in self.keyboard.release_all() {
                    let _released = self.execute(event).await?;
                }
                Ok(())
            }
        }
    }
}
