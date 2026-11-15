//! Parse side input before issuing work, sharing the node's prompt bound.

use super::Problem;
use vessel_proto::automation::PromptText;

/// A question accepted by the side-only ingress, never a slash command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question(PromptText);

impl Question {
    /// Check the existing prompt contract and refuse command interpretation.
    /// # Errors
    /// Blank, oversized, control-bearing or slash-prefixed input is refused.
    pub fn parse(text: &str) -> Result<Self, Problem> {
        let meaningful = text.trim_start_matches(|c: char| c.is_whitespace() || c == '\u{feff}');
        if meaningful.starts_with('/') {
            return Err(Problem::Slash);
        }
        // BOM-only input must reach the same canonical empty-prompt validator;
        // otherwise preserve the person's exact indentation and message bytes.
        let validated = if meaningful.is_empty() {
            meaningful
        } else {
            text
        };
        validated
            .parse()
            .map(Self)
            .map_err(Problem::InvalidQuestion)
    }

    /// The existing wire value, already validated at the input edge.
    #[must_use]
    pub const fn prompt(&self) -> &PromptText {
        &self.0
    }
}
