//! A session around its timeline: its title, its marks and the actions
//! on them, when it marks itself read.

mod delivery;
mod marks;
mod parking;
mod titles;

pub use delivery::{SendFacts, send_delivery, shows_at_once};
pub use marks::{
    CONVERSATION_PROTOCOL, ENDS_BESIDE_PROTOCOL, MarkAction, ReadView, can_start_again,
    is_archived, is_live, mark_actions, read_through,
};
pub use parking::{ParkState, park_state};
pub use titles::{
    can_rename, check_title, folder_name, has_title, session_name, title_from_prompt,
};
