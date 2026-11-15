//! Input is confined to the owned page and its current CSS viewport.
use super::{DeliveryState, Page, PageError};
use crate::cdp::requests::{InsertText, MouseEvent};
use vessel_proto::browser::{HumanInput, KeyModifiers};

impl Page {
    /// The node must first admit principal, resource and displayed-document revision.
    pub(crate) async fn human_input(&mut self, input: HumanInput) -> Result<(), PageError> {
        if self.delivery == DeliveryState::Uncertain {
            return Err(PageError::Uncertain);
        }
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
            // Keyboard/IME admission must not fall back to evaluated JavaScript
            // or literal text. The sealed key adapter is a separate next slice.
            HumanInput::Key { .. } => Err(PageError::Unsupported),
            HumanInput::ReleaseAll => {
                if let Some(position) = self.pointer_position {
                    for event in self.pointer.release_all(position).into_iter().flatten() {
                        let _released = self.execute(event).await?;
                    }
                }
                Ok(())
            }
        }
    }
}
