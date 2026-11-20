//! Open in editor and reveal in the file manager (TRACKER 467, ADR 0123):
//! Paseo's desktop editor targets (`desktop/src/features/editor-targets/`,
//! pinned 0.15.4), as pure rules. The app hands in what it read from this
//! computer (a [`Probe`]) and starts what [`plan`] says.
//!
//! [Decision Log]
//! - Purpose: open the workspace, a folder of it or the file shown in the
//!   person's editor, and reveal an entry in the file manager, as Paseo's
//!   desktop app does from its workspace header, its Files menu and its
//!   sidebar.
//! - Constraint: it only makes sense for a machine whose node runs on
//!   this computer (816's `ThisComputer`), with a path that exists here.
//!   An editor is a program the person installed; Vessel starts it, with
//!   arguments made only from absolute canonical paths under the shown
//!   workspace (the coordinator's conditions for ADR 0123).
//! - Alternatives: the system's "open with" for everything (no line, no
//!   choice of editor; Paseo lists its 25 targets); a target list kept in
//!   the app (no tests without a window).
//! - Choice: Paseo's 25 targets in its order with its ids, labels,
//!   command names, install paths and argument shapes, here with no I/O:
//!   [`listing::installed`] and [`listing::list`] read through a [`Probe`],
//!   [`guard::checked`] canonicalises and refuses paths outside the root,
//!   [`launch::plan`] says what to start. The app owns the process seam.
//! - Cost: Paseo's list is copied, so a target Paseo adds later is added
//!   here by hand (one arm per table).

mod commands;
pub mod forge;
pub mod guard;
pub mod launch;
pub mod listing;

#[cfg(test)]
mod tests;

pub use commands::Env;
pub use forge::{Forge, LinkedFile, forge_of, page};
pub use guard::{Checked, Refused, Request, checked};
pub use launch::{Plan, Unavailable, plan};
pub use listing::{Entry, EntryKind, Probe, installed, list};

/// The platform's rules (Paseo's `darwin`, `win32` and the rest).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// macOS: app bundles, Finder.
    MacOs,
    /// Windows: `.exe`/`.cmd` on PATH, Explorer.
    Windows,
    /// Linux and the rest: the desktop's file manager.
    Other,
}

impl Platform {
    /// The platform this build runs on.
    #[must_use]
    pub const fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::MacOs
        } else if cfg!(windows) {
            Self::Windows
        } else {
            Self::Other
        }
    }
}

/// An editor or a file manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// It opens a folder and a file at a line.
    Editor,
    /// It reveals an entry or opens a folder.
    FileManager,
}

/// An official mark Vessel embeds (`kit::agent_mark`'s editor marks;
/// sources and terms in `docs/design/editor-marks/README.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorMark {
    /// Cursor.
    Cursor,
    /// Visual Studio Code.
    VsCode,
    /// Zed.
    Zed,
    /// Android Studio.
    AndroidStudio,
    /// Xcode.
    Xcode,
    /// `WebStorm`.
    WebStorm,
}

/// What a target is drawn with: its mark, or Paseo's symbol for the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// An official mark.
    Mark(EditorMark),
    /// Paseo's `folder` symbol (file managers).
    Folder,
    /// Paseo's `terminal` symbol (`SquareTerminal`, editors without a mark).
    Terminal,
}

/// One of Paseo's targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TargetId {
    /// Cursor.
    Cursor,
    /// Trae.
    Trae,
    /// Kiro.
    Kiro,
    /// VS Code.
    VsCode,
    /// VS Code Insiders.
    VsCodeInsiders,
    /// `VSCodium`.
    VsCodium,
    /// Zed.
    Zed,
    /// Antigravity.
    Antigravity,
    /// Android Studio.
    AndroidStudio,
    /// Xcode (listed only for a workspace it can open).
    Xcode,
    /// `IntelliJ` IDEA.
    IntellijIdea,
    /// Aqua.
    Aqua,
    /// `CLion`.
    Clion,
    /// `DataGrip`.
    DataGrip,
    /// `DataSpell`.
    DataSpell,
    /// `GoLand`.
    GoLand,
    /// `PhpStorm`.
    PhpStorm,
    /// `PyCharm`.
    PyCharm,
    /// Rider.
    Rider,
    /// `RubyMine`.
    RubyMine,
    /// `RustRover`.
    RustRover,
    /// `WebStorm`.
    WebStorm,
    /// Finder (macOS).
    Finder,
    /// Explorer (Windows).
    Explorer,
    /// The desktop's file manager ("Files", the rest).
    FileManager,
}

impl TargetId {
    /// Every target, in Paseo's order (`registry.ts` `EDITOR_TARGETS`).
    pub const ALL: [Self; 25] = [
        Self::Cursor,
        Self::Trae,
        Self::Kiro,
        Self::VsCode,
        Self::VsCodeInsiders,
        Self::VsCodium,
        Self::Zed,
        Self::Antigravity,
        Self::AndroidStudio,
        Self::Xcode,
        Self::IntellijIdea,
        Self::Aqua,
        Self::Clion,
        Self::DataGrip,
        Self::DataSpell,
        Self::GoLand,
        Self::PhpStorm,
        Self::PyCharm,
        Self::Rider,
        Self::RubyMine,
        Self::RustRover,
        Self::WebStorm,
        Self::Finder,
        Self::Explorer,
        Self::FileManager,
    ];

    /// Paseo's id, as the preferred editor is kept.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Cursor => "cursor",
            Self::Trae => "trae",
            Self::Kiro => "kiro",
            Self::VsCode => "vscode",
            Self::VsCodeInsiders => "vscode-insiders",
            Self::VsCodium => "vscodium",
            Self::Zed => "zed",
            Self::Antigravity => "antigravity",
            Self::AndroidStudio => "android-studio",
            Self::Xcode => "xcode",
            Self::IntellijIdea => "intellij-idea",
            Self::Aqua => "aqua",
            Self::Clion => "clion",
            Self::DataGrip => "datagrip",
            Self::DataSpell => "dataspell",
            Self::GoLand => "goland",
            Self::PhpStorm => "phpstorm",
            Self::PyCharm => "pycharm",
            Self::Rider => "rider",
            Self::RubyMine => "rubymine",
            Self::RustRover => "rustrover",
            Self::WebStorm => "webstorm",
            Self::Finder => "finder",
            Self::Explorer => "explorer",
            Self::FileManager => "file-manager",
        }
    }

    /// The target with Paseo's `id`.
    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|target| target.id() == id)
    }

    /// Its name, a proper noun in every language (Paseo's `label`).
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Cursor => "Cursor",
            Self::Trae => "Trae",
            Self::Kiro => "Kiro",
            Self::VsCode => "VS Code",
            Self::VsCodeInsiders => "VS Code Insiders",
            Self::VsCodium => "VSCodium",
            Self::Zed => "Zed",
            Self::Antigravity => "Antigravity",
            Self::AndroidStudio => "Android Studio",
            Self::Xcode => "Xcode",
            Self::IntellijIdea => "IntelliJ IDEA",
            Self::Aqua => "Aqua",
            Self::Clion => "CLion",
            Self::DataGrip => "DataGrip",
            Self::DataSpell => "DataSpell",
            Self::GoLand => "GoLand",
            Self::PhpStorm => "PhpStorm",
            Self::PyCharm => "PyCharm",
            Self::Rider => "Rider",
            Self::RubyMine => "RubyMine",
            Self::RustRover => "RustRover",
            Self::WebStorm => "WebStorm",
            Self::Finder => "Finder",
            Self::Explorer => "Explorer",
            Self::FileManager => "Files",
        }
    }

    /// An editor or a file manager.
    #[must_use]
    pub const fn kind(self) -> Kind {
        match self {
            Self::Finder | Self::Explorer | Self::FileManager => Kind::FileManager,
            Self::Cursor
            | Self::Trae
            | Self::Kiro
            | Self::VsCode
            | Self::VsCodeInsiders
            | Self::VsCodium
            | Self::Zed
            | Self::Antigravity
            | Self::AndroidStudio
            | Self::Xcode
            | Self::IntellijIdea
            | Self::Aqua
            | Self::Clion
            | Self::DataGrip
            | Self::DataSpell
            | Self::GoLand
            | Self::PhpStorm
            | Self::PyCharm
            | Self::Rider
            | Self::RubyMine
            | Self::RustRover
            | Self::WebStorm => Kind::Editor,
        }
    }

    /// What it is drawn with. Paseo has a PNG for Cursor, VS Code, Zed,
    /// Antigravity, Android Studio, Xcode, `WebStorm` and Finder; Vessel has
    /// an openly licensed or vendor vector for six of them, and Paseo's
    /// symbol for Antigravity and Finder (none was found: ADR 0123).
    #[must_use]
    pub const fn icon(self) -> Icon {
        match self {
            Self::Cursor => Icon::Mark(EditorMark::Cursor),
            Self::VsCode => Icon::Mark(EditorMark::VsCode),
            Self::Zed => Icon::Mark(EditorMark::Zed),
            Self::AndroidStudio => Icon::Mark(EditorMark::AndroidStudio),
            Self::Xcode => Icon::Mark(EditorMark::Xcode),
            Self::WebStorm => Icon::Mark(EditorMark::WebStorm),
            Self::Finder | Self::Explorer | Self::FileManager => Icon::Folder,
            Self::Trae
            | Self::Kiro
            | Self::VsCodeInsiders
            | Self::VsCodium
            | Self::Antigravity
            | Self::IntellijIdea
            | Self::Aqua
            | Self::Clion
            | Self::DataGrip
            | Self::DataSpell
            | Self::GoLand
            | Self::PhpStorm
            | Self::PyCharm
            | Self::Rider
            | Self::RubyMine
            | Self::RustRover => Icon::Terminal,
        }
    }
}

/// Paseo's `resolvePreferredEditorId`: the stored target while it is
/// listed, else the first listed.
#[must_use]
pub fn preferred(listed: &[TargetId], stored: Option<TargetId>) -> Option<TargetId> {
    stored
        .filter(|stored| listed.contains(stored))
        .or_else(|| listed.first().copied())
}
