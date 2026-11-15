//! Decode only documented question-answer envelopes at the CLI edge.
//! Generic header/value mapping stays in vessel-client's shared form codec;
//! arbitrary provider tool edits remain opaque and cannot enter this path.
use super::ModifiedInput;
use serde::Deserialize;
use std::{collections::BTreeMap, fmt};
use vessel_proto::QuestionAnswer;

/// Either canonical typed values or Paseo's header-keyed answer map.
pub(crate) enum QuestionInput {
    Canonical(QuestionAnswer),
    Headers(BTreeMap<String, String>),
}
impl fmt::Debug for QuestionInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Canonical(_) => "QuestionInput::Canonical([redacted])",
            Self::Headers(_) => "QuestionInput::Headers([redacted])",
        })
    }
}

/// Input-free semantic errors; provider or answer text never enters diagnostics.
#[derive(Debug, thiserror::Error)]
pub(crate) enum InputError {
    #[error("question --input needs an answers object or an accepted answer with field values")]
    Layout,
    #[error("question --input needs an answers object or an accepted answer with field values")]
    Decode(#[source] DecodeFailure),
    #[error("permit allow cannot submit a declined question answer; use permit deny")]
    Decline,
}

/// Keep the original typed parser failure behind an opaque diagnostic boundary.
/// Its source chain must stop here: serde may put an answer inside an unknown-
/// variant message, and the CLI prints every automatically exposed cause.
#[derive(thiserror::Error)]
#[error("the question input does not match the answer schema")]
pub(crate) struct DecodeFailure(serde_json::Error);

impl fmt::Debug for DecodeFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("DecodeFailure")
            .field(&self.0.classify())
            .finish()
    }
}

// This checks the envelope's keys, not the provider schema or answer values.
// Reuse QuestionAnswer's authoritative decoder for canonical value types.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    #[serde(default, deserialize_with = "headers")]
    answers: Option<BTreeMap<String, String>>,
    #[serde(default, rename = "answer", deserialize_with = "present")]
    has_answer: bool,
    #[serde(default, rename = "values", deserialize_with = "present")]
    has_values: bool,
}

// A present JSON null must not be mistaken for an absent envelope key.
fn present<'de, D: serde::Deserializer<'de>>(input: D) -> Result<bool, D::Error> {
    serde::de::IgnoredAny::deserialize(input)?;
    Ok(true)
}

fn headers<'de, D: serde::Deserializer<'de>>(
    input: D,
) -> Result<Option<BTreeMap<String, String>>, D::Error> {
    BTreeMap::deserialize(input).map(Some)
}

impl ModifiedInput {
    /// Parse after the selected request is known to be a question, before effects.
    /// Header ambiguity and field validation are the shared codec's next step.
    pub(crate) fn question(&self) -> Result<QuestionInput, InputError> {
        let envelope: Envelope = serde_json::from_str(self.raw().as_text())
            .map_err(DecodeFailure)
            .map_err(InputError::Decode)?;
        match (envelope.answers, envelope.has_answer, envelope.has_values) {
            (Some(headers), false, false) => Ok(QuestionInput::Headers(headers)),
            (None, true, _) => {
                let answer: QuestionAnswer = serde_json::from_str(self.raw().as_text())
                    .map_err(DecodeFailure)
                    .map_err(InputError::Decode)?;
                match answer {
                    accepted @ QuestionAnswer::Accept { .. } => {
                        Ok(QuestionInput::Canonical(accepted))
                    }
                    QuestionAnswer::Decline => Err(InputError::Decline),
                }
            }
            (Some(_), true, _) | (Some(_), false, true) | (None, false, _) => {
                Err(InputError::Layout)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::conversation::AnswerValue;

    #[test]
    fn header_maps_and_canonical_values_remain_distinct_and_private() {
        let input: ModifiedInput = r#"{"answers":{"Question 1":"synthetic-private-answer"}}"#
            .parse()
            .unwrap();
        let parsed = input.question().unwrap();
        assert!(!format!("{parsed:?}").contains("synthetic-private-answer"));
        let QuestionInput::Headers(headers) = parsed else {
            panic!("header map")
        };
        assert_eq!(
            headers.get("Question 1").map(String::as_str),
            Some("synthetic-private-answer")
        );
        let input: ModifiedInput = r#"{"answer":"accept","values":{"field":{"type":"choices","value":["actual-value","other-value"]}}}"#.parse().unwrap();
        let parsed = input.question().unwrap();
        assert!(!format!("{parsed:?}").contains("actual-value"));
        let QuestionInput::Canonical(QuestionAnswer::Accept { values }) = parsed else {
            panic!("typed values")
        };
        assert_eq!(
            values.get(&"field".parse().unwrap()),
            Some(&AnswerValue::Choices(vec![
                "actual-value".into(),
                "other-value".into()
            ]))
        );
    }

    #[test]
    fn contradictory_or_extra_edit_envelopes_are_refused_before_mapping() {
        for json in [
            r#"{"answers":{},"answer":"accept","values":{}}"#,
            r#"{"answers":{},"answer":null}"#,
            r#"{"answers":{},"values":null}"#,
            r#"{"answer":"accept","values":{},"answers":null}"#,
            r#"{"answers":null}"#,
            r#"{"answers":{},"updatedPermissions":["arbitrary"]}"#,
            r#"{"answers":{"Header":["not a string"]}}"#,
            r#"{"answer":"accept","values":null}"#,
            r#"{"values":{}}"#,
            "false",
            "null",
            "0",
            "[]",
        ] {
            let input: ModifiedInput = json.parse().unwrap();
            assert!(matches!(
                input.question(),
                Err(InputError::Layout | InputError::Decode(_))
            ));
            assert_eq!(
                input.raw().as_text(),
                json,
                "opaque permission input remains unchanged"
            );
        }
        let input: ModifiedInput = r#"{"answer":"decline"}"#.parse().unwrap();
        assert!(matches!(input.question(), Err(InputError::Decline)));
    }

    #[test]
    fn schema_failure_retains_the_parser_without_printing_answers_in_any_cause() {
        let input: ModifiedInput = r#"{"answer":"synthetic-private-invalid-answer","values":{}}"#
            .parse()
            .unwrap();
        let error = input.question().unwrap_err();
        let InputError::Decode(failure) = &error else {
            panic!("retained parser failure")
        };
        assert_eq!(failure.0.classify(), serde_json::error::Category::Data);
        assert!(std::error::Error::source(&error).is_some());
        assert!(!format!("{error:?}").contains("synthetic-private-invalid-answer"));
        let mut cause: Option<&dyn std::error::Error> = Some(&error);
        while let Some(current) = cause {
            assert!(
                !current
                    .to_string()
                    .contains("synthetic-private-invalid-answer")
            );
            assert!(!format!("{current:?}").contains("synthetic-private-invalid-answer"));
            cause = current.source();
        }
    }
}
