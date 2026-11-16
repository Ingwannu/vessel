//! Picker drafts are seeded on open, never by a background status refresh.
use vessel_proto::skills::{SkillName, SkillNames, SkillSelection};

/// A selection draft and the catalogue captured when it opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picker {
    /// Exact user draft.
    pub draft: SkillSelection,
    /// Opening catalogue; a changed bundle is validated by the node at save.
    pub available: SkillNames,
    pub(super) confirmed_once: bool,
}
impl Picker {
    /// Open from a confirmed snapshot.
    #[must_use]
    pub fn open(selection: SkillSelection, available: SkillNames) -> Self {
        Self {
            draft: selection,
            available,
            confirmed_once: false,
        }
    }
    /// All includes future skills; leaving it starts with every current name.
    pub fn toggle_all(&mut self) {
        self.draft = match self.draft {
            SkillSelection::All => SkillSelection::Custom {
                skills: self.available.clone(),
            },
            SkillSelection::Custom { .. } => SkillSelection::All,
        };
        self.confirmed_once = false;
    }
    /// Toggle an available name only in custom mode.
    pub fn toggle(&mut self, name: &SkillName) {
        if !self.available.contains(name) {
            return;
        }
        let SkillSelection::Custom { skills } = &mut self.draft else {
            return;
        };
        let mut next = skills.as_slice().to_vec();
        if skills.contains(name) {
            next.retain(|entry| entry != name);
        } else {
            next.push(name.clone());
        }
        // This is a subset of the already bounded opening catalogue.
        if let Ok(chosen) = SkillNames::new(next) {
            *skills = chosen;
        }
        self.confirmed_once = false;
    }
    /// A row's value, without changing its draft.
    #[must_use]
    pub fn selected(&self, name: &SkillName) -> bool {
        match &self.draft {
            SkillSelection::All => true,
            SkillSelection::Custom { skills } => skills.contains(name),
        }
    }
}
