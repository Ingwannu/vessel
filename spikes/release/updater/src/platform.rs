//! Which build a copy of the app needs: its system and processor, and how
//! it was installed.
//!
//! Tauri's feed keys its artifacts by `<os>-<arch>` alone, because each
//! system has one installer. Vessel's Linux builds come two ways (a
//! tarball's bare binary, which is replaced as a file, and an `AppImage`,
//! which is one file too but sits wherever the user put it), and macOS
//! is a bundle, a directory. The swap differs per way, so an artifact is
//! keyed by both: a [`Target`] and a [`Packaging`]. The set of each is
//! closed: a new system or packaging is a new variant, and every `match`
//! on it must then say what it does.

use std::fmt;

use serde::{Deserialize, Serialize};

/// A system and processor the app is built for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Target {
    /// Windows 10 and 11 on x86-64.
    #[serde(rename = "windows-x86_64")]
    WindowsX86_64,
    /// Windows 11 on ARM.
    #[serde(rename = "windows-aarch64")]
    WindowsAarch64,
    /// Linux on x86-64 (glibc).
    #[serde(rename = "linux-x86_64")]
    LinuxX86_64,
    /// Linux on ARM64 (glibc).
    #[serde(rename = "linux-aarch64")]
    LinuxAarch64,
    /// macOS on Intel.
    #[serde(rename = "macos-x86_64")]
    MacosX86_64,
    /// macOS on Apple silicon.
    #[serde(rename = "macos-aarch64")]
    MacosAarch64,
}

impl Target {
    /// The target this binary was built for, if the app is released for it.
    #[must_use]
    pub const fn current() -> Option<Self> {
        if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
            Some(Self::WindowsX86_64)
        } else if cfg!(all(target_os = "windows", target_arch = "aarch64")) {
            Some(Self::WindowsAarch64)
        } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            Some(Self::LinuxX86_64)
        } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
            Some(Self::LinuxAarch64)
        } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
            Some(Self::MacosX86_64)
        } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            Some(Self::MacosAarch64)
        } else {
            None
        }
    }

    /// The name the feed and the artifacts' file names use.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WindowsX86_64 => "windows-x86_64",
            Self::WindowsAarch64 => "windows-aarch64",
            Self::LinuxX86_64 => "linux-x86_64",
            Self::LinuxAarch64 => "linux-aarch64",
            Self::MacosX86_64 => "macos-x86_64",
            Self::MacosAarch64 => "macos-aarch64",
        }
    }

    /// The packaging a copy on this target has when nothing else is known:
    /// a lone executable, except on macOS, where the app is a bundle.
    #[must_use]
    pub const fn default_packaging(self) -> Packaging {
        match self {
            Self::WindowsX86_64 | Self::WindowsAarch64 | Self::LinuxX86_64 | Self::LinuxAarch64 => {
                Packaging::Executable
            }
            Self::MacosX86_64 | Self::MacosAarch64 => Packaging::AppBundle,
        }
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for Target {
    type Err = UnknownTarget;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        [
            Self::WindowsX86_64,
            Self::WindowsAarch64,
            Self::LinuxX86_64,
            Self::LinuxAarch64,
            Self::MacosX86_64,
            Self::MacosAarch64,
        ]
        .into_iter()
        .find(|target| target.as_str() == text)
        .ok_or(UnknownTarget)
    }
}

/// A name that is no [`Target`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("not a target the app is released for")]
pub struct UnknownTarget;

/// How a copy of the app sits on disk, which decides what an update
/// replaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Packaging {
    /// One executable file, replaced by the artifact, which is the new
    /// executable itself (Windows' `.exe`; Linux's tarball binary).
    Executable,
    /// A Linux `AppImage`: one executable file wherever the user keeps it
    /// (`$APPIMAGE`), replaced by the artifact, the new `AppImage`.
    #[serde(rename = "appimage")]
    AppImage,
    /// A macOS `.app` directory, replaced by the one in the artifact, a
    /// `.tar.gz` of the bundle.
    #[serde(rename = "app-bundle")]
    AppBundle,
}

impl Packaging {
    /// The name the feed uses.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Executable => "executable",
            Self::AppImage => "appimage",
            Self::AppBundle => "app-bundle",
        }
    }
}

impl fmt::Display for Packaging {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for Packaging {
    type Err = UnknownPackaging;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        [Self::Executable, Self::AppImage, Self::AppBundle]
            .into_iter()
            .find(|packaging| packaging.as_str() == text)
            .ok_or(UnknownPackaging)
    }
}

/// A name that is no [`Packaging`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("not a packaging the app is released as")]
pub struct UnknownPackaging;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip_through_text_and_json() {
        for target in [
            Target::WindowsX86_64,
            Target::WindowsAarch64,
            Target::LinuxX86_64,
            Target::LinuxAarch64,
            Target::MacosX86_64,
            Target::MacosAarch64,
        ] {
            assert_eq!(target.as_str().parse::<Target>(), Ok(target));
            let json = serde_json::to_string(&target).unwrap();
            assert_eq!(json, format!("\"{target}\""));
        }
        for packaging in [
            Packaging::Executable,
            Packaging::AppImage,
            Packaging::AppBundle,
        ] {
            assert_eq!(packaging.as_str().parse::<Packaging>(), Ok(packaging));
            let json = serde_json::to_string(&packaging).unwrap();
            assert_eq!(json, format!("\"{packaging}\""));
        }
        assert!("windows".parse::<Target>().is_err());
        assert!("exe".parse::<Packaging>().is_err());
    }

    #[test]
    fn this_machine_is_a_target() {
        // The tests run on the systems the app is released for.
        assert!(Target::current().is_some());
    }
}
