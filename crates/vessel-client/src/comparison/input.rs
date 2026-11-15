//! Parse startup inputs once; no arbitrary session spec or file buffer
//! crosses into the worker. Encoded bounds include JSON escaping.
use serde::{Deserialize, Deserializer, Serialize};
use std::io::{self, Write};
use vessel_proto::automation::PromptText;
use vessel_proto::workspace::{BranchName, InvalidText, WorktreeBase};
use vessel_proto::{
    AttachmentLimits, AttachmentRef, HarnessRef, ModelId, OptionChoice, ProfileName, ProjectId,
    SessionId,
};

/// Invalid input, before remote creation.
#[derive(Debug, thiserror::Error)]
pub enum InputError {
    /// At least one model, at most eight.
    #[error("choose between one and eight models")]
    Count,
    /// Duplicate provider/profile/model.
    #[error("each agent, profile and model selection occurs only once")]
    Duplicate,
    /// Too many or repeated choices.
    #[error("a selection takes at most 32 distinct choices")]
    Choices,
    /// Encoded metadata exceeds its budget.
    #[error("comparison metadata exceeds its encoded size limit")]
    Encoding(#[source] serde_json::Error),
    /// Random branch suffix failed.
    #[error("could not choose comparison branch names")]
    Random(#[source] getrandom::Error),
    /// Invalid branch suffix.
    #[error("could not form a comparison branch")]
    Branch(#[source] InvalidText),
    /// Invalid common file references.
    #[error("choose at most sixteen distinct attachments within the session byte limit")]
    Attachments,
    /// Empty common input.
    #[error("write a prompt or attach a file")]
    Empty,
}

/// A bounded structured selection, with no path, resume or side-link field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Selection {
    /// Agent on the selected node.
    pub harness: HarnessRef,
    /// Its existing profile.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<ProfileName>,
    /// Its model, or default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<ModelId>,
    /// Normal structured choices.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<OptionChoice>,
}
impl Selection {
    /// Validate a selection. The plan checks again after any public edit.
    ///
    /// # Errors
    /// More than 32 choices, duplicates, or more than 16 KiB encoded.
    pub fn new(
        harness: HarnessRef,
        profile: Option<ProfileName>,
        model: Option<ModelId>,
        options: Vec<OptionChoice>,
    ) -> Result<Self, InputError> {
        let selected = Self {
            harness,
            profile,
            model,
            options,
        };
        selected.check()?;
        Ok(selected)
    }
    fn check(&self) -> Result<(), InputError> {
        if self.options.len() > 32
            || self.options.iter().enumerate().any(|(at, choice)| {
                self.options
                    .iter()
                    .take(at)
                    .any(|earlier| earlier.id == choice.id)
            })
        {
            return Err(InputError::Choices);
        }
        encoded(self, 16 * 1024)
    }
}
impl<'de> Deserialize<'de> for Selection {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Normal wire structs tolerate future fields. A startup manifest is
        // deliberately stricter: misspelled placement/history/choice fields
        // must not be silently ignored before several resources are created.
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Agent {
            Known(vessel_proto::HarnessKind),
            Added(Added),
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Added {
            added: vessel_proto::agents::AgentName,
        }
        #[derive(Deserialize)]
        #[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
        enum Choice {
            Select {
                id: vessel_proto::OptionId,
                value: vessel_proto::OptionValueId,
            },
            Toggle {
                id: vessel_proto::OptionId,
                on: bool,
            },
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Given {
            harness: Agent,
            #[serde(default)]
            profile: Option<ProfileName>,
            #[serde(default)]
            model: Option<ModelId>,
            #[serde(default)]
            options: Vec<Choice>,
        }
        let given = Given::deserialize(deserializer)?;
        let harness = match given.harness {
            Agent::Known(kind) => kind.into(),
            Agent::Added(added) => HarnessRef::Added(added.added),
        };
        let options = given
            .options
            .into_iter()
            .map(|choice| match choice {
                Choice::Select { id, value } => OptionChoice {
                    id,
                    value: vessel_proto::ChoiceValue::Select { value },
                },
                Choice::Toggle { id, on } => OptionChoice {
                    id,
                    value: vessel_proto::ChoiceValue::Toggle { on },
                },
            })
            .collect();
        Self::new(harness, given.profile, given.model, options).map_err(serde::de::Error::custom)
    }
}

/// Immutable startup plan, not a server-side comparison resource.
#[derive(Debug)]
pub struct ComparisonPlan {
    pub(super) project: ProjectId,
    pub(super) base: WorktreeBase,
    pub(super) targets: Vec<(Selection, BranchName)>,
}
impl ComparisonPlan {
    /// Plan independent branches for these selections at one base.
    ///
    /// # Errors
    /// Empty/oversized/duplicate selections, metadata or randomness failure.
    pub fn new(
        project: ProjectId,
        base: WorktreeBase,
        selections: Vec<Selection>,
    ) -> Result<Self, InputError> {
        if selections.is_empty() || selections.len() > 8 {
            return Err(InputError::Count);
        }
        encoded(&selections, 128 * 1024)?;
        for (at, target) in selections.iter().enumerate() {
            target.check()?;
            if selections.iter().take(at).any(|earlier| {
                earlier.harness == target.harness
                    && earlier.profile == target.profile
                    && earlier.model == target.model
            }) {
                return Err(InputError::Duplicate);
            }
        }
        let targets = selections
            .into_iter()
            .map(|selection| {
                let mut random = [0_u8; 16];
                getrandom::fill(&mut random).map_err(InputError::Random)?;
                let suffix: String = format!("{:032x}", u128::from_be_bytes(random));
                let branch = format!("vessel/compare-{suffix}")
                    .parse()
                    .map_err(InputError::Branch)?;
                Ok((selection, branch))
            })
            .collect::<Result<Vec<_>, InputError>>()?;
        Ok(Self {
            project,
            base,
            targets,
        })
    }
}

/// Finished uploads on one ready target, reused after its turn starts.
#[derive(Debug)]
pub struct SourceAttachments {
    pub(super) session: SessionId,
    pub(super) files: Vec<AttachmentRef>,
}
impl SourceAttachments {
    /// Bind finished references within normal prompt/session limits.
    ///
    /// # Errors
    /// Empty, duplicate, too many or too large references.
    pub fn new(session: SessionId, files: Vec<AttachmentRef>) -> Result<Self, InputError> {
        let mut total = 0_u64;
        if files.is_empty() || files.len() > 16 {
            return Err(InputError::Attachments);
        }
        for (at, file) in files.iter().enumerate() {
            total = total
                .checked_add(file.size)
                .ok_or(InputError::Attachments)?;
            if total > AttachmentLimits::DEFAULT.per_session
                || files.iter().take(at).any(|earlier| earlier.id == file.id)
            {
                return Err(InputError::Attachments);
            }
        }
        Ok(Self { session, files })
    }
}

/// A candidate for the job's one common input. Debug omits its text.
pub struct ComparisonInput {
    pub(super) text: Option<PromptText>,
    pub(super) source: Option<SourceAttachments>,
}
impl ComparisonInput {
    /// Submit text, finished files, or both.
    ///
    /// # Errors
    /// Neither text nor attachments.
    pub fn new(
        text: Option<PromptText>,
        source: Option<SourceAttachments>,
    ) -> Result<Self, InputError> {
        if text.is_none() && source.is_none() {
            return Err(InputError::Empty);
        }
        Ok(Self { text, source })
    }
}
impl std::fmt::Debug for ComparisonInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComparisonInput")
            .field(
                "text_bytes",
                &self.text.as_ref().map_or(0, |text| text.as_str().len()),
            )
            .field("source", &self.source)
            .finish()
    }
}
fn encoded<T: Serialize>(value: &T, limit: usize) -> Result<(), InputError> {
    struct Budget(usize);
    impl Write for Budget {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0 = self
                .0
                .checked_sub(bytes.len())
                .ok_or_else(|| io::Error::other("metadata budget"))?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(Budget(limit), value).map_err(InputError::Encoding)
}

#[cfg(test)]
#[path = "input_tests.rs"]
mod tests;
