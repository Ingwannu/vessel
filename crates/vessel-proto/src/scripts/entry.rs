//! A named command whose declared port cannot outlive its service kind.
//! Approval views borrow the full owner; execution never accepts a display view.

use serde::{Deserialize, Serialize};

use super::{ScriptKind, ScriptName};
use crate::workspace::{ProjectCommand, RelPath, ServicePort};

/// One script or service, with construction enforcing its declaration's meaning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(try_from = "RawScriptEntry")]
pub struct ScriptEntry {
    /// The file's parsed name, also used to find the command by name.
    pub name: ScriptName,
    /// Keep the role inseparable from its declared port.
    kind: ScriptKind,
    /// The validated text the approved digest covers.
    pub command: ProjectCommand,
    /// A folder inside the checkout; absence selects its root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dir: Option<RelPath>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// An approved override; absence leaves allocation to the runtime.
    port: Option<ServicePort>,
}

/// Why validated parts cannot form one declared script or service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidScriptEntry {
    /// A plain command cannot reserve a service port.
    #[error("only a service can declare a port")]
    PortOnScript,
}

/// Every approved field, borrowed for an exhaustive display or comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScriptEntryView<'a> {
    /// Identify which entry this display belongs to.
    pub name: &'a ScriptName,
    /// The execution role whose meaning the person approves.
    pub kind: ScriptKind,
    /// Show the exact command covered by the digest.
    pub command: &'a ProjectCommand,
    /// Show where the command runs, including an explicit root selection.
    pub dir: Option<&'a RelPath>,
    /// Show a declaration even when allocation would otherwise choose a port.
    pub port: Option<ServicePort>,
}

impl ScriptEntry {
    /// Keep kind and port consistent before an entry reaches approval or execution.
    ///
    /// # Errors
    /// [`InvalidScriptEntry::PortOnScript`] for a plain script with a port.
    pub fn new(
        name: ScriptName,
        kind: ScriptKind,
        command: ProjectCommand,
        dir: Option<RelPath>,
        port: Option<ServicePort>,
    ) -> Result<Self, InvalidScriptEntry> {
        match kind {
            ScriptKind::Script => {
                if port.is_some() {
                    return Err(InvalidScriptEntry::PortOnScript);
                }
            }
            ScriptKind::Service => {}
        }
        Ok(Self {
            name,
            kind,
            command,
            dir,
            port,
        })
    }

    /// Expose the role without allowing mutation to invalidate its port.
    #[must_use]
    pub const fn kind(&self) -> ScriptKind {
        self.kind
    }

    /// The declared override, distinct from the port an allocator may later choose.
    #[must_use]
    pub const fn port(&self) -> Option<ServicePort> {
        self.port
    }

    /// Force every approved field through one complete projection before display.
    #[must_use]
    pub const fn view(&self) -> ScriptEntryView<'_> {
        let Self {
            name,
            kind,
            command,
            dir,
            port,
        } = self;
        ScriptEntryView {
            name,
            kind: *kind,
            command,
            dir: dir.as_ref(),
            port: *port,
        }
    }
}

#[derive(Deserialize)]
struct RawScriptEntry {
    name: ScriptName,
    kind: ScriptKind,
    command: ProjectCommand,
    #[serde(default)]
    dir: Option<RelPath>,
    #[serde(default)]
    port: Option<ServicePort>,
}

impl TryFrom<RawScriptEntry> for ScriptEntry {
    type Error = InvalidScriptEntry;

    fn try_from(raw: RawScriptEntry) -> Result<Self, Self::Error> {
        Self::new(raw.name, raw.kind, raw.command, raw.dir, raw.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(kind: ScriptKind, port: Option<ServicePort>) -> ScriptEntry {
        ScriptEntry::new(
            "dev".parse().unwrap(),
            kind,
            "npm run dev".parse().unwrap(),
            None,
            port,
        )
        .unwrap()
    }

    #[test]
    fn construction_cannot_attach_a_port_to_a_plain_script() {
        let port = ServicePort::try_from(3210_u16).unwrap();
        let valid = entry(ScriptKind::Script, None);
        assert_eq!(
            ScriptEntry::new(
                valid.name,
                ScriptKind::Script,
                valid.command,
                valid.dir,
                Some(port)
            ),
            Err(InvalidScriptEntry::PortOnScript)
        );
        assert_eq!(entry(ScriptKind::Script, None).port(), None);
        assert_eq!(entry(ScriptKind::Service, Some(port)).port(), Some(port));
    }

    #[test]
    fn wire_decoding_preserves_the_kind_string_and_enforces_the_same_rule() {
        let legacy = r#"{"name":"dev","kind":"service","command":"npm run dev"}"#;
        let old: ScriptEntry = serde_json::from_str(legacy).unwrap();
        assert_eq!(old.kind(), ScriptKind::Service);
        assert_eq!(old.port(), None);
        assert_eq!(serde_json::to_string(&old).unwrap(), legacy);
        let declared = r#"{"name":"dev","kind":"service","command":"npm run dev","port":3210}"#;
        let parsed: ScriptEntry = serde_json::from_str(declared).unwrap();
        assert_eq!(serde_json::to_string(&parsed).unwrap(), declared);
        let invalid = r#"{"name":"dev","kind":"script","command":"npm run dev","port":3210}"#;
        let error = serde_json::from_str::<ScriptEntry>(invalid).unwrap_err();
        assert!(
            error
                .to_string()
                .contains(&InvalidScriptEntry::PortOnScript.to_string())
        );
    }

    #[test]
    fn the_display_view_borrows_every_approved_field() {
        let port = ServicePort::try_from(3210_u16).unwrap();
        let mut entry = entry(ScriptKind::Service, Some(port));
        entry.dir = Some("frontend".parse().unwrap());
        let ScriptEntryView {
            name,
            kind,
            command,
            dir,
            port: shown_port,
        } = entry.view();
        assert_eq!(name, &entry.name);
        assert_eq!(kind, entry.kind());
        assert_eq!(command, &entry.command);
        assert_eq!(dir, entry.dir.as_ref());
        assert_eq!(shown_port, Some(port));
    }
}
