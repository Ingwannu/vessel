//! Where each settings picker's trigger was last drawn, so its list opens
//! under it (moved out of the settings area's root module to keep it within
//! the size budget; no change in behaviour).

use vessel_app_kit::select::Anchor;

use super::List;

/// Where each trigger was last drawn.
#[derive(Default)]
pub(crate) struct Anchors {
    machine: Anchor,
    send: Anchor,
    delivery: Anchor,
    interface: Anchor,
    code: Anchor,
    width: Anchor,
    display: Anchor,
    alerts: Anchor,
}

impl Anchors {
    pub(crate) const fn of(&self, list: List) -> &Anchor {
        match list {
            List::Machine => &self.machine,
            List::Send => &self.send,
            List::Delivery => &self.delivery,
            List::Interface => &self.interface,
            List::Code => &self.code,
            List::Width => &self.width,
            List::Display => &self.display,
            List::Alerts => &self.alerts,
        }
    }
}
