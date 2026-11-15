//! A rejected local lifecycle command proves no send; it is not a peer refusal.
//! Keep this construction beside the exhaustive shared dispatcher so adding
//! session correlation does not grow its cross-feature settle function.
use vessel_app_link::{
    Event,
    sessions::lifecycle::{Answer, Failure},
};
use vessel_app_view::sessions::LifecycleTicket;

pub(super) fn lifecycle(ticket: LifecycleTicket) -> Event {
    Event::SessionLifecycle(Answer {
        ticket,
        result: Err(Failure::Unsent),
    })
}
