//! Map the closed inventory roles to prepared catalog labels. A Fluent
//! selector would format on every row draw; plain getters borrow the cache.

use super::Role;
use crate::localization::About;

impl<'a> About<'a> {
    /// Borrow what a bundled part does here without formatting during a frame.
    #[must_use]
    pub fn role(self, role: Role) -> &'a str {
        match role {
            Role::Drawing => self.role_drawing(),
            Role::Windows => self.role_windows(),
            Role::Editing => self.role_editing(),
            Role::Controls => self.role_controls(),
            Role::Bytes => self.role_bytes(),
            Role::Dates => self.role_dates(),
            Role::Folders => self.role_folders(),
            Role::ComputerName => self.role_computer_name(),
            Role::Data => self.role_data(),
            Role::Translations => self.role_translations(),
            Role::Identifiers => self.role_identifiers(),
            Role::Language => self.role_language(),
            Role::Desktop => self.role_desktop(),
            Role::Tasks => self.role_tasks(),
            Role::Images => self.role_images(),
            Role::Errors => self.role_errors(),
            Role::Network => self.role_network(),
            Role::Logs => self.role_logs(),
            Role::QrCodes => self.role_qr_codes(),
            Role::Checksums => self.role_checksums(),
            Role::Terminal => self.role_terminal(),
            Role::Accents => self.role_accents(),
            Role::Face => self.role_face(),
            Role::CodeFace => self.role_code_face(),
            Role::Hangul => self.role_hangul(),
        }
    }
}

#[cfg(test)]
#[path = "words_tests.rs"]
mod tests;
