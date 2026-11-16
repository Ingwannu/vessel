//! Apply the pure binding result to the window's field-owning area.

use super::Labels;
use vessel_app_view::{Fleet, labels::bind_hosts};

pub(super) fn bind(state: &mut Labels, fleet: &Fleet) {
    let bindings = bind_hosts(&state.hosts, fleet);
    state.hosts = bindings.hosts;
    // SCRATCH, capture-only and never for adoption: the node does not serve
    // label catalogs yet, so the visual lane seeds a fixed catalog to show the
    // manager's listing, filtering and editing on a fake fleet.
    if std::env::var_os("VESSEL_APP_CAPTURE_LABEL_FIXTURE").is_some() {
        use vessel_app_view::labels::{Catalog, Color, HostState, Label};
        let fixture = [
            ("Ready", Color::Violet),
            ("Blocked", Color::Orange),
            ("Needs review", Color::Sky),
            ("Design", Color::Pink),
            ("Backend", Color::Emerald),
            ("Release candidate", Color::Indigo),
            ("검토 필요", Color::Teal),
        ];
        for host in &mut state.hosts {
            host.state = HostState::Online;
            host.catalog = Catalog::snapshot(fixture.iter().map(|(name, color)| Label {
                name: name.parse().unwrap(),
                color: *color,
            }));
        }
    }
    state.unresolved = bindings.unresolved;
    if !state.manager.state().busy()
        && state.manager.state().draft().is_none()
        && state.selected_host().is_none()
        && let Some(route) = state.hosts.first().map(|host| host.route.clone())
        && let Err(failure) = state.select_route(&route)
    {
        state.error = Some(failure);
    }
    if let Some(failure) = bindings.error {
        state.error = Some(failure);
    }
}

impl Labels {
    pub(super) fn selected_host(&self) -> Option<&vessel_app_view::labels::Host> {
        self.hosts.iter().find(|host| {
            Some(&host.route) == self.route.as_ref() && self.manager.selected() == Some(&host.key)
        })
    }

    pub(super) fn select_route(
        &mut self,
        route: &vessel_app_view::machine_book::Name,
    ) -> Result<(), vessel_app_view::labels::Failure> {
        let host = self
            .hosts
            .iter()
            .find(|host| &host.route == route)
            .ok_or(vessel_app_view::labels::Failure::Offline)?;
        if self.route.as_ref() != Some(route) || self.manager.selected() != Some(&host.key) {
            self.manager.back()?;
            self.manager.select(host.key)?;
            self.route = Some(route.clone());
        }
        Ok(())
    }
}
