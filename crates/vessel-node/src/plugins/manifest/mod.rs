//! Decode a bounded native author-port manifest before filesystem preparation.
//! Package paths remain untrusted until the candidate owner proves canonical
//! containment. This module does no file/network/process work.
mod inspection;
mod paths;
#[cfg(test)]
mod tests;

use paths::{Media, PackagePath};
use serde::Deserialize;
use std::{collections::BTreeSet, fmt};
use vessel_proto::{
    ProtocolVersion,
    plugins::{Caption, ContentText, HelperProtocol, PluginId, PluginJsonError},
};

const MAX_MANIFEST_BYTES: usize = 64 * 1024;
const MAX_BUILD_COMMANDS: usize = 16;
const MAX_ARGUMENTS: usize = 64;
const MAX_ARGUMENT_BYTES: usize = 16 * 1024;
const MAX_TOTAL_ARGUMENT_BYTES: usize = 32 * 1024;
const MAX_MEDIA: usize = 16;
const MAX_TARGETS: usize = 8;

/// Private format version, independent of public method-number assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
enum Format {
    #[serde(rename = "vessel.plugin/v1")]
    V1,
}

/// Exact executable ABI target; no fallback to another architecture/libc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
enum Target {
    #[serde(rename = "x86_64-unknown-linux-gnu")]
    LinuxX64,
    #[serde(rename = "aarch64-unknown-linux-gnu")]
    LinuxArm64,
    #[serde(rename = "x86_64-unknown-linux-musl")]
    LinuxMuslX64,
    #[serde(rename = "aarch64-unknown-linux-musl")]
    LinuxMuslArm64,
    #[serde(rename = "x86_64-apple-darwin")]
    MacX64,
    #[serde(rename = "aarch64-apple-darwin")]
    MacArm64,
    #[serde(rename = "x86_64-pc-windows-msvc")]
    WindowsX64,
    #[serde(rename = "aarch64-pc-windows-msvc")]
    WindowsArm64,
}

impl Target {
    // R's release contract: broader labels are author data, not a support
    // claim. A third eligible target must first join R's qualified matrix.
    fn eligible(self) -> bool {
        matches!(self, Self::LinuxX64 | Self::WindowsX64)
    }

    fn compiled() -> Result<Self, ManifestError> {
        // Node build.rs exports Cargo TARGET including libc/ABI. The updater's
        // OS/arch-only feed Target and SSH discovery cannot establish this.
        match env!("VESSEL_TARGET") {
            "x86_64-unknown-linux-gnu" => Ok(Self::LinuxX64),
            "x86_64-pc-windows-msvc" => Ok(Self::WindowsX64),
            _unsupported_build_triple => Err(ManifestError::UnsupportedTarget),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Requirements {
    node: ProtocolVersion,
    client: ProtocolVersion,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Helper {
    target: Target,
    protocol: HelperProtocol,
    program: PackagePath,
    #[serde(default)]
    args: Vec<String>,
}
impl fmt::Debug for Helper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Helper")
            .field("target", &self.target)
            .field("protocol", &self.protocol)
            .finish_non_exhaustive()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestInput {
    format: Format,
    id: PluginId,
    name: Option<Caption>,
    description: Option<ContentText>,
    icon: Option<PackagePath>,
    #[serde(default)]
    media: Vec<Media>,
    requirements: Requirements,
    client: Option<PackagePath>,
    #[serde(default)]
    helpers: Vec<Helper>,
    #[serde(default)]
    build: Vec<Vec<String>>,
}
struct Manifest(ManifestInput);

impl fmt::Debug for Manifest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Manifest")
            .field("format", &self.0.format)
            .field("id", &self.0.id)
            .field("requirements", &self.0.requirements)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, thiserror::Error)]
enum ManifestError {
    #[error("plugin manifest exceeds its byte budget")]
    Bytes,
    #[error("invalid native plugin manifest JSON")]
    Decode(#[source] PluginJsonError),
    #[error("plugin manifest requires a client document or helper")]
    NoEntry,
    #[error("plugin manifest exceeds its declaration budget")]
    Capacity,
    #[error("plugin manifest contains duplicate executable targets")]
    DuplicateTarget,
    #[error("plugin manifest arguments are empty or contain unsupported characters")]
    Argument,
    #[error("plugin manifest icon must name a package PNG")]
    Icon,
    #[error("plugin manifest protocol requirements are incompatible")]
    Incompatible,
    #[error("plugin manifest has no executable for this target")]
    UnsupportedTarget,
}

impl Manifest {
    fn decode(bytes: &[u8]) -> Result<Self, ManifestError> {
        if bytes.len() > MAX_MANIFEST_BYTES {
            return Err(ManifestError::Bytes);
        }
        let input: ManifestInput = serde_json::from_slice(bytes)
            .map_err(|cause| ManifestError::Decode(PluginJsonError::new(cause)))?;
        let manifest = Self(input);
        manifest.validate()?;
        Ok(manifest)
    }

    fn validate(&self) -> Result<(), ManifestError> {
        if self.0.client.is_none() && self.0.helpers.is_empty() {
            return Err(ManifestError::NoEntry);
        }
        if self.0.helpers.len() > MAX_TARGETS
            || self.0.media.len() > MAX_MEDIA
            || self.0.build.len() > MAX_BUILD_COMMANDS
        {
            return Err(ManifestError::Capacity);
        }
        if self.0.requirements.node.get() == 0 || self.0.requirements.client.get() == 0 {
            return Err(ManifestError::Incompatible);
        }
        if self
            .0
            .icon
            .as_ref()
            .is_some_and(|path| !path.as_str().to_ascii_lowercase().ends_with(".png"))
        {
            return Err(ManifestError::Icon);
        }
        let mut targets = BTreeSet::new();
        for helper in &self.0.helpers {
            if !targets.insert(helper.target) {
                return Err(ManifestError::DuplicateTarget);
            }
            arguments(&helper.args)?;
        }
        for command in &self.0.build {
            if command.is_empty() {
                return Err(ManifestError::Argument);
            }
            arguments(command)?;
        }
        Ok(())
    }

    fn compatible(
        &self,
        node: ProtocolVersion,
        client: ProtocolVersion,
    ) -> Result<(), ManifestError> {
        if self.0.requirements.node > node || self.0.requirements.client > client {
            return Err(ManifestError::Incompatible);
        }
        Ok(())
    }

    fn helper(&self, target: Target) -> Result<&Helper, ManifestError> {
        if !target.eligible() {
            return Err(ManifestError::UnsupportedTarget);
        }
        self.0
            .helpers
            .iter()
            .find(|helper| helper.target == target)
            .ok_or(ManifestError::UnsupportedTarget)
    }
}

fn arguments(arguments: &[String]) -> Result<(), ManifestError> {
    if arguments.len() > MAX_ARGUMENTS {
        return Err(ManifestError::Capacity);
    }
    let mut bytes = 0_usize;
    for arg in arguments {
        bytes = bytes.saturating_add(arg.len());
        if arg.len() > MAX_ARGUMENT_BYTES || bytes > MAX_TOTAL_ARGUMENT_BYTES {
            return Err(ManifestError::Capacity);
        }
        if vessel_proto::workspace::trim_project_text(arg).is_empty()
            || arg.chars().any(char::is_control)
        {
            return Err(ManifestError::Argument);
        }
    }
    Ok(())
}
