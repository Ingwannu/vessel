//! Nonempty project allocation choices, before runtime precedence or probing.
//! A private decode edge keeps serde from bypassing the constructor's invariant.

use serde::{Deserialize, Serialize};

use super::{PortRange, PortScript};

/// A project overrides allocation only by providing at least one valid choice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(try_from = "RawServicePorts")]
pub struct ServicePorts {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    range: Option<PortRange>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    port_script: Option<PortScript>,
}

/// Every approved allocation choice, borrowed without exposing mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServicePortsView<'a> {
    /// Show the range even when the program has execution precedence.
    pub range: Option<PortRange>,
    /// Show the exact approved program, never a resolved filename.
    pub port_script: Option<&'a PortScript>,
}

/// Why an allocation block cannot replace the node's allocation choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidServicePorts {
    /// An empty override would discard defaults without selecting an allocator.
    #[error("service ports require a range or a port script")]
    Empty,
}

impl ServicePorts {
    /// Refuse an empty override even when callers already hold validated fields.
    ///
    /// # Errors
    /// [`InvalidServicePorts::Empty`] when neither choice is present.
    pub fn new(
        range: Option<PortRange>,
        port_script: Option<PortScript>,
    ) -> Result<Self, InvalidServicePorts> {
        if range.is_none() && port_script.is_none() {
            return Err(InvalidServicePorts::Empty);
        }
        Ok(Self { range, port_script })
    }

    /// Make a new approved field require a decision in every display.
    #[must_use]
    pub const fn view(&self) -> ServicePortsView<'_> {
        let Self { range, port_script } = self;
        ServicePortsView {
            range: *range,
            port_script: port_script.as_ref(),
        }
    }

    /// Retain the inclusive range, even when a port script takes precedence later.
    #[must_use]
    pub const fn range(&self) -> Option<PortRange> {
        self.range
    }

    /// Borrow the validated program without exposing a mutable allocation choice.
    #[must_use]
    pub const fn port_script(&self) -> Option<&PortScript> {
        self.port_script.as_ref()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawServicePorts {
    #[serde(default, deserialize_with = "present_choice")]
    range: Option<PortRange>,
    #[serde(default, deserialize_with = "present_choice")]
    port_script: Option<PortScript>,
}

// An omitted choice is optional; an explicitly present null is not a path/range.
fn present_choice<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

impl TryFrom<RawServicePorts> for ServicePorts {
    type Error = InvalidServicePorts;

    fn try_from(raw: RawServicePorts) -> Result<Self, Self::Error> {
        Self::new(raw.range, raw.port_script)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn either_choice_or_both_survive_validated_serde_round_trips() {
        let range: PortRange = "7000-7009".parse().unwrap();
        let script: PortScript = "tools/port".parse().unwrap();
        for (range, port_script) in [
            (Some(range), None),
            (None, Some(script.clone())),
            (Some(range), Some(script.clone())),
        ] {
            let ports = ServicePorts::new(range, port_script.clone()).unwrap();
            assert_eq!(ports.range(), range);
            assert_eq!(ports.port_script(), port_script.as_ref());
            let json = serde_json::to_string(&ports).unwrap();
            assert_eq!(serde_json::from_str::<ServicePorts>(&json).unwrap(), ports);
            if range.is_none() {
                assert!(!json.contains("range"));
            }
            if port_script.is_none() {
                assert!(!json.contains("port_script"));
            }
        }
    }

    #[test]
    fn empty_unknown_or_invalid_choices_cannot_bypass_the_constructor() {
        assert_eq!(
            ServicePorts::new(None, None),
            Err(InvalidServicePorts::Empty)
        );
        for json in [
            "{}",
            r#"{"range":null,"port_script":null}"#,
            r#"{"range":"7000-7009","extra":true}"#,
            r#"{"range":"7009-7000"}"#,
            r#"{"range":"7000-7009","port_script":null}"#,
            r#"{"range":null,"port_script":"tools/port"}"#,
            r#"{"range":"7000-7009","port_script":""}"#,
            r#"{"port_script":"../port"}"#,
        ] {
            assert!(
                serde_json::from_str::<ServicePorts>(json).is_err(),
                "{json}"
            );
        }
    }

    #[test]
    fn the_complete_view_keeps_each_present_or_absent_choice() {
        let range: PortRange = "7000-7009".parse().unwrap();
        let program: PortScript = "tools/port".parse().unwrap();
        let choices = [
            ServicePorts::new(Some(range), None).unwrap(),
            ServicePorts::new(None, Some(program.clone())).unwrap(),
            ServicePorts::new(Some(range), Some(program)).unwrap(),
        ];
        for ports in choices {
            let ServicePortsView { range, port_script } = ports.view();
            assert_eq!(range, ports.range());
            assert_eq!(port_script, ports.port_script());
        }
    }

    #[cfg(feature = "ts")]
    #[test]
    fn typescript_choices_admit_omission_without_admitting_null() {
        use ts_rs::TS as _;
        let declaration = ServicePorts::decl(&ts_rs::Config::new());
        assert!(declaration.contains("range?: PortRange"), "{declaration}");
        assert!(
            declaration.contains("port_script?: PortScript"),
            "{declaration}"
        );
        assert!(!declaration.contains("null"), "{declaration}");
    }
}
