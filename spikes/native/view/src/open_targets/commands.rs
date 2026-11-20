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

/// `base` joined with each `/`-separated part of `rel`.
fn under(base: &str, rel: &str) -> PathBuf {
    rel.split('/')
        .fold(PathBuf::from(base), |path, part| path.join(part))
}

/// Where a target is installed besides PATH.
struct Installs {
    /// Inside `/Applications` and `~/Applications` (macOS).
    mac: Option<&'static str>,
    /// Under `LOCALAPPDATA` (Windows).
    local: &'static [&'static str],
    /// Under `ProgramFiles` (Windows).
    program: &'static [&'static str],
}

/// None besides PATH.
const NOWHERE: Installs = Installs {
    mac: None,
    local: &[],
    program: &[],
};

/// The names looked for on PATH (Paseo's `COMMANDS` and the first entries
/// of its `commands()`).
const fn names(target: TargetId) -> &'static [&'static str] {
    match target {
        TargetId::Cursor => &["cursor"],
        TargetId::Trae => &["trae"],
        TargetId::Kiro => &["kiro"],
        TargetId::VsCode => &["code"],
        TargetId::VsCodeInsiders => &["code-insiders"],
        TargetId::VsCodium => &["codium"],
        TargetId::Zed => &["zed", "zeditor"],
        TargetId::Antigravity => &["agy", "antigravity"],
        TargetId::AndroidStudio => &["studio", "studio64"],
        TargetId::IntellijIdea => &["idea", "idea64"],
        TargetId::Aqua => &["aqua", "aqua64"],
        TargetId::Clion => &["clion", "clion64"],
        TargetId::DataGrip => &["datagrip", "datagrip64"],
        TargetId::DataSpell => &["dataspell", "dataspell64"],
        TargetId::GoLand => &["goland", "goland64"],
        TargetId::PhpStorm => &["phpstorm", "phpstorm64"],
        TargetId::PyCharm => &["pycharm", "pycharm64"],
        TargetId::Rider => &["rider", "rider64"],
        TargetId::RubyMine => &["rubymine", "rubymine64"],
        TargetId::RustRover => &["rustrover", "rustrover64"],
        TargetId::WebStorm => &["webstorm", "webstorm64"],
        TargetId::Xcode | TargetId::Finder | TargetId::Explorer | TargetId::FileManager => &[],
    }
}

/// The install paths Paseo's `commands()` adds after the names.
const fn installs(target: TargetId) -> Installs {
    match target {
        TargetId::Cursor => Installs {
            mac: Some("Cursor.app/Contents/Resources/app/bin/cursor"),
            local: &[
                "Programs/cursor/resources/app/bin/cursor.cmd",
                "Programs/cursor/Cursor.exe",
            ],
            program: &["Cursor/resources/app/bin/cursor.cmd", "Cursor/Cursor.exe"],
        },
        TargetId::VsCode => Installs {
            mac: Some("Visual Studio Code.app/Contents/Resources/app/bin/code"),
            local: &["Programs/Microsoft VS Code/bin/code.cmd"],
            program: &["Microsoft VS Code/bin/code.cmd"],
        },
        TargetId::VsCodeInsiders => Installs {
            mac: Some("Visual Studio Code - Insiders.app/Contents/Resources/app/bin/code"),
            local: &["Programs/Microsoft VS Code Insiders/bin/code-insiders.cmd"],
            program: &["Microsoft VS Code Insiders/bin/code-insiders.cmd"],
        },
        TargetId::VsCodium => Installs {
            mac: Some("VSCodium.app/Contents/Resources/app/bin/codium"),
            local: &["Programs/VSCodium/bin/codium.cmd"],
            program: &["VSCodium/bin/codium.cmd"],
        },
        TargetId::AndroidStudio => Installs {
            mac: Some("Android Studio.app/Contents/MacOS/studio"),
            ..NOWHERE
        },
        TargetId::Trae
        | TargetId::Kiro
        | TargetId::Zed
        | TargetId::Antigravity
        | TargetId::Xcode
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
        | TargetId::FileManager => NOWHERE,
    }
}

/// Where `target`'s command is looked for, in Paseo's order: its names on
/// PATH, then (macOS) `/Applications` and `~/Applications`, then (Windows)
/// `LOCALAPPDATA` and `ProgramFiles`.
pub(super) fn candidates(target: TargetId, platform: Platform, env: &Env) -> Vec<Candidate> {
    let mut found: Vec<Candidate> = names(target).iter().copied().map(Candidate::Name).collect();
    let installs = installs(target);
    if let (Platform::MacOs, Some(inside)) = (platform, installs.mac) {
        found.push(Candidate::Path(under("/Applications", inside)));
        if let Some(home) = &env.home {
            found.push(Candidate::Path(under(
                home,
                &format!("Applications/{inside}"),
            )));
        }
    }
    if platform == Platform::Windows {
        for (base, paths) in [
            (&env.local_app_data, installs.local),
            (&env.program_files, installs.program),
        ] {
            if let Some(base) = base {
                found.extend(paths.iter().map(|rel| Candidate::Path(under(base, rel))));
            }
        }
    }
    found
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

/// The PATH folders, split as the platform does. A relative folder (`.`,
/// `bin`) would resolve against the app's working directory, not a place
/// the person installed an editor, so it is left out (Paseo keeps it).
pub(super) fn path_folders(platform: Platform, env: &Env) -> Vec<PathBuf> {
    let delimiter = if platform == Platform::Windows {
        ';'
    } else {
        ':'
    };
    env.path
        .as_deref()
        .unwrap_or_default()
        .split(delimiter)
        .filter(|folder| absolute(platform, folder))
        .map(PathBuf::from)
        .collect()
}

/// Whether `folder` is absolute on `platform` (`/…`; Windows `C:\…`, `C:/…`
/// or `\\server\…`), read as text so the rule is the same on every host.
fn absolute(platform: Platform, folder: &str) -> bool {
    if platform != Platform::Windows {
        return folder.starts_with('/');
    }
    let mut chars = folder.chars();
    let drive = chars
        .next()
        .is_some_and(|letter| letter.is_ascii_alphabetic())
        && chars.next() == Some(':')
        && matches!(chars.next(), Some('\\' | '/'));
    drive || folder.starts_with(r"\\")
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
