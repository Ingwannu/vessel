//! Pure editing races and redaction, never fake transport/guard authority.
use super::*;
use crate::connection::{ConnectionEpochs, MachineKey};
use vessel_proto::{NodeId, ProjectId};

fn target(epochs: &mut ConnectionEpochs, project: ProjectId, node: NodeId) -> ConfigTarget {
    ConfigTarget::new(
        epochs
            .admit(MachineKey::from_book("fixture"), node)
            .unwrap(),
        project,
    )
}
fn document(text: &str, marker: char, validity: ConfigValidity) -> ConfigDocument {
    ConfigDocument {
        text: ConfigText::parse(text).unwrap(),
        revision: ConfigRevision::Present {
            hash: marker.to_string().repeat(64).parse().unwrap(),
        },
        validity,
    }
}
fn loaded() -> (ProjectConfig, ConfigTarget) {
    let target = target(
        &mut ConnectionEpochs::default(),
        ProjectId::random(),
        NodeId::random(),
    );
    let mut model = ProjectConfig::default();
    let key = model
        .begin_read(target.clone(), ReloadDraft::Preserve)
        .unwrap();
    assert!(model.read(&key, Ok(document("# original", 'a', ConfigValidity::Valid))));
    (model, target)
}

#[test]
fn config_save_preserves_edits_made_while_the_original_save_is_pending() {
    let (mut model, target) = loaded();
    model.edit("# first edit").unwrap();
    let key = model.begin_save().unwrap();
    assert_eq!(key.request().project, target.project());
    assert_eq!(key.request().expected, model.document().unwrap().revision);
    assert_eq!(model.begin_save(), Err(ConfigRefusal::Busy));
    model.edit("# second edit").unwrap();
    assert!(model.saved(
        &key,
        Ok(document("# first edit", 'b', ConfigValidity::Valid))
    ));
    assert_eq!(model.draft().unwrap().as_str(), "# second edit");
    assert!(model.dirty());
    let next = model.begin_save().unwrap();
    assert_eq!(
        next.expected(),
        &document("# first edit", 'b', ConfigValidity::Valid).revision
    );
    assert_ne!(next, key);
}

#[test]
fn config_conflict_and_uncertain_outcomes_require_explicit_reload_without_losing_the_draft() {
    for problem in [
        ConfigProblem::Conflict,
        ConfigProblem::Uncertain,
        ConfigProblem::Offline,
    ] {
        let (mut model, target) = loaded();
        model.edit("# keep my edit").unwrap();
        let key = model.begin_save().unwrap();
        assert!(model.saved(&key, Err(problem)));
        assert_eq!(model.draft().unwrap().as_str(), "# keep my edit");
        assert!(model.reload_required());
        assert_eq!(model.begin_save(), Err(ConfigRefusal::ReloadRequired));
        assert_eq!(
            model.begin_read(target.clone(), ReloadDraft::Preserve),
            Err(ConfigRefusal::Dirty)
        );
        let reload = model.begin_read(target, ReloadDraft::Discard).unwrap();
        assert!(!model.saved(
            &key,
            Ok(document("# keep my edit", 'c', ConfigValidity::Valid))
        ));
        assert!(model.read(
            &reload,
            Ok(document("# external edit", 'd', ConfigValidity::Valid))
        ));
        assert!(!model.dirty());
        assert!(!model.reload_required());
    }
}

#[test]
fn config_reopen_same_target_and_same_node_reconnect_ignore_old_tickets() {
    let project = ProjectId::random();
    let node = NodeId::random();
    let mut epochs = ConnectionEpochs::default();
    let first = target(&mut epochs, project, node);
    let mut model = ProjectConfig::default();
    let old = model
        .begin_read(first.clone(), ReloadDraft::Preserve)
        .unwrap();
    model.detach();
    let fresh = model.begin_read(first, ReloadDraft::Preserve).unwrap();
    assert_ne!(old, fresh);
    assert!(!model.read(&old, Ok(document("# stale", 'a', ConfigValidity::Valid))));
    assert!(model.pending());
    model.detach();
    let replacement = target(&mut epochs, project, node);
    let current = model
        .begin_read(replacement.clone(), ReloadDraft::Preserve)
        .unwrap();
    assert!(!model.read(
        &fresh,
        Ok(document("# old peer", 'a', ConfigValidity::Valid))
    ));
    assert_eq!(current.target(), &replacement);
    assert!(model.read(
        &current,
        Ok(document("# current", 'b', ConfigValidity::Valid))
    ));
}

#[test]
fn config_invalid_source_is_editable_but_incoherent_save_receipts_do_not_become_success() {
    let (mut model, target) = loaded();
    let reload = model.begin_read(target, ReloadDraft::Preserve).unwrap();
    assert!(model.read(
        &reload,
        Ok(document("broken = [", 'a', ConfigValidity::Invalid))
    ));
    model.edit("# repaired").unwrap();
    let save = model.begin_save().unwrap();
    assert!(model.saved(
        &save,
        Ok(document("# another document", 'b', ConfigValidity::Valid))
    ));
    assert_eq!(model.problem(), Some(ConfigProblem::Incoherent));
    assert!(model.reload_required());
    assert_eq!(model.draft().unwrap().as_str(), "# repaired");
}

#[test]
fn config_missing_file_and_secret_text_keep_canonical_bounds_and_redacted_diagnostics() {
    let (mut model, target) = loaded();
    let key = model.begin_read(target, ReloadDraft::Preserve).unwrap();
    assert!(model.read(
        &key,
        Ok(ConfigDocument {
            text: ConfigText::parse("").unwrap(),
            revision: ConfigRevision::Missing,
            validity: ConfigValidity::Valid
        })
    ));
    let secret = "credential = 'canary-private-editor-source'";
    model.edit(secret).unwrap();
    let save = model.begin_save().unwrap();
    assert_eq!(save.expected(), &ConfigRevision::Missing);
    assert!(!format!("{model:?} {save:?}").contains(secret));
    assert_eq!(
        model.edit("bad\0source"),
        Err(ConfigRefusal::Text(ConfigTextError::Nul))
    );
    assert_eq!(model.draft().unwrap().as_str(), secret);
    model.detach();
    assert_eq!(model.edit("# not loaded"), Err(ConfigRefusal::NotHydrated));
    model.serial = u64::MAX;
    assert_eq!(
        model.begin_read(save.target().clone(), ReloadDraft::Discard),
        Err(ConfigRefusal::Exhausted)
    );
}

#[test]
fn config_phase_keeps_loading_saving_conflict_and_failed_reads_distinct_from_absence() {
    let (mut model, target) = loaded();
    assert_eq!(model.phase(), ConfigPhase::Ready);
    model.edit("# save").unwrap();
    let key = model.begin_save().unwrap();
    assert_eq!(model.phase(), ConfigPhase::Saving);
    assert!(model.saved(&key, Err(ConfigProblem::Conflict)));
    assert_eq!(model.phase(), ConfigPhase::ReloadRequired);
    let read = model
        .begin_read(target.clone(), ReloadDraft::Discard)
        .unwrap();
    assert_eq!(model.phase(), ConfigPhase::Loading);
    assert!(model.read(&read, Err(ConfigProblem::Unsupported)));
    assert_eq!(model.phase(), ConfigPhase::Failed);
    assert!(model.document().is_none());
    model.detach();
    assert_eq!(model.phase(), ConfigPhase::NotHydrated);
    let read = model.begin_read(target, ReloadDraft::Preserve).unwrap();
    assert!(model.read(
        &read,
        Ok(ConfigDocument {
            text: ConfigText::parse("").unwrap(),
            revision: ConfigRevision::Missing,
            validity: ConfigValidity::Valid,
        })
    ));
    assert_eq!(model.phase(), ConfigPhase::Ready);
}
