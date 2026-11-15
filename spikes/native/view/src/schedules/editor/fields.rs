//! Schedule text is validated at edit edges, never reparsed by drawing or save.
//! Optional limits use the pinned form's decimal-prefix/Unlimited behavior;
//! positive values still pass the node's existing typed bounds without clamping.
use crate::localization::{Number, Translator};
use vessel_proto::automation::{
    AutomationText, InvalidAutomationText, MaxRuns, OutOfRange, PromptText, ScheduleDefinition,
    ScheduleName,
};

/// Full typed source for one invalid field; Display is diagnostic only.
#[derive(Debug, thiserror::Error)]
pub enum FieldError {
    /// Optional name supplied but invalid.
    #[error("schedule name: {0}")]
    Name(#[source] InvalidAutomationText),
    /// Nonempty, bounded prompt required.
    #[error("schedule prompt: {0}")]
    Prompt(#[source] InvalidAutomationText),
    /// Positive decimal prefix cannot fit the node's integer representation.
    #[error("schedule run count: {0}")]
    Number(#[source] std::num::ParseIntError),
    /// Positive integer lies outside the node's bound.
    #[error("schedule run count range: {0}")]
    Range(#[source] OutOfRange),
}
impl FieldError {
    /// Whole localized field error, never a translation of diagnostic prose.
    #[must_use]
    pub fn label(&self, catalog: &Translator) -> String {
        let words = catalog.schedules();
        match self {
            Self::Name(_) => words.form_name_invalid().to_owned(),
            Self::Prompt(_) => u32::try_from(AutomationText::PROMPT_MAX)
                .ok()
                .and_then(|maximum| words.form_prompt_invalid(Number::count(maximum)).ok())
                .unwrap_or_else(|| words.display_unavailable().to_owned()),
            Self::Number(_) | Self::Range(_) => words
                .form_max_runs_invalid(Number::count(MaxRuns::MAX))
                .unwrap_or_else(|_| words.display_unavailable().to_owned()),
        }
    }
}
#[derive(Debug)]
struct Input<T> {
    text: String,
    value: Result<T, FieldError>,
    edited: bool,
}
/// Which visible text field owns validation feedback and its source binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    /// Optional display name.
    Name,
    /// Required prompt, initially blank without a premature warning.
    Prompt,
    /// Optional positive whole run count.
    Limit,
}
/// The new-chat form's three editable text fields; recurring prompts do not
/// expose this value at all, so a cadence-only edit preserves their definition.
#[derive(Debug)]
pub struct Fields {
    name: Input<Option<ScheduleName>>,
    prompt: Input<PromptText>,
    limit: Input<Option<MaxRuns>>,
}
/// Already parsed form values for one definition build.
pub(super) struct Values {
    pub name: Option<ScheduleName>,
    pub prompt: PromptText,
    pub max_runs: Option<MaxRuns>,
}
impl Fields {
    /// Initial empty form. Its missing prompt is explicitly invalid.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            name: Input {
                text: String::new(),
                value: Ok(None),
                edited: false,
            },
            prompt: Input {
                text: String::new(),
                value: "".parse().map_err(FieldError::Prompt),
                edited: false,
            },
            limit: Input {
                text: String::new(),
                value: Ok(None),
                edited: false,
            },
        }
    }
    /// Typed wire fields are opened without reparsing or normalizing content.
    #[must_use]
    pub fn open(definition: &ScheduleDefinition) -> Self {
        Self {
            name: Input {
                text: definition
                    .name
                    .as_ref()
                    .map_or_else(String::new, |name| name.as_str().to_owned()),
                value: Ok(definition.name.clone()),
                edited: false,
            },
            prompt: Input {
                text: definition.prompt.as_str().to_owned(),
                value: Ok(definition.prompt.clone()),
                edited: false,
            },
            limit: Input {
                text: definition
                    .max_runs
                    .map_or_else(String::new, |runs| runs.get().to_string()),
                value: Ok(definition.max_runs),
                edited: false,
            },
        }
    }
    /// Name input shown by the field, already cached.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name.text
    }
    /// Prompt input shown by the field, already cached.
    #[must_use]
    pub fn prompt(&self) -> &str {
        &self.prompt.text
    }
    /// Optional limit input shown by the field, already cached.
    #[must_use]
    pub fn limit(&self) -> &str {
        &self.limit.text
    }
    /// Optional blank name means clear, not a missing required value.
    pub fn set_name(&mut self, text: String) {
        let value = if text.trim().is_empty() {
            Ok(None)
        } else {
            text.parse().map(Some).map_err(FieldError::Name)
        };
        self.name = Input {
            text,
            value,
            edited: true,
        };
    }
    /// Prompt validation at the edit edge, once. A complete unedited wire
    /// prompt remains byte-identical when only another field changes.
    pub fn set_prompt(&mut self, text: String) {
        let value = text.trim().parse().map_err(FieldError::Prompt);
        self.prompt = Input {
            text,
            value,
            edited: true,
        };
    }
    /// Pinned `schedule-form-sheet.tsx:68–71` uses `parseInt(raw,10)`: a positive
    /// decimal prefix counts; blank, malformed, zero and negative mean Unlimited.
    /// The node's typed upper bound still refuses a positive out-of-range value.
    pub fn set_limit(&mut self, text: String) {
        let value = parse_limit(&text);
        self.limit = Input {
            text,
            value,
            edited: true,
        };
    }
    pub(super) fn values(&self) -> Result<Values, &FieldError> {
        Ok(Values {
            name: self.name.value.as_ref()?.clone(),
            prompt: self.prompt.value.as_ref()?.clone(),
            max_runs: *self.limit.value.as_ref()?,
        })
    }
    /// Cached feedback for an edited field only. Required blank input still
    /// blocks saving; opening a pristine form does not label it as a mistake.
    #[must_use]
    pub fn feedback(&self, field: Field) -> Option<&FieldError> {
        match field {
            Field::Name => self
                .name
                .edited
                .then(|| self.name.value.as_ref().err())
                .flatten(),
            Field::Prompt => self
                .prompt
                .edited
                .then(|| self.prompt.value.as_ref().err())
                .flatten(),
            Field::Limit => self
                .limit
                .edited
                .then(|| self.limit.value.as_ref().err())
                .flatten(),
        }
    }
    /// Field validity is independent of project/model/cadence readiness.
    #[must_use]
    pub fn error(&self) -> Option<&FieldError> {
        self.values().err()
    }
}

// Match the source JavaScript form at the edit edge, without a JS runtime or
// floating-point conversion. A large positive prefix remains a typed refusal;
// it is never clamped to the node maximum or cleared as a representation error.
fn parse_limit(text: &str) -> Result<Option<MaxRuns>, FieldError> {
    let text = text.trim_start_matches(parse_int_space);
    if text.starts_with('-') {
        return Ok(None);
    }
    let unsigned = text.strip_prefix('+').unwrap_or(text);
    let digits = unsigned
        .split(|ch: char| !ch.is_ascii_digit())
        .next()
        .unwrap_or_default();
    if digits.is_empty() {
        return Ok(None);
    }
    let value = digits.parse::<u32>().map_err(FieldError::Number)?;
    if value == 0 {
        return Ok(None);
    }
    MaxRuns::try_from(value)
        .map(Some)
        .map_err(FieldError::Range)
}
// ECMAScript leading WhiteSpace/LineTerminator, including BOM but not NEL.
fn parse_int_space(ch: char) -> bool {
    matches!(
        ch,
        '\u{0009}'..='\u{000d}' | ' ' | '\u{00a0}' | '\u{1680}'
        | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}'
        | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'
    )
}

#[cfg(test)]
mod tests;
