//! A completed action carries observed selectors/effect facts, not only a tab id.
use super::{BrowserTool, CssDelta, CssPoint, ElementRef, InputText, RequiredInput};
use serde::{Deserialize, Serialize};

/// Construct only after actual dispatch/selection acknowledgement. These values
/// describe effects; they grant no input, filesystem or reference authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ActionOutcome {
    /// Actual selected element and trusted click point.
    Click {
        /// Current ref resolved by the completed transaction.
        element: ElementRef,
        /// Actual CSS point dispatched to the page.
        position: CssPoint,
    },
    /// The editable control whose replacement was acknowledged.
    Fill {
        /// Current editable ref, not a selector echoed before execution.
        element: ElementRef,
    },
    /// Focus/ref and optional point used by the completed text transaction.
    Type {
        /// Missing means the already-focused target, as requested.
        element: Option<ElementRef>,
        /// Present only when the executor used an observed point.
        position: Option<CssPoint>,
    },
    /// Executed chord identity, with actual selected focus/ref context.
    Keypress {
        /// Supported chord whose dispatch completed; redacted from Debug.
        key: RequiredInput,
        /// Optional current ref used to establish focus.
        element: Option<ElementRef>,
        /// Present only when the executor used an observed point.
        position: Option<CssPoint>,
    },
    /// The option actually selected, including an empty value when valid.
    Select {
        /// The select control resolved for the completed operation.
        element: ElementRef,
        /// Selected page content, redacted from Debug.
        value: InputText,
    },
    /// Actionable element and point of the trusted move.
    Hover {
        /// Current hovered element ref.
        element: ElementRef,
        /// Actual CSS destination of the acknowledged move.
        position: CssPoint,
    },
    /// Both current elements and actual endpoints of one trusted transaction.
    Drag {
        /// Current element whose point received the press.
        source: ElementRef,
        /// Current element whose point received the release.
        target: ElementRef,
        /// Actual CSS starting point.
        source_position: CssPoint,
        /// Actual CSS ending point.
        target_position: CssPoint,
    },
    /// Sent input deltas, not resulting document displacement.
    Scroll {
        /// Missing means the viewport rather than an element.
        element: Option<ElementRef>,
        /// Horizontal input delta actually dispatched.
        delta_x: CssDelta,
        /// Vertical input delta actually dispatched.
        delta_y: CssDelta,
        /// Actual CSS point used for the input event.
        position: CssPoint,
    },
}
impl ActionOutcome {
    /// A completed action cannot masquerade as another catalogued tool.
    #[must_use]
    pub const fn tool(&self) -> BrowserTool {
        match self {
            Self::Click { .. } => BrowserTool::Click,
            Self::Fill { .. } => BrowserTool::Fill,
            Self::Type { .. } => BrowserTool::Type,
            Self::Keypress { .. } => BrowserTool::Keypress,
            Self::Select { .. } => BrowserTool::Select,
            Self::Hover { .. } => BrowserTool::Hover,
            Self::Drag { .. } => BrowserTool::Drag,
            Self::Scroll { .. } => BrowserTool::Scroll,
        }
    }
}

/// The successful predicate kind is a fact of the completed wait operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum WaitMatch {
    /// Visible text contained the admitted substring.
    Text,
    /// The observed location contained the admitted substring.
    Url,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_mode_has_a_closed_tool_and_round_trips_its_required_completed_facts() {
        for (source, tool) in [
            (
                r#"{"action":"click","element":"@e1","position":{"x":2,"y":3}}"#,
                BrowserTool::Click,
            ),
            (r#"{"action":"fill","element":"@e1"}"#, BrowserTool::Fill),
            (
                r#"{"action":"type","element":null,"position":null}"#,
                BrowserTool::Type,
            ),
            (
                r#"{"action":"keypress","key":"Enter","element":null,"position":null}"#,
                BrowserTool::Keypress,
            ),
            (
                r#"{"action":"select","element":"@e1","value":""}"#,
                BrowserTool::Select,
            ),
            (
                r#"{"action":"hover","element":"@e1","position":{"x":2,"y":3}}"#,
                BrowserTool::Hover,
            ),
            (
                r#"{"action":"drag","source":"@e1","target":"@e2","source_position":{"x":2,"y":3},"target_position":{"x":4,"y":5}}"#,
                BrowserTool::Drag,
            ),
            (
                r#"{"action":"scroll","element":null,"delta_x":0,"delta_y":4,"position":{"x":2,"y":3}}"#,
                BrowserTool::Scroll,
            ),
        ] {
            let outcome: ActionOutcome = serde_json::from_str(source).unwrap();
            assert_eq!(outcome.tool(), tool);
            let encoded = serde_json::to_string(&outcome).unwrap();
            assert_eq!(
                serde_json::from_str::<ActionOutcome>(&encoded).unwrap(),
                outcome
            );
        }
    }

    #[test]
    fn missing_or_unknown_effects_and_invalid_points_cannot_default_to_applied() {
        for source in [
            r#"{"kind":"applied","browser_id":"00000000-0000-4000-8000-000000000001"}"#,
            r#"{"kind":"applied","browser_id":"00000000-0000-4000-8000-000000000001","outcome":{"action":"future"}}"#,
            r#"{"kind":"applied","browser_id":"00000000-0000-4000-8000-000000000001","outcome":{"action":"click","element":"@e1","position":{"x":-1,"y":3}}}"#,
        ] {
            assert!(serde_json::from_str::<super::super::BrowserResult>(source).is_err());
        }
        let secret = "synthetic-result-secret";
        let outcome = ActionOutcome::Select {
            element: "@e1".parse().unwrap(),
            value: InputText::try_from(secret.to_owned()).unwrap(),
        };
        assert!(!format!("{outcome:?}").contains(secret));
    }
}
