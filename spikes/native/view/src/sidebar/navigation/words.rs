//! Closed builtin vocabulary borrowed from the prepared Fluent catalog.
use super::{Builtin, Section};
use crate::localization::Sidebar;

impl<'a> Sidebar<'a> {
    /// One label for both the navigation row and its settings row.
    #[must_use]
    pub fn builtin(self, item: Builtin) -> &'a str {
        match item {
            Builtin::NewWorkspace => self.new_workspace(),
            Builtin::History => self.history(),
            Builtin::Search => self.search(),
            Builtin::Schedules => self.schedules(),
            Builtin::Usage => self.usage(),
        }
    }
    /// The settings card heading for this independent band.
    #[must_use]
    pub fn band_title(self, section: Section) -> &'a str {
        match section {
            Section::Header => self.header_title(),
            Section::Footer => self.footer_title(),
        }
    }
    /// Explanation shown by the settings card title info tip.
    #[must_use]
    pub fn band_description(self, section: Section) -> &'a str {
        match section {
            Section::Header => self.header_description(),
            Section::Footer => self.footer_description(),
        }
    }
}
