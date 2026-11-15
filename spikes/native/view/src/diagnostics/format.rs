//! The shareable text only renders allowlisted facts and closed failure reasons.
use super::collection::{HostRow, MAX_REPORT_BYTES, Problem, Progress, ReportText};
use std::fmt::Write as _;
use vessel_proto::NodeState;

pub(super) fn combined(
    client: Option<&ReportText>,
    rows: &[HostRow],
) -> Result<ReportText, Problem> {
    let mut text = client.map_or_else(String::new, |client| client.as_str().to_owned());
    if !rows.is_empty() {
        let _written = write!(text, "\n  Requested host reports: {}", rows.len());
    }
    for (index, row) in rows.iter().enumerate() {
        // Saturation only guards the ordinal; the row bound keeps it well below usize::MAX.
        let ordinal = index.saturating_add(1);
        let _written = writeln!(text, "\n\nHost {ordinal}");
        match &row.progress {
            Progress::Done(answer) | Progress::Partial(answer) => {
                match &answer.facts {
                    Ok(facts) => {
                        let state = match facts.state {
                            NodeState::Serving => "serving",
                            NodeState::Draining => "draining",
                        };
                        let _written = writeln!(
                            text,
                            "  Node ID: {}\n  Protocol: {}\n  Lifecycle: {state}",
                            facts.node, facts.protocol
                        );
                    }
                    Err(problem) => {
                        let _written = writeln!(text, "  Node info: {problem}");
                    }
                }
                text.push_str(answer.daemon.render()?.as_str());
                text.push('\n');
            }
            Progress::Failed(problem) => {
                let _written = writeln!(text, "  Node info: {problem}");
            }
            Progress::Pending | Progress::Running => return Err(Problem::Failed),
        }
        text.push_str("  Arbitrary process inspection: not collected\n  Daemon log: not collected (failed node calls are a separate bounded summary)");
        if text.len() > MAX_REPORT_BYTES {
            return Err(Problem::TooLarge);
        }
    }
    ReportText::new(&text)
}
