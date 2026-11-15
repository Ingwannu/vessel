//! Validate complete catalog/assignment mutations before changing SQL rows.
//! Workspace lifecycle validation stays inside the writer transaction.

use vessel_proto::workspace_labels::{
    Assignment, CatalogDelta, Label, LabelName, UpdateWorkspaceLabel, WorkspaceLabelFailure,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum AssignmentChange {
    Unchanged,
    Changed,
}

pub(super) struct AssignmentPlan {
    pub(super) definition: Label,
    pub(super) names: Vec<LabelName>,
    pub(super) assignment: AssignmentChange,
    pub(super) catalog: Option<CatalogDelta>,
}

pub(super) fn assign(
    catalog: &[Label],
    current: &[LabelName],
    requested: &Label,
    assignment: Assignment,
) -> AssignmentPlan {
    let key = requested.name.key();
    let existing = catalog.iter().find(|label| label.name.key() == key);
    let definition = existing.unwrap_or(requested).clone();
    let present = current.iter().position(|name| name.key() == key);
    let catalog = if existing.is_none() && assignment == Assignment::Assigned {
        Some(CatalogDelta::Upsert {
            label: definition.clone(),
            previous: None,
        })
    } else {
        None
    };
    let (names, assignment) = match (assignment, present) {
        (Assignment::Assigned, None) => {
            let mut names = current.to_vec();
            names.push(definition.name.clone());
            (names, AssignmentChange::Changed)
        }
        (Assignment::Unassigned, Some(remove)) => (
            current
                .iter()
                .enumerate()
                .filter(|(index, _)| *index != remove)
                .map(|(_, name)| name.clone())
                .collect(),
            AssignmentChange::Changed,
        ),
        (Assignment::Assigned, Some(_)) | (Assignment::Unassigned, None) => {
            (current.to_vec(), AssignmentChange::Unchanged)
        }
    };
    AssignmentPlan {
        definition,
        names,
        assignment,
        catalog,
    }
}

pub(super) struct UpdatePlan {
    pub(super) definition: Label,
    pub(super) catalog: Option<CatalogDelta>,
}

pub(super) fn update(
    catalog: &[Label],
    request: &UpdateWorkspaceLabel,
) -> Result<UpdatePlan, WorkspaceLabelFailure> {
    let from = request.name.key();
    let existing = catalog
        .iter()
        .find(|label| label.name.key() == from)
        .ok_or(WorkspaceLabelFailure::LabelNotFound)?;
    let name = request.new_name.as_ref().unwrap_or(&existing.name);
    let to = name.key();
    if to != from && catalog.iter().any(|label| label.name.key() == to) {
        return Err(WorkspaceLabelFailure::NameTaken);
    }
    let definition = Label {
        name: name.clone(),
        color: request.color.unwrap_or(existing.color),
    };
    let catalog = (definition != *existing).then(|| CatalogDelta::Upsert {
        previous: (definition.name != existing.name).then(|| existing.name.clone()),
        label: definition.clone(),
    });
    Ok(UpdatePlan {
        definition,
        catalog,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::workspace_labels::LabelColor;

    fn label(name: &str, color: LabelColor) -> Label {
        Label {
            name: name.parse().unwrap(),
            color,
        }
    }

    #[test]
    fn assignment_reuses_host_spelling_color_and_preserves_existing_order() {
        let catalog = vec![label("Ready", LabelColor::Blue)];
        let current = vec!["Other".parse().unwrap()];
        let plan = assign(
            &catalog,
            &current,
            &label(" ready ", LabelColor::Red),
            Assignment::Assigned,
        );
        assert_eq!(plan.definition, catalog.first().unwrap().clone());
        assert_eq!(
            plan.names,
            vec!["Other".parse().unwrap(), "Ready".parse().unwrap()]
        );
        assert_eq!(plan.assignment, AssignmentChange::Changed);
        assert_eq!(plan.catalog, None);
        let repeated = assign(
            &catalog,
            &plan.names,
            &label("READY", LabelColor::Pink),
            Assignment::Assigned,
        );
        assert_eq!(repeated.names, plan.names);
        assert_eq!(repeated.assignment, AssignmentChange::Unchanged);
    }

    #[test]
    fn unknown_unassignment_is_noop_and_unknown_assignment_creates_once() {
        let requested = label("New", LabelColor::Emerald);
        let removed = assign(&[], &[], &requested, Assignment::Unassigned);
        assert!(removed.names.is_empty());
        assert_eq!(removed.catalog, None);
        assert_eq!(removed.assignment, AssignmentChange::Unchanged);
        let created = assign(&[], &[], &requested, Assignment::Assigned);
        assert_eq!(created.names, vec![requested.name.clone()]);
        assert_eq!(
            created.catalog,
            Some(CatalogDelta::Upsert {
                label: requested,
                previous: None
            })
        );
        let removed = assign(
            &[],
            &created.names,
            &label("new", LabelColor::Violet),
            Assignment::Unassigned,
        );
        assert!(removed.names.is_empty());
        assert_eq!(removed.catalog, None);
    }

    #[test]
    fn rename_collision_rejects_both_fields_and_case_only_rename_is_a_change() {
        let catalog = vec![
            label("Ready", LabelColor::Violet),
            label("Taken", LabelColor::Blue),
        ];
        let request = UpdateWorkspaceLabel {
            name: "ready".parse().unwrap(),
            new_name: Some("taken".parse().unwrap()),
            color: Some(LabelColor::Red),
        };
        assert!(matches!(
            update(&catalog, &request),
            Err(WorkspaceLabelFailure::NameTaken)
        ));
        let request = UpdateWorkspaceLabel {
            new_name: Some("READY".parse().unwrap()),
            ..request
        };
        let plan = update(&catalog, &request).unwrap();
        assert_eq!(plan.definition, label("READY", LabelColor::Red));
        assert!(
            matches!(plan.catalog, Some(CatalogDelta::Upsert { previous: Some(previous), .. }) if previous.as_str() == "Ready")
        );
        assert_eq!(catalog.first(), Some(&label("Ready", LabelColor::Violet)));
    }

    #[test]
    fn no_op_update_has_no_publication_and_missing_target_is_typed() {
        let catalog = vec![label("Ready", LabelColor::Violet)];
        let request = UpdateWorkspaceLabel {
            name: "ready".parse().unwrap(),
            new_name: None,
            color: None,
        };
        assert_eq!(update(&catalog, &request).unwrap().catalog, None);
        let recolor = update(
            &catalog,
            &UpdateWorkspaceLabel {
                color: Some(LabelColor::Sky),
                ..request
            },
        )
        .unwrap();
        assert!(matches!(
            recolor.catalog,
            Some(CatalogDelta::Upsert { previous: None, .. })
        ));
        assert!(matches!(
            update(
                &[],
                &UpdateWorkspaceLabel {
                    name: "Missing".parse().unwrap(),
                    new_name: None,
                    color: None
                }
            ),
            Err(WorkspaceLabelFailure::LabelNotFound)
        ));
    }
}
