//! Plan proposals share one outcome model between their dock and stream.
//! A tool finishing does not prove approval: only its permission record does.

use serde::Deserialize;
use std::borrow::Cow;
use std::collections::HashMap;
use vessel_proto::timeline::ToolCallDetail;
use vessel_proto::{PermissionOutcome, ToolCallId, ToolKind};

use super::{Entry, PlanExpansion, PlanOutcome, ToolDetailSource, allows};

/// User choices survive text updates, but a new decision starts a fresh card.
#[derive(Debug, Default)]
pub struct PlanDisclosures(HashMap<ToolCallId, (PlanOutcome, PlanExpansion)>);

impl PlanDisclosures {
    /// The current decision owns its disclosure; an old decision's toggle cannot leak.
    #[must_use]
    pub fn expansion(&self, tool: &ToolCallId, outcome: PlanOutcome) -> PlanExpansion {
        self.0
            .get(tool)
            .filter(|(saved, _)| *saved == outcome)
            .map_or_else(
                || PlanExpansion::for_outcome(outcome),
                |(_, expansion)| *expansion,
            )
    }

    /// Record one deliberate toggle for this tool and decision.
    pub fn toggle(&mut self, tool: &ToolCallId, outcome: PlanOutcome) {
        let expansion = self.expansion(tool, outcome).toggled();
        let _previous = self.0.insert(tool.clone(), (outcome, expansion));
    }

    /// A different conversation must not inherit local card choices.
    pub fn clear(&mut self) {
        self.0.clear();
    }
}

/// The legacy wire carries a plan in opaque tool input. Decode only this
/// field here until the canonical detail contract supplies it explicitly.
#[derive(Deserialize)]
struct PlanInput {
    plan: String,
}

/// Canonical Plan semantics win over legacy kind/input in every stream path.
/// Only an absent detail may use the old opaque plan field; Markdown is borrowed
/// from a real canonical producer and legacy decoding owns its resulting text.
#[must_use]
pub fn plan_text(source: &impl ToolDetailSource) -> Option<Cow<'_, str>> {
    match source.detail() {
        Some(ToolCallDetail::Plan(plan)) => return Some(Cow::Borrowed(&plan.text)),
        Some(
            ToolCallDetail::Shell(_)
            | ToolCallDetail::Read(_)
            | ToolCallDetail::Edit(_)
            | ToolCallDetail::Write(_)
            | ToolCallDetail::Search(_)
            | ToolCallDetail::Fetch(_)
            | ToolCallDetail::WorktreeSetup(_)
            | ToolCallDetail::SubAgent(_)
            | ToolCallDetail::PlainText(_)
            | ToolCallDetail::Unknown(_),
        ) => return None,
        None => {}
    }
    let tool = source.tool();
    if tool.kind != ToolKind::SwitchMode {
        return None;
    }
    serde_json::from_str::<PlanInput>(tool.input.as_ref()?.as_text())
        .ok()
        .map(|input| input.plan)
        .filter(|plan| !plan.trim().is_empty())
        .map(Cow::Owned)
}

impl super::Timeline {
    /// A plan's permission changes its stream row too, so virtual layout
    /// invalidates the proposal's earlier anchor when its decision arrives.
    pub(super) fn plan_permission_changed(&mut self, tool: &ToolCallId) {
        let Some(index) = self.tools.get(tool).copied() else {
            return;
        };
        if self
            .entries
            .get(index)
            .and_then(|entry| entry.kind.as_tool())
            .and_then(plan_text)
            .is_some()
        {
            let _changed = self.change(index);
        }
    }
}

/// The latest permission for this tool determines its displayed decision;
/// another tool's decision and unknown option ids never imply approval.
#[must_use]
pub fn plan_outcome(entries: &[Entry], tool: &ToolCallId) -> PlanOutcome {
    let permission = entries
        .iter()
        .rev()
        .filter_map(|entry| entry.kind.as_permission())
        .find(|permission| permission.request.tool_call == *tool);
    let Some(permission) = permission else {
        return PlanOutcome::Proposed;
    };
    match &permission.outcome {
        None => PlanOutcome::Pending,
        Some(PermissionOutcome::Cancelled) => PlanOutcome::Canceled,
        Some(PermissionOutcome::Selected { option, .. }) => permission
            .request
            .options
            .iter()
            .find(|offered| offered.id == *option)
            .map_or(PlanOutcome::Proposed, |offered| {
                if allows(offered.kind) {
                    PlanOutcome::Approved
                } else {
                    PlanOutcome::Rejected
                }
            }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use vessel_proto::{EventRecord, EventSeq, SessionEvent};

    fn record(seq: u64, event: serde_json::Value) -> EventRecord {
        EventRecord {
            seq: EventSeq::new(seq),
            at: chrono::DateTime::UNIX_EPOCH,
            event: SessionEvent::Agent {
                event: serde_json::from_value(event).unwrap(),
            },
        }
    }

    fn proposal() -> EventRecord {
        record(
            0,
            json!({"event":"tool_call", "id":"plan", "title":"Leave plan mode",
            "kind":"switch_mode", "status":"in_progress", "raw_input":{"plan":"## A plan\n\nRead, then fix."}}),
        )
    }

    fn permission() -> EventRecord {
        record(
            1,
            json!({"event":"permission_requested", "id":"00000000-0000-4000-8000-000000000001",
            "tool_call":"plan", "options":[
                {"id":"allow", "name":"Implement", "kind":"allow_once"},
                {"id":"deny", "name":"Keep planning", "kind":"reject_once"}]}),
        )
    }

    #[test]
    fn a_new_decision_resets_disclosure_but_same_decision_keeps_the_toggle() {
        let tool = ToolCallId::new("plan");
        let mut cards = PlanDisclosures::default();
        cards.toggle(&tool, PlanOutcome::Proposed);
        assert_eq!(
            cards.expansion(&tool, PlanOutcome::Proposed),
            PlanExpansion::Collapsed
        );
        assert_eq!(
            cards.expansion(&tool, PlanOutcome::Approved),
            PlanExpansion::Expanded
        );
        cards.toggle(&tool, PlanOutcome::Rejected);
        assert_eq!(
            cards.expansion(&tool, PlanOutcome::Rejected),
            PlanExpansion::Expanded
        );
        assert_eq!(
            cards.expansion(&tool, PlanOutcome::Canceled),
            PlanExpansion::Collapsed
        );
        cards.clear();
        assert_eq!(
            cards.expansion(&tool, PlanOutcome::Rejected),
            PlanExpansion::Collapsed
        );
    }

    #[test]
    fn explicit_decisions_reveal_the_original_anchor_and_never_assume_an_unknown_allow() {
        let tool = ToolCallId::new("plan");
        for (outcome, expected) in [
            (
                json!({"outcome":"selected", "option":"allow"}),
                PlanOutcome::Approved,
            ),
            (
                json!({"outcome":"selected", "option":"deny"}),
                PlanOutcome::Rejected,
            ),
            (json!({"outcome":"cancelled"}), PlanOutcome::Canceled),
            (
                json!({"outcome":"selected", "option":"not-offered"}),
                PlanOutcome::Proposed,
            ),
        ] {
            let mut timeline = super::super::Timeline::default();
            let _added = timeline.apply(&[proposal()]);
            assert_eq!(
                plan_outcome(timeline.entries(), &tool),
                PlanOutcome::Proposed
            );
            assert!(
                !timeline.entries()[0].kind.folds(),
                "a plan never enters a tool group"
            );
            let pending = timeline.apply(&[permission()]);
            assert_eq!(
                pending.changed,
                [0],
                "the earlier proposal must be measured again"
            );
            assert_eq!(
                plan_outcome(timeline.entries(), &tool),
                PlanOutcome::Pending
            );
            let changed = timeline.apply(&[record(2, json!({
                "event":"permission_resolved", "id":"00000000-0000-4000-8000-000000000001", "outcome":outcome
            }))]);
            assert_eq!(changed.changed, [0, 1]);
            assert_eq!(plan_outcome(timeline.entries(), &tool), expected);
            assert_eq!(
                plan_outcome(timeline.entries(), &ToolCallId::new("other")),
                PlanOutcome::Proposed
            );
        }
    }

    #[test]
    fn only_negative_decisions_start_folded() {
        for state in [
            PlanOutcome::Proposed,
            PlanOutcome::Pending,
            PlanOutcome::Approved,
        ] {
            assert!(state.expanded());
        }
        for state in [PlanOutcome::Rejected, PlanOutcome::Canceled] {
            assert!(!state.expanded());
        }
        assert_eq!(
            plan_outcome(&[], &ToolCallId::new("plan")),
            PlanOutcome::Proposed
        );
    }
    #[test]
    fn canonical_plan_semantics_control_all_disclosure_paths_without_legacy_input() {
        use super::super::ToolView;
        use vessel_proto::timeline::{PlanDetail, ToolFailure, UnknownToolDetail};
        struct Source {
            tool: ToolView,
            detail: Option<ToolCallDetail>,
        }
        impl ToolDetailSource for Source {
            fn tool(&self) -> &ToolView {
                &self.tool
            }
            fn detail(&self) -> Option<&ToolCallDetail> {
                self.detail.as_ref()
            }
            fn failure(&self) -> Option<&ToolFailure> {
                None
            }
        }
        let mut timeline = super::super::Timeline::default();
        let _change = timeline.apply(&[proposal()]);
        let mut source = Source {
            tool: timeline
                .tool_call(&ToolCallId::new("plan"))
                .unwrap()
                .clone(),
            detail: None,
        };
        assert_eq!(
            plan_text(&source).as_deref(),
            Some("## A plan\n\nRead, then fix.")
        );
        source.detail = Some(ToolCallDetail::Unknown(UnknownToolDetail {
            input: None,
            output: None,
        }));
        assert!(
            plan_text(&source).is_none(),
            "canonical non-plan must not fall back to old input"
        );
        source.tool.kind = ToolKind::Other;
        source.tool.input = None;
        source.detail = Some(ToolCallDetail::Plan(PlanDetail {
            text: "# Canonical".to_owned(),
        }));
        assert_eq!(plan_text(&source).as_deref(), Some("# Canonical"));
        source.detail = Some(ToolCallDetail::Plan(PlanDetail {
            text: String::new(),
        }));
        assert_eq!(plan_text(&source).as_deref(), Some(""));
    }
}
