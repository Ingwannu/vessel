//! Where each target's command is looked for (Paseo's per-target
//! `commands()` and `runtime.ts` `resolveExecutable`): names on PATH, then
//! the platform's install paths; and the macOS app a target may open
//! through instead.

use std::path::{Path, PathBuf};

use super::{Platform, TargetId};

/// The variables a lookup reads, taken once from this computer's
/// environment by the app (tests make their own).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Env {
    /// `PATH` (Windows: `Path` or `path` too).
    pub path: Option<String>,
    /// `HOME`.
    pub home: Option<String>,
    /// `LOCALAPPDATA` (Windows).
    pub local_app_data: Option<String>,
    /// `ProgramFiles` (Windows).
    pub program_files: Option<String>,
}

/// One place a command may be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Candidate {
    /// A name looked for in each PATH folder.
    Name(&'static str),
    /// A full path.
    Path(PathBuf),
}

/// `base` joined with each of `parts`.
fn under(base: &str, parts: &[&str]) -> PathBuf {
    parts
        .iter()
        .fold(PathBuf::from(base), |path, part| path.join(part))
}

/// A macOS app's command at `/Applications/<app>` and `~/Applications/<app>`.
fn mac_paths(platform: Platform, env: &Env, inside: &[&str]) -> Vec<Candidate> {
    if platform != Platform::MacOs {
        return Vec::new();
    }
    let mut found = vec![Candidate::Path(under("/Applications", inside))];
    if let Some(home) = &env.home {
        let mut parts = vec!["Applications"];
        parts.extend_from_slice(inside);
        found.push(Candidate::Path(under(home, &parts)));
    }
    found
}

/// A Windows install's command under `LOCALAPPDATA` (with `Programs`) and
/// `ProgramFiles`.
fn windows_paths(platform: Platform, env: &Env, local: &[&str], program: &[&str]) -> Vec<Candidate> {
    if platform != Platform::Windows {
        return Vec::new();
    }
    let mut found = Vec::new();
    if let Some(base) = &env.local_app_data {
        found.push(Candidate::Path(under(base, local)));
    }
    if let Some(base) = &env.program_files {
        found.push(Candidate::Path(under(base, program)));
    }
    found
}

/// Where `target`'s command is looked for, in Paseo's order.
pub(super) fn candidates(target: TargetId, platform: Platform, env: &Env) -> Vec<Candidate> {
    let names = |names: &[&'static str]| -> Vec<Candidate> {
        names.iter().copied().map(Candidate::Name).collect()
    };
    match target {
        TargetId::Cursor => {
            let mut found = names(&["cursor"]);
            found.extend(mac_paths(
                platform,
                env,
                &["Cursor.app", "Contents", "Resources", "app", "bin", "cursor"],
            ));
            if platform == Platform::Windows {
                if let Some(base) = &env.local_app_data {
                    found.extend([
                        Candidate::Path(under(
                            base,
                            &["Programs", "cursor", "resources", "app", "bin", "cursor.cmd"],
                        )),
                        Candidate::Path(under(base, &["Programs", "cursor", "Cursor.exe"])),
                    ]);
                }
                if let Some(base) = &env.program_files {
                    found.extend([
                        Candidate::Path(under(
                            base,
                            &["Cursor", "resources", "app", "bin", "cursor.cmd"],
                        )),
                        Candidate::Path(under(base, &["Cursor", "Cursor.exe"])),
                    ]);
                }
            }
            found
        }
        TargetId::VsCode => {
            let mut found = names(&["code"]);
            found.extend(mac_paths(
                platform,
                env,
                &["Visual Studio Code.app", "Contents", "Resources", "app", "bin", "code"],
            ));
            found.extend(windows_paths(
                platform,
                env,
                &["Programs", "Microsoft VS Code", "bin", "code.cmd"],
                &["Microsoft VS Code", "bin", "code.cmd"],
            ));
            found
        }
        TargetId::VsCodeInsiders => {
            let mut found = names(&["code-insiders"]);
            found.extend(mac_paths(
                platform,
                env,
                &[
                    "Visual Studio Code - Insiders.app",
                    "Contents",
                    "Resources",
                    "app",
                    "bin",
                    "code",
                ],
            ));
            found.extend(windows_paths(
                platform,
                env,
                &["Programs", "Microsoft VS Code Insiders", "bin", "code-insiders.cmd"],
                &["Microsoft VS Code Insiders", "bin", "code-insiders.cmd"],
            ));
            found
        }
        TargetId::VsCodium => {
            let mut found = names(&["codium"]);
            found.extend(mac_paths(
                platform,
                env,
                &["VSCodium.app", "Contents", "Resources", "app", "bin", "codium"],
            ));
            found.extend(windows_paths(
                platform,
                env,
                &["Programs", "VSCodium", "bin", "codium.cmd"],
                &["VSCodium", "bin", "codium.cmd"],
            ));
            found
        }
        TargetId::AndroidStudio => {
            let mut found = names(&["studio", "studio64"]);
            found.extend(mac_paths(
                platform,
                env,
                &["Android Studio.app", "Contents", "MacOS", "studio"],
            ));
            found
        }
        TargetId::Trae => names(&["trae"]),
        TargetId::Kiro => names(&["kiro"]),
        TargetId::Zed => names(&["zed", "zeditor"]),
        TargetId::Antigravity => names(&["agy", "antigravity"]),
        TargetId::IntellijIdea => names(&["idea", "idea64"]),
        TargetId::Aqua => names(&["aqua", "aqua64"]),
        TargetId::Clion => names(&["clion", "clion64"]),
        TargetId::DataGrip => names(&["datagrip", "datagrip64"]),
        TargetId::DataSpell => names(&["dataspell", "dataspell64"]),
        TargetId::GoLand => names(&["goland", "goland64"]),
        TargetId::PhpStorm => names(&["phpstorm", "phpstorm64"]),
        TargetId::PyCharm => names(&["pycharm", "pycharm64"]),
        TargetId::Rider => names(&["rider", "rider64"]),
        TargetId::RubyMine => names(&["rubymine", "rubymine64"]),
        TargetId::RustRover => names(&["rustrover", "rustrover64"]),
        TargetId::WebStorm => names(&["webstorm", "webstorm64"]),
        TargetId::Xcode | TargetId::Finder | TargetId::Explorer | TargetId::FileManager => {
            Vec::new()
        }
    }
}

/// The macOS app a target opens through when it has no command
/// (`hasMacApplication` / `openMacApplication`).
pub(super) const fn mac_app(target: TargetId) -> Option<&'static str> {
    match target {
        TargetId::Cursor => Some("Cursor"),
        TargetId::VsCode => Some("Visual Studio Code"),
        TargetId::VsCodeInsiders => Some("Visual Studio Code - Insiders"),
        TargetId::VsCodium => Some("VSCodium"),
        TargetId::Zed => Some("Zed"),
        TargetId::Xcode => Some("Xcode"),
        TargetId::Trae
        | TargetId::Kiro
        | TargetId::Antigravity
        | TargetId::AndroidStudio
        | TargetId::IntellijIdea
        | TargetId::Aqua
        | TargetId::Clion
        | TargetId::DataGrip
        | TargetId::DataSpell
        | TargetId::GoLand
        | TargetId::PhpStorm
        | TargetId::PyCharm
        | TargetId::Rider
        | TargetId::RubyMine
        | TargetId::RustRover
        | TargetId::WebStorm
        | TargetId::Finder
        | TargetId::Explorer
        | TargetId::FileManager => None,
    }
}

/// Where a macOS app bundle may be (`runtime.ts`
/// `resolveMacApplicationBundle`).
pub(super) fn mac_bundles(app: &str, platform: Platform, env: &Env) -> Vec<PathBuf> {
    if platform != Platform::MacOs {
        return Vec::new();
    }
    let bundle = format!("{app}.app");
    let mut found = vec![Path::new("/Applications").join(&bundle)];
    if let Some(home) = &env.home {
        found.push(Path::new(home).join("Applications").join(&bundle));
    }
    found.push(Path::new("/System/Applications").join(&bundle));
    found
}

/// The PATH folders, split as the platform does.
pub(super) fn path_folders(platform: Platform, env: &Env) -> Vec<PathBuf> {
    let delimiter = if platform == Platform::Windows { ';' } else { ':' };
    env.path
        .as_deref()
        .unwrap_or_default()
        .split(delimiter)
        .filter(|folder| !folder.is_empty())
        .map(PathBuf::from)
        .collect()
}

/// A name's forms on this platform: Windows tries `.exe .cmd .bat .com`
/// and then the bare name, unless it already has an extension.
pub(super) fn name_forms(name: &str, platform: Platform) -> Vec<String> {
    if platform != Platform::Windows || Path::new(name).extension().is_some() {
        return vec![name.to_owned()];
    }
    [".exe", ".cmd", ".bat", ".com", ""]
        .into_iter()
        .map(|extension| format!("{name}{extension}"))
        .collect()
}
