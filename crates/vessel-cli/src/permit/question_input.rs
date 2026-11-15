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
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum InputError {
    #[error("question --input needs an answers object or an accepted answer with field values")]
    Layout,
    #[error("permit allow cannot submit a declined question answer; use permit deny")]
    Decline,
}

// This checks the envelope's keys, not the provider schema or answer values.
// Reuse QuestionAnswer's authoritative decoder for canonical value types.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    #[serde(default)]
    answers: Option<BTreeMap<String, String>>,
    #[serde(default)]
    answer: Option<serde::de::IgnoredAny>,
    #[serde(default)]
    values: Option<serde::de::IgnoredAny>,
}

impl ModifiedInput {
    /// Parse after the selected request is known to be a question, before effects.
    /// Header ambiguity and field validation are the shared codec's next step.
    pub(crate) fn question(&self) -> Result<QuestionInput, InputError> {
        let envelope: Envelope =
            serde_json::from_str(self.raw().as_text()).map_err(|_shape| InputError::Layout)?;
        match (envelope.answers, envelope.answer, envelope.values) {
            (Some(headers), None, None) => Ok(QuestionInput::Headers(headers)),
            (None, Some(_), _) => {
                let answer: QuestionAnswer = serde_json::from_str(self.raw().as_text())
                    .map_err(|_shape| InputError::Layout)?;
                match answer {
                    accepted @ QuestionAnswer::Accept { .. } => {
                        Ok(QuestionInput::Canonical(accepted))
                    }
                    QuestionAnswer::Decline => Err(InputError::Decline),
                }
            }
            (Some(_), Some(_), _) | (Some(_), None, Some(_)) | (None, None, _) => {
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
            assert_eq!(input.question().err(), Some(InputError::Layout));
            assert_eq!(
                input.raw().as_text(),
                json,
                "opaque permission input remains unchanged"
            );
        }
        let input: ModifiedInput = r#"{"answer":"decline"}"#.parse().unwrap();
        assert_eq!(input.question().err(), Some(InputError::Decline));
    }
}
