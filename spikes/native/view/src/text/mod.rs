//! Legacy area tables beside the embedded Fluent catalogs (ADR 0039).
//!
//! [Decision Log]
//! - Purpose: move whole areas into translator-owned files while active area
//!   owners can keep editing their existing tables without conflicts.
//! - Constraint: untranslated areas still switch English/Korean together;
//!   neither a new language enum arm nor runtime keys may become necessary.
//! - Choice: `localization` generates typed accessors from English Fluent;
//!   pairing uses that API completely. `Lang` remains the temporary table
//!   adapter for areas their owners have not moved yet.
//! - Why: one area per migration keeps code review and concurrent ownership
//!   clear. A screen's shared words stay with their original area until it moves.
//! - Cost: additional languages cover migrated areas only during this staged
//!   move. The app ships complete English and Korean catalogs today.

mod agents;
mod app;
mod board;
mod conversation;
mod devices;
mod files;
mod git;
pub mod localization;
mod machine_book;
mod machines;
mod marks;
mod motion;
mod notices;
mod options;
mod overview;
mod palette;
mod settings;
mod shortcuts;
mod sidebar;
mod sync;
pub mod workspace;

pub use app::Text;
pub use board::BoardWords;
pub use conversation::{ConversationWords, grouped, meter_tokens, short_tokens};
pub use git::{
    GitFailureText, GitHashText, GitHeaderText, GitLabels, GitStashText, GitSuccessCommits,
    GitSuccessText,
};
pub use machines::MachinesText;
pub use marks::MarkText;
pub use notices::NoticeWords;
pub use options::OptionsText;
pub use overview::OverviewText;
pub use palette::PaletteWords;
pub use settings::SettingsText;
pub use shortcuts::ShortcutWords;
pub use sidebar::SidebarWords;

/// A language the app speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    /// English.
    En,
    /// Korean.
    Ko,
}

impl Lang {
    /// The language for a BCP 47 tag or a POSIX locale (`ko-KR`,
    /// `ko_KR.UTF-8`): Korean for any `ko`, English otherwise.
    #[must_use]
    pub fn from_tag(tag: &str) -> Self {
        let tag = tag.to_ascii_lowercase();
        let korean = tag == "ko"
            || ["ko-", "ko_", "ko."]
                .iter()
                .any(|prefix| tag.starts_with(prefix));
        if korean { Self::Ko } else { Self::En }
    }
}
