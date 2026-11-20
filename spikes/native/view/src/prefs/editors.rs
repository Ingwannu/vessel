//! The editor chosen per project for Open (TRACKER 467, Paseo's
//! `hooks/use-preferred-editor.ts`, pinned 0.15.4).
//!
//! [Decision Log]
//! - Purpose: the header's Open uses the editor last chosen for the
//!   project, as Paseo keeps one per (server, project) on the device.
//! - Choice: the native document keeps Paseo's target id per project
//!   ([`ProjectKey`]: the machine's name and the project's id), written
//!   only once one is chosen (as the folds), and kept tidy with the other
//!   per-project entries ([`ProjectKeyed`]). An id this build does not know
//!   is kept and read as "none listed", so Open falls back to the first
//!   target, as Paseo's `resolvePreferredEditorId` does.
//! - Difference: Paseo also reads an older device-wide choice as a
//!   fallback; Vessel never had one.
//! - Cost: at most [`MAX_PREFERRED_EDITORS`] projects; the oldest choice
//!   goes first.

use serde::{Deserialize, Serialize};
use vessel_proto::ProjectId;

use super::projects::{ProjectKey, ProjectKeyed};
use crate::open_targets::TargetId;

/// The most projects with a chosen editor kept.
pub const MAX_PREFERRED_EDITORS: usize = 256;

/// One project's choice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreferredEditor {
    /// The project.
    #[serde(flatten)]
    pub key: ProjectKey,
    /// Paseo's target id.
    pub editor: String,
}

/// The chosen editors, oldest choice first.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PreferredEditors(Vec<PreferredEditor>);

impl PreferredEditors {
    /// Whether none is chosen (the document leaves the field out).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The target chosen for `project` on `machine`, when this build knows
    /// it.
    #[must_use]
    pub fn get(&self, machine: &str, project: ProjectId) -> Option<TargetId> {
        self.0
            .iter()
            .find(|chosen| chosen.key.is(machine, project))
            .and_then(|chosen| TargetId::from_id(&chosen.editor))
    }

    /// Choose `target` for `project` on `machine`; whether anything changed.
    pub fn set(&mut self, machine: &str, project: ProjectId, target: TargetId) -> bool {
        if self.get(machine, project) == Some(target) {
            return false;
        }
        self.0.retain(|chosen| !chosen.key.is(machine, project));
        self.0.push(PreferredEditor {
            key: ProjectKey::new(machine, project),
            editor: target.id().to_owned(),
        });
        let over = self.0.len().saturating_sub(MAX_PREFERRED_EDITORS);
        drop(self.0.drain(..over));
        true
    }
}

impl ProjectKeyed for PreferredEditors {
    fn keep_listed(&mut self, machine: &str, listed: &[ProjectId]) -> bool {
        let before = self.0.len();
        self.0
            .retain(|chosen| chosen.key.machine != machine || listed.contains(&chosen.key.project));
        self.0.len() != before
    }

    fn rename_machine(&mut self, from: &str, to: &str) -> bool {
        let mut any = false;
        for chosen in self.0.iter_mut().filter(|chosen| chosen.key.machine == from) {
            to.clone_into(&mut chosen.key.machine);
            any = true;
        }
        any
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_choice_is_kept_per_project_and_machine_and_tidied_with_the_book() {
        let ids = [ProjectId::random(), ProjectId::random()];
        let project = |n: usize| ids[n];
        let mut editors = PreferredEditors::default();
        assert!(editors.set("laptop", project(1), TargetId::Zed));
        assert!(!editors.set("laptop", project(1), TargetId::Zed));
        assert!(editors.set("laptop", project(1), TargetId::VsCode));
        assert!(editors.set("desk", project(1), TargetId::Cursor));
        assert_eq!(editors.get("laptop", project(1)), Some(TargetId::VsCode));
        assert_eq!(editors.get("desk", project(1)), Some(TargetId::Cursor));
        assert_eq!(editors.get("laptop", project(2)), None);
        assert!(editors.rename_machine("laptop", "work"));
        assert_eq!(editors.get("work", project(1)), Some(TargetId::VsCode));
        assert!(editors.keep_listed("work", &[]));
        assert_eq!(editors.get("work", project(1)), None);
        assert!(editors.forget_machine("desk"));
        assert!(editors.is_empty());
    }

    #[test]
    fn the_document_holds_paseo_s_ids_and_an_unknown_one_reads_as_none() {
        let seven = ProjectId::random();
        let project = |_n: usize| seven;
        let mut editors = PreferredEditors::default();
        let _changed = editors.set("laptop", project(7), TargetId::AndroidStudio);
        let json = serde_json::to_value(&editors).unwrap();
        assert_eq!(json[0]["editor"], "android-studio");
        assert_eq!(json[0]["machine"], "laptop");
        let mut newer = json.clone();
        newer[0]["editor"] = "fleet".into();
        let read: PreferredEditors = serde_json::from_value(newer).unwrap();
        assert_eq!(read.get("laptop", project(7)), None);
    }

    #[test]
    fn past_the_cap_the_oldest_choice_goes() {
        let ids: Vec<ProjectId> = (0..=MAX_PREFERRED_EDITORS)
            .map(|_n| ProjectId::random())
            .collect();
        let project = |n: usize| ids[n];
        let mut editors = PreferredEditors::default();
        for n in 0..=MAX_PREFERRED_EDITORS {
            let _changed = editors.set("laptop", project(n), TargetId::Zed);
        }
        assert_eq!(editors.get("laptop", project(0)), None);
        assert_eq!(editors.get("laptop", project(1)), Some(TargetId::Zed));
    }
}
