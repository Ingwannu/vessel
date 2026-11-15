//! Apply the pure binding result to the window's field-owning area.

use super::Labels;
use vessel_app_view::{Fleet, labels::bind_hosts};

pub(super) fn bind(state: &mut Labels, fleet: &Fleet) {
    let bindings = bind_hosts(&state.hosts, fleet);
    state.hosts = bindings.hosts;
    state.unresolved = bindings.unresolved;
    if let Some(failure) = bindings.error {
        state.error = Some(failure);
    }
}
