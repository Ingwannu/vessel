//! Settings' sections and where Settings is (the page's
//! `data/settings.ts` and `settings-place.ts`): the list the sidebar
//! shows while Settings is open, grouped as the page groups them.
//!
//! [Decision Log]
//! - Purpose: every surface lists the same sections in the same order
//!   (ADR 0036), so the order and the groups are one table, here.
//! - Choice: the page's sections, groups and default; a section is named
//!   in an address by its [`Section::slug`]. Which screen draws a section
//!   is the app's registry (another track's section is one line there).

/// One section of Settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Section {
    /// The send key, the default send, where preferences are kept.
    General,
    /// Theme, sizes, the conversation's width and display.
    Appearance,
    /// Keyboard shortcuts (the shortcut registry's track).
    Shortcuts,
    /// How sessions are announced (the notifications' track).
    Notifications,
    /// Each machine's agents.
    Agents,
    /// Model routes and profiles.
    Models,
    /// The machines in the book.
    Machines,
    /// Each machine's projects (the workspace's track).
    Projects,
    /// A machine's paired devices.
    Devices,
    /// The fleet's shared configuration.
    Sync,
    /// Versions, licences, updates.
    About,
}

impl Section {
    /// Every section, in the order the sidebar lists them.
    pub const ALL: [Self; 11] = [
        Self::General,
        Self::Appearance,
        Self::Shortcuts,
        Self::Notifications,
        Self::Agents,
        Self::Models,
        Self::Machines,
        Self::Projects,
        Self::Devices,
        Self::Sync,
        Self::About,
    ];

    /// The section Settings opens on.
    pub const DEFAULT: Self = Self::General;

    /// Its name in an address (`settings/<slug>`).
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Appearance => "appearance",
            Self::Shortcuts => "shortcuts",
            Self::Notifications => "notifications",
            Self::Agents => "agents",
            Self::Models => "models",
            Self::Machines => "machines",
            Self::Projects => "projects",
            Self::Devices => "devices",
            Self::Sync => "sync",
            Self::About => "about",
        }
    }

    /// The section an address names, exactly (`Machines` and `fleet` name
    /// none; nor does a section not shown).
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

    /// The group it is listed under.
    #[must_use]
    pub const fn group(self) -> Group {
        match self {
            Self::General | Self::Appearance | Self::Shortcuts | Self::Notifications => Group::App,
            Self::Agents | Self::Models => Group::Agents,
            Self::Machines | Self::Projects | Self::Devices | Self::Sync => Group::Machines,
            Self::About => Group::About,
        }
    }

    /// Whether it is about one machine, chosen by the machine picker (the
    /// Machines section shows the chosen one's detail).
    #[must_use]
    pub const fn per_machine(self) -> bool {
        match self {
            Self::Agents
            | Self::Models
            | Self::Machines
            | Self::Projects
            | Self::Devices
            | Self::Sync => true,
            Self::General
            | Self::Appearance
            | Self::Shortcuts
            | Self::Notifications
            | Self::About => false,
        }
    }
}

/// A heading of the sections' list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Group {
    /// The app itself.
    App,
    /// Agents and their models.
    Agents,
    /// Machines and what they share.
    Machines,
    /// About Vessel.
    About,
}

impl Group {
    /// Every group, in order.
    pub const ALL: [Self; 4] = [Self::App, Self::Agents, Self::Machines, Self::About];

    /// Its sections, in order.
    #[must_use]
    pub fn sections(self) -> Vec<Section> {
        Section::ALL
            .into_iter()
            .filter(|section| section.group() == self && section.shown())
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
