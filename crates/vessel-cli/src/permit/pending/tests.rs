//! Canonical row identity and shared presentation never depend on an Inbox row.
use super::*;

#[test]
fn permit_rows_use_the_reviewed_formatter_with_correct_identity_fidelity() {
    use crate::args::parity::{Presentation, output::Headers};
    use crate::parity::output::render_list;
    let session = SessionId::random();
    let id = PendingId::Question(QuestionId::random());
    let row = Row {
        id,
        agent_id: session,
        agent_short_id: session.to_string().chars().take(AGENT_PREFIX).collect(),
        name: "Question".into(),
        description: "Line one\nLine two".into(),
    };
    let table =
        String::from_utf8(render_list(Presentation::Table(Headers::Show), &[row.clone()]).unwrap())
            .unwrap();
    assert!(table.contains("AGENT") && table.contains("REQ_ID") && table.contains("DESCRIPTION"));
    assert!(table.contains(&row.agent_short_id) && table.contains(&row.short_request()));
    assert!(!table.contains(&session.to_string()) && !table.contains(&id.to_string()));
    assert!(
        table.contains("Line one\\nLine two"),
        "shared escaping is applied exactly once"
    );
    let quiet =
        String::from_utf8(render_list(Presentation::Quiet, &[row.clone()]).unwrap()).unwrap();
    assert_eq!(quiet.trim_end(), id.to_string());
    for mode in [Presentation::Json, Presentation::Yaml] {
        let structured = String::from_utf8(render_list(mode, &[row.clone()]).unwrap()).unwrap();
        assert!(structured.contains(&session.to_string()));
        assert!(structured.contains(&id.to_string()));
        assert!(!structured.contains("source") && !structured.contains("seq"));
    }
}
