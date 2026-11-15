//! Release companion locations follow ADR 0094; discovery never starts a tool.

use std::path::{Path, PathBuf};

use super::{DesktopError, cli::path_setup::Platform};

/// The release's closed set of companions shares one location policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundledTool {
    /// The command-line client installed on the user's PATH.
    Cli,
    /// The managed local node, launched only by its lifecycle owner.
    Node,
}

/// Locate a companion without assuming the release actually contains it.
///
/// The caller checks availability separately: older/portable installations may
/// legitimately lack either file. Keeping both tools in `bin/` avoids Windows'
/// case-insensitive `Vessel.exe`/`vessel.exe` collision and lets release updates
/// replace a coherent set. macOS has no published signed bundle layout yet.
///
/// # Errors
/// Refuses a relative app executable and the unassigned macOS bundle layout.
pub fn bundled_tool_path(
    executable: &Path,
    platform: Platform,
    tool: BundledTool,
) -> Result<PathBuf, DesktopError> {
    if !executable.is_absolute() {
        return Err(DesktopError::RelativePath);
    }
    let name = match (platform, tool) {
        (Platform::Windows, BundledTool::Cli) => "vessel.exe",
        (Platform::Windows, BundledTool::Node) => "vesseld.exe",
        (Platform::Unix, BundledTool::Cli) => "vessel",
        (Platform::Unix, BundledTool::Node) => "vesseld",
        (Platform::Mac, BundledTool::Cli | BundledTool::Node) => {
            return Err(DesktopError::UnsupportedBundle);
        }
    };
    let directory = executable.parent().ok_or(DesktopError::RelativePath)?;
    Ok(directory.join("bin").join(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_companions_use_the_release_bin_directory_without_requiring_files() {
        let directory = std::env::temp_dir().join("vessel-bundle-policy");
        for (platform, app, cli, node) in [
            (Platform::Unix, "vessel-app", "vessel", "vesseld"),
            (Platform::Windows, "Vessel.exe", "vessel.exe", "vesseld.exe"),
        ] {
            let executable = directory.join(app);
            for (tool, name) in [(BundledTool::Cli, cli), (BundledTool::Node, node)] {
                assert_eq!(
                    bundled_tool_path(&executable, platform, tool).unwrap(),
                    directory.join("bin").join(name)
                );
            }
        }
        assert!(matches!(
            bundled_tool_path(&directory.join("Vessel"), Platform::Mac, BundledTool::Cli),
            Err(DesktopError::UnsupportedBundle)
        ));
        assert!(matches!(
            bundled_tool_path(Path::new("relative"), Platform::Unix, BundledTool::Node),
            Err(DesktopError::RelativePath)
        ));
    }
}
