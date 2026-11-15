//! Complete display-only comparison of approved project allocation choices.
//! A project allocation choice never represents an observed listening socket.
use vessel_proto::workspace::{PortRange, PortScript, ServicePortsView};

/// Owned, display-only copy of every approved allocation choice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllocationFields {
    range: Option<PortRange>,
    port_script: Option<PortScript>,
}

impl AllocationFields {
    /// Projection is exhaustive so a new approved choice cannot disappear.
    #[must_use]
    pub fn of(view: ServicePortsView<'_>) -> Self {
        let ServicePortsView { range, port_script } = view;
        Self {
            range,
            port_script: port_script.cloned(),
        }
    }

    /// Rendering uses exactly the approved spelling, never a resolved executable.
    #[must_use]
    pub fn view(&self) -> ServicePortsView<'_> {
        let Self { range, port_script } = self;
        ServicePortsView {
            range: *range,
            port_script: port_script.as_ref(),
        }
    }
}

/// Comparing the whole block preserves both choices, including removal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AllocationLine {
    /// Neither set overrides project allocation.
    None,
    /// Same complete override.
    Same(AllocationFields),
    /// Introduced even if there are no named services.
    Added(AllocationFields),
    /// The project now returns to defaults; show the removed exact choices.
    Removed(AllocationFields),
    /// Both complete sets, including choices with lower runtime precedence.
    Changed {
        /// Current declared allocation choices.
        now: AllocationFields,
        /// Previously approved choices, including removed values.
        was: AllocationFields,
    },
}

impl AllocationLine {
    /// Summary text must not claim no executable changes for a block-only edit.
    #[must_use]
    pub const fn changed(&self) -> bool {
        match self {
            Self::None | Self::Same(_) => false,
            Self::Added(_) | Self::Removed(_) | Self::Changed { now: _, was: _ } => true,
        }
    }
}

/// A missing choice remains missing; no allocator default is fabricated here.
#[must_use]
pub fn compare(
    before: Option<ServicePortsView<'_>>,
    after: Option<ServicePortsView<'_>>,
) -> AllocationLine {
    let before = before.map(AllocationFields::of);
    let after = after.map(AllocationFields::of);
    match (before, after) {
        (None, None) => AllocationLine::None,
        (None, Some(now)) => AllocationLine::Added(now),
        (Some(was), None) => AllocationLine::Removed(was),
        (Some(was), Some(now)) if was == now => AllocationLine::Same(now),
        (Some(was), Some(now)) => AllocationLine::Changed { now, was },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::workspace::ServicePorts;

    #[test]
    fn a_lower_precedence_range_edit_remains_visible_beside_an_unchanged_helper() {
        let helper: PortScript = "tools/port".parse().unwrap();
        let old =
            ServicePorts::new(Some("7000-7010".parse().unwrap()), Some(helper.clone())).unwrap();
        let new = ServicePorts::new(Some("9000-9010".parse().unwrap()), Some(helper)).unwrap();
        let AllocationLine::Changed { now, was } = compare(Some(old.view()), Some(new.view()))
        else {
            panic!()
        };
        assert_eq!(was.view(), old.view());
        assert_eq!(now.view(), new.view());
        assert_ne!(was.view().range, now.view().range);
        assert_eq!(was.view().port_script, now.view().port_script);
    }

    #[test]
    fn adding_and_removing_a_standalone_block_keeps_all_declared_choices() {
        let ports = ServicePorts::new(None, Some("tools/port".parse().unwrap())).unwrap();
        let AllocationLine::Added(added) = compare(None, Some(ports.view())) else {
            panic!()
        };
        let AllocationLine::Removed(removed) = compare(Some(ports.view()), None) else {
            panic!()
        };
        assert_eq!(added.view(), ports.view());
        assert_eq!(removed.view(), ports.view());
        assert!(!compare(Some(ports.view()), Some(ports.view())).changed());
        assert_eq!(compare(None, None), AllocationLine::None);
    }
}
