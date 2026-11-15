//! Settings' sections and where Settings is: the list the sidebar shows
//! while Settings is open, in Paseo's two groups and Paseo's order
//! (`screens/settings-screen.tsx`, `SIDEBAR_SECTION_ITEMS` and
//! `HOST_SECTION_ITEMS`).
//!
//! [Decision Log]
//! - Purpose: the owner (2026-11-14, words 43 and 45): this version is
//!   Paseo's features and settings carried over as they are, so Settings
//!   lists Paseo's sections, under Paseo's two headings, in Paseo's order,
//!   on every surface (ADR 0036). The order and the groups are one table,
//!   here.
//! - Before: the page's eleven sections in four groups (General,
//!   Appearance, Shortcuts, Notifications | Agents, Models | Machines,
//!   Projects, Devices, Sync | About).
//! - Alternatives: keep the page's groups and add Paseo's sections to
//!   them; or Paseo's table with the page's sections mapped onto it.
//! - Choice: Paseo's table. The page's sections land where Paseo has the
//!   same thing: Machines is Paseo's host Overview, Devices is "Pair
//!   device" (it also lists and revokes the paired devices, which Paseo's
//!   page does not), the agents list with install and sign-in is Paseo's
//!   Providers. Two sections are Vessel's own and sit after Paseo's in the
//!   machine group: Models (model routes; Paseo shows models inside each
//!   provider) and Sync (the fleet's shared configuration; Paseo has no
//!   fleet).
//! - Which surface lists a section is Paseo's `desktopOnly` / `webOnly`
//!   ([`Section::available_on`]); which screen draws it, and so whether it
//!   is listed at all in a build, is the app's registry.
//! - Cost: a section's slug is Paseo's (`pair-device`, `host`), so the
//!   page's `settings/devices` and `settings/machines` addresses name
//!   nothing here until the page follows (ADR 0036: the page is brought
//!   to the app's layout afterwards).

/// One section of Settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Section {
    /// Language, the send key, what Send does while a turn runs.
    General,
    /// Theme, type sizes, the conversation's width.
    Appearance,
    /// What the sidebar shows.
    Sidebar,
    /// How a conversation is shown.
    Chat,
    /// The terminal's scrollback.
    Terminal,
    /// The in-app browser's data (Paseo's desktop app only).
    Browser,
    /// The code editor's keys (Paseo's web and desktop only).
    Editor,
    /// Which panel opens a kind of file.
    Files,
    /// Keyboard shortcuts (desktop only).
    Shortcuts,
    /// The command-line tool (desktop only).
    Integrations,
    /// How the system announces a session (desktop only).
    Notifications,
    /// The system's permissions the app asks for (desktop only).
    Permissions,
    /// A diagnostic report of the app and its machines.
    Diagnostics,
    /// Version, updates, the machines' versions.
    About,
    /// The chosen machine: its name and what can be done to it (Paseo's
    /// host "Overview").
    Overview,
    /// The machine's projects.
    Projects,
    /// How the app reaches the machine.
    Connections,
    /// Pairing a device with the machine, and its paired devices.
    Devices,
    /// How the machine runs its agents (tools, prompt, restart, release).
    Agents,
    /// Generated titles and branch names.
    Metadata,
    /// Dictation and voice mode.
    Voice,
    /// What happens to a workspace (archiving after a merge).
    Workspaces,
    /// The agents the machine can run: installed, signed in, their models.
    Providers,
    /// The agents' accounts' usage and limits.
    Usage,
    /// Terminal profiles and terminal agents.
    Terminals,
    /// The machine's plugins.
    Plugins,
    /// Model routes (Vessel's own).
    Models,
    /// The fleet's shared configuration (Vessel's own).
    Sync,
}

/// Where the app runs, for the sections Paseo lists only on some
/// surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    /// The desktop app (Paseo's Electron app).
    Desktop,
    /// A browser tab (Paseo's web app).
    Browser,
    /// The phone app (Paseo's native app).
    Phone,
}

/// Which surfaces list a section (Paseo's `desktopOnly`, `webOnly`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Availability {
    /// Every surface.
    Everywhere,
    /// The desktop app only (`desktopOnly`).
    Desktop,
    /// The desktop app and a browser (`webOnly`: Paseo's desktop app is a
    /// web page too).
    Web,
}

impl Section {
    /// Every section, in the order Settings lists them.
    pub const ALL: [Self; 28] = [
        Self::General,
        Self::Appearance,
        Self::Sidebar,
        Self::Chat,
        Self::Terminal,
        Self::Browser,
        Self::Editor,
        Self::Files,
        Self::Shortcuts,
        Self::Integrations,
        Self::Notifications,
        Self::Permissions,
        Self::Diagnostics,
        Self::About,
        Self::Overview,
        Self::Projects,
        Self::Connections,
        Self::Devices,
        Self::Agents,
        Self::Metadata,
        Self::Voice,
        Self::Workspaces,
        Self::Providers,
        Self::Usage,
        Self::Terminals,
        Self::Plugins,
        Self::Models,
        Self::Sync,
    ];

    /// The section Settings opens on.
    pub const DEFAULT: Self = Self::General;

    /// Its name in an address (`settings/<slug>`): Paseo's section ids.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Appearance => "appearance",
            Self::Sidebar => "sidebar",
            Self::Chat => "chat",
            Self::Terminal => "terminal",
            Self::Browser => "browser",
            Self::Editor => "editor",
            Self::Files => "files",
            Self::Shortcuts => "shortcuts",
            Self::Integrations => "integrations",
            Self::Notifications => "notifications",
            Self::Permissions => "permissions",
            Self::Diagnostics => "diagnostics",
            Self::About => "about",
            Self::Overview => "host",
            Self::Projects => "projects",
            Self::Connections => "connections",
            Self::Devices => "pair-device",
            Self::Agents => "agents",
            Self::Metadata => "metadata",
            Self::Voice => "voice",
            Self::Workspaces => "workspaces",
            Self::Providers => "providers",
            Self::Usage => "usage",
            Self::Terminals => "terminals",
            Self::Plugins => "plugins",
            Self::Models => "models",
            Self::Sync => "sync",
        }
    }

    /// The section an address names, exactly (nor does a section not
    /// shown).
    #[must_use]
    pub fn of(slug: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .filter(|section| section.shown())
            .find(|section| section.slug() == slug)
    }

    /// Whether the section is listed and can be opened. Sync is not, for
    /// now (the coordinator, 2026-11-14): on a real node its page said a
    /// paired admin device lacked permission, because it asked for the
    /// member protocol's reads; until it says the truth on a standalone,
    /// home, member and protocol 11 machine, nothing shown may mislead.
    #[must_use]
    pub const fn shown(self) -> bool {
        !matches!(self, Self::Sync)
    }

    /// Whether `surface` lists it (Paseo's `isSectionAvailable`).
    #[must_use]
    pub const fn available_on(self, surface: Surface) -> bool {
        match self.availability() {
            Availability::Everywhere => true,
            Availability::Desktop => matches!(surface, Surface::Desktop),
            Availability::Web => matches!(surface, Surface::Desktop | Surface::Browser),
        }
    }

    /// Paseo's `desktopOnly` and `webOnly` flags.
    const fn availability(self) -> Availability {
        match self {
            Self::Browser
            | Self::Shortcuts
            | Self::Integrations
            | Self::Notifications
            | Self::Permissions => Availability::Desktop,
            Self::Editor => Availability::Web,
            Self::General
            | Self::Appearance
            | Self::Sidebar
            | Self::Chat
            | Self::Terminal
            | Self::Files
            | Self::Diagnostics
            | Self::About
            | Self::Overview
            | Self::Projects
            | Self::Connections
            | Self::Devices
            | Self::Agents
            | Self::Metadata
            | Self::Voice
            | Self::Workspaces
            | Self::Providers
            | Self::Usage
            | Self::Terminals
            | Self::Plugins
            | Self::Models
            | Self::Sync => Availability::Everywhere,
        }
    }

    /// The group it is listed under.
    #[must_use]
    pub const fn group(self) -> Group {
        match self {
            Self::General
            | Self::Appearance
            | Self::Sidebar
            | Self::Chat
            | Self::Terminal
            | Self::Browser
            | Self::Editor
            | Self::Files
            | Self::Shortcuts
            | Self::Integrations
            | Self::Notifications
            | Self::Permissions
            | Self::Diagnostics
            | Self::About => Group::App,
            Self::Overview
            | Self::Projects
            | Self::Connections
            | Self::Devices
            | Self::Agents
            | Self::Metadata
            | Self::Voice
            | Self::Workspaces
            | Self::Providers
            | Self::Usage
            | Self::Terminals
            | Self::Plugins
            | Self::Models
            | Self::Sync => Group::Machine,
        }
    }

    /// Whether it is about one machine, chosen by the machine picker that
    /// heads its group.
    #[must_use]
    pub const fn per_machine(self) -> bool {
        matches!(self.group(), Group::Machine)
    }
}

/// A heading of the sections' list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Group {
    /// The app itself, on this device (Paseo's "App").
    App,
    /// The machine the picker chose (Paseo's "Host").
    Machine,
}

impl Group {
    /// Every group, in order.
    pub const ALL: [Self; 2] = [Self::App, Self::Machine];

    /// Its sections `surface` lists, in order.
    #[must_use]
    pub fn sections(self, surface: Surface) -> Vec<Section> {
        Section::ALL
            .into_iter()
            .filter(|section| {
                section.group() == self && section.shown() && section.available_on(surface)
            })
            .collect()
    }
}

/// Where in Settings an address points (the page's `SettingsPlace`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    /// The section shown: the one named, else the default.
    pub section: Section,
    /// Whether the address named exactly that section (an unknown or
    /// longer one is shown as the default and replaced).
    pub exact: bool,
    /// The machine asked for, by its name in the book.
    pub machine: Option<String>,
    /// The agent asked for (an agent's key).
    pub agent: Option<String>,
}

impl Place {
    /// The place an address's segments point to; `None` outside Settings.
    #[must_use]
    pub fn of(segments: &[&str], machine: Option<&str>, agent: Option<&str>) -> Option<Self> {
        let (first, rest) = segments.split_first()?;
        if *first != "settings" {
            return None;
        }
        let named = rest.first().and_then(|slug| Section::of(slug));
        Some(Self {
            section: named.unwrap_or(Section::DEFAULT),
            exact: named.is_some() && rest.len() == 1,
            machine: machine.map(str::to_owned),
            agent: agent.map(str::to_owned),
        })
    }
}

mod words;

mod general;
pub use general::StorageError;

mod refusal;
pub use refusal::RefusalWordsError;
