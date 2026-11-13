//! Every word the app shows, in English and Korean, chosen by one switch
//! ([`Lang`]: the system's language, or `VESSEL_APP_LANG`).
//!
//! [Decision Log]
//! - Purpose: the owner and most of the app's users read Korean (the
//!   coordinator's report on build 0.0.0.0.1): every string goes through
//!   tables that switch together, from the first screens on.
//! - Constraint: four tracks write strings at once; one table edited by
//!   all of them would conflict on every merge (the coordinator's ruling).
//! - Alternatives: `rust-i18n` (in the tree through the component layer:
//!   keys are strings, a missing translation is found at run time); Fluent
//!   (a new dependency and a runtime format).
//! - Choice: one module per area under `text/` (`app.rs` is the lead's:
//!   the sidebar, the board, the new chat, pairing, the frame). Each holds
//!   a struct with one field per sentence, an `EN` and a `KO` constant of
//!   it (a sentence missing from one language does not compile), and an
//!   `impl Lang` with the area's accessor (`lang.conversation()`). Adding
//!   an area's words is its own file and one `mod` line here. Sentences
//!   with values in them are methods matching on the language, so the
//!   word order is the language's.
//! - Cost: plural rules are written by hand (English needs few here,
//!   Korean none). Not translated yet: the link's errors (they come from
//!   `vessel-client` and the node, in English).

mod about;
mod agents;
mod app;
mod board;
mod conversation;
mod devices;
mod files;
pub mod localization;
mod machine_book;
mod machines;
mod notices;
mod options;
mod pairing;
mod palette;
mod settings;
mod shortcuts;
mod sidebar;
mod update;
pub mod workspace;

pub use about::AboutText;
pub use app::Text;
pub use board::BoardWords;
pub use conversation::{ConversationWords, grouped, meter_tokens, short_tokens};
pub use machines::MachinesText;
pub use notices::NoticeWords;
pub use options::OptionsText;
pub use pairing::PairingWords;
pub use palette::PaletteWords;
pub use settings::SettingsText;
pub use shortcuts::ShortcutWords;
pub use sidebar::SidebarWords;
pub use update::UpdateText;

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
