//! A multi-provider agent's saved `OpenAI` login is not evidence that its
//! current conversation uses `OpenAI`. Follow the reference's explicit native
//! model prefix; missing/ambiguous selection is never guessed from a profile.
use super::LimitBinding;
use vessel_proto::{
    OptionCategory, OptionKind, SessionOption,
    limits::{LimitFailure, LimitLoginStore},
};
impl LimitBinding {
    /// Whether this store needs the live agent's selected-provider evidence.
    #[must_use]
    pub fn requires_model_evidence(&self) -> bool {
        match self.source() {
            LimitLoginStore::OpenCode => true,
            LimitLoginStore::ClaudeCode | LimitLoginStore::Codex => false,
        }
    }
    /// Check a current, typed agent snapshot before reading its saved login.
    /// Call again after a refresh: a provider switch must not reuse this report.
    /// # Errors
    /// Missing/ambiguous selection, or a model of another provider.
    pub fn check_session_model(&self, options: &[SessionOption]) -> Result<(), LimitFailure> {
        if !self.requires_model_evidence() {
            return Ok(());
        }
        let mut models = options
            .iter()
            .filter(|option| option.category == OptionCategory::Model);
        let Some(model) = models.next() else {
            return Err(LimitFailure::Unassociated);
        };
        if models.next().is_some() {
            return Err(LimitFailure::Unassociated);
        }
        match &model.kind {
            OptionKind::Select { current, .. } => match current.as_str().split_once('/') {
                Some(("openai", model)) if !model.is_empty() => Ok(()),
                Some((provider, model)) if !provider.is_empty() && !model.is_empty() => {
                    Err(LimitFailure::DifferentRoute)
                }
                Some(_) | None => Err(LimitFailure::Unassociated),
            },
            OptionKind::Toggle { .. } | OptionKind::Unknown => Err(LimitFailure::Unassociated),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HarnessEnv, LimitEnvironment, Profile, limits_binding};
    use vessel_proto::HarnessKind;
    fn selection(model: &str) -> SessionOption {
        SessionOption {
            id: vessel_proto::OptionId::new("model"),
            name: "Model".to_owned(),
            description: None,
            category: OptionCategory::Model,
            kind: OptionKind::Select {
                current: vessel_proto::OptionValueId::new(model),
                values: Vec::new(),
            },
        }
    }
    #[test]
    fn opencode_requires_one_current_openai_selection_not_a_stored_default() {
        let root = tempfile::tempdir().unwrap();
        let binding = limits_binding(
            &LimitEnvironment::new(),
            &HarnessEnv::new(root.path().to_owned()),
            &Profile::default_for(HarnessKind::OpenCode),
        )
        .unwrap();
        assert!(binding.requires_model_evidence());
        assert_eq!(
            binding.check_session_model(&[]),
            Err(LimitFailure::Unassociated)
        );
        assert_eq!(
            binding.check_session_model(&[selection("cheap")]),
            Err(LimitFailure::Unassociated)
        );
        assert_eq!(
            binding.check_session_model(&[selection("anthropic/claude")]),
            Err(LimitFailure::DifferentRoute)
        );
        assert_eq!(
            binding.check_session_model(&[selection("openai/gpt")]),
            Ok(())
        );
        assert_eq!(
            binding.check_session_model(&[selection("openai/gpt"), selection("openai/other")]),
            Err(LimitFailure::Unassociated)
        );
    }
}
