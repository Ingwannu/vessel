//! Model replacement/addition rules, preserving discovery order and warnings.
//! No model is probed or selected by these pure merge operations.

use std::collections::BTreeSet;
use vessel_proto::{
    ModelId,
    providers::{CatalogueError, DefaultChoice, Model, ModelVisibility, Models},
};

/// A normalized list plus the IDs that require a duplicate-catalogue warning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergedModels {
    /// Shared catalogue value after replacement/addition and normalization.
    pub models: Models,
    /// Repeated runtime IDs, first definition retained in the catalogue.
    pub duplicates: Vec<ModelId>,
}

/// Merge profile replacement models (when nonempty) and additional definitions
/// over runtime discovery. Missing addition flags preserve existing metadata;
/// explicitly added compatibility aliases become selectable.
///
/// # Errors
/// `CatalogueError` refuses an invalid or oversized merged catalogue atomically.
pub fn merge_models(
    runtime: &Models,
    replacement: Option<&Models>,
    additions: &Models,
) -> Result<MergedModels, CatalogueError> {
    let base = replacement
        .filter(|models| !models.all().is_empty())
        .unwrap_or(runtime);
    let mut merged = base.all().to_vec();
    let overrides_default = additions
        .all()
        .iter()
        .any(|m| m.spec().default == Some(DefaultChoice::Yes));
    for addition in additions.all() {
        if let Some(existing) = merged
            .iter_mut()
            .find(|model| model.spec().id == addition.spec().id)
        {
            let mut spec = existing.spec().clone();
            let next = addition.spec();
            spec.label = next.label.clone();
            spec.default = next.default.or(spec.default);
            spec.visibility = next.visibility.or_else(|| {
                if spec.visibility == Some(ModelVisibility::Compatibility) {
                    Some(ModelVisibility::Selectable)
                } else {
                    spec.visibility
                }
            });
            spec.thinking = next.thinking.clone().or(spec.thinking);
            spec.default_thinking = next.default_thinking.clone().or(spec.default_thinking);
            *existing = Model::new(spec)?;
        } else {
            merged.push(addition.clone());
        }
    }
    if overrides_default {
        let defaults: BTreeSet<_> = additions
            .all()
            .iter()
            .filter(|m| m.spec().default == Some(DefaultChoice::Yes))
            .map(|m| &m.spec().id)
            .collect();
        for model in &mut merged {
            if !defaults.contains(&model.spec().id) {
                let mut spec = model.spec().clone();
                spec.default = Some(DefaultChoice::No);
                *model = Model::new(spec)?;
            }
        }
    }
    normalize(&Models::new(merged)?)
}

/// Normalize repeated IDs with a first-wins rule, retaining diagnostic evidence.
///
/// # Errors
/// `CatalogueError` preserves the bounded output contract.
pub fn normalize(models: &Models) -> Result<MergedModels, CatalogueError> {
    let mut seen = BTreeSet::new();
    let mut duplicates = Vec::new();
    let mut normalized = Vec::new();
    for model in models.all() {
        if seen.insert(&model.spec().id) {
            normalized.push(model.clone());
        } else {
            duplicates.push(model.spec().id.clone());
        }
    }
    Ok(MergedModels {
        models: Models::new(normalized)?,
        duplicates,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::{
        OptionValueId,
        providers::{ModelSpec, ThinkingChoice, ThinkingChoices},
    };

    fn model(id: &str, default: Option<DefaultChoice>) -> Model {
        Model::new(ModelSpec {
            id: ModelId::new(id),
            label: id.parse().unwrap(),
            default,
            visibility: None,
            thinking: None,
            default_thinking: None,
        })
        .unwrap()
    }

    #[test]
    fn replacement_and_additions_keep_order_and_new_defaults_clear_old_ones() {
        let runtime = Models::new(vec![model("runtime", Some(DefaultChoice::Yes))]).unwrap();
        let replacement = Models::new(vec![model("configured", Some(DefaultChoice::Yes))]).unwrap();
        let additions = Models::new(vec![model("added", Some(DefaultChoice::Yes))]).unwrap();
        let result = merge_models(&runtime, Some(&replacement), &additions).unwrap();
        assert_eq!(
            result
                .models
                .all()
                .iter()
                .map(|m| m.spec().id.as_str())
                .collect::<Vec<_>>(),
            ["configured", "added"]
        );
        assert_eq!(
            result
                .models
                .all()
                .iter()
                .map(|m| m.spec().default)
                .collect::<Vec<_>>(),
            [Some(DefaultChoice::No), Some(DefaultChoice::Yes)]
        );
        assert_eq!(
            merge_models(&runtime, Some(&Models::default()), &Models::default())
                .unwrap()
                .models,
            runtime
        );
    }

    #[test]
    fn added_alias_becomes_selectable_and_missing_thinking_metadata_is_preserved() {
        let mut spec = model("alias", None).spec().clone();
        spec.visibility = Some(ModelVisibility::Compatibility);
        spec.thinking = Some(
            ThinkingChoices::new(vec![ThinkingChoice {
                id: OptionValueId::new("high"),
                label: "High".parse().unwrap(),
                default: None,
            }])
            .unwrap(),
        );
        spec.default_thinking = Some(OptionValueId::new("high"));
        let runtime = Models::new(vec![Model::new(spec.clone()).unwrap()]).unwrap();
        let result = merge_models(
            &runtime,
            None,
            &Models::new(vec![model("alias", None)]).unwrap(),
        )
        .unwrap();
        let merged = result.models.all().first().unwrap().spec();
        assert_eq!(merged.visibility, Some(ModelVisibility::Selectable));
        assert_eq!(merged.thinking, spec.thinking);
        assert_eq!(merged.default_thinking, spec.default_thinking);
    }

    #[test]
    fn duplicate_runtime_definitions_retain_first_and_report_the_duplicate() {
        let first = model("same", Some(DefaultChoice::Yes));
        let list =
            Models::new(vec![first.clone(), model("same", Some(DefaultChoice::No))]).unwrap();
        let result = normalize(&list).unwrap();
        assert_eq!(result.models.all(), [first]);
        assert_eq!(result.duplicates, [ModelId::new("same")]);
    }
}
