//! Copy resume command (TRACKER 797): Paseo's client table, its id notice
//! before its template notice.

use crate::session::{ResumeUnavailable, resume_command};
use vessel_proto::{AgentName, HarnessKind, HarnessRef, HarnessSessionId};

#[test]
fn builds_the_resume_command_of_each_cli_as_paseo() {
    let id = HarnessSessionId::new("4f1c2a");
    let known = |kind| resume_command(Some(&HarnessRef::Known(kind)), Some(&id));
    assert_eq!(
        known(HarnessKind::ClaudeCode).as_deref(),
        Ok("claude --resume 4f1c2a")
    );
    assert_eq!(
        known(HarnessKind::Codex).as_deref(),
        Ok("codex resume 4f1c2a")
    );
    assert_eq!(
        known(HarnessKind::OpenCode).as_deref(),
        Ok("opencode --session 4f1c2a")
    );
    // Paseo's table has no Gemini entry.
    assert_eq!(
        known(HarnessKind::Gemini),
        Err(ResumeUnavailable::NoCommand)
    );
}

#[test]
fn says_the_id_is_missing_before_the_command() {
    let added: AgentName = "pi".parse().unwrap();
    let id = HarnessSessionId::new("4f1c2a");
    // No provider id: the id notice, whatever the agent.
    assert_eq!(
        resume_command(Some(&HarnessRef::Known(HarnessKind::ClaudeCode)), None),
        Err(ResumeUnavailable::NoId)
    );
    assert_eq!(
        resume_command(Some(&HarnessRef::Added(added.clone())), None),
        Err(ResumeUnavailable::NoId)
    );
    // An added agent, or none known: no template.
    assert_eq!(
        resume_command(Some(&HarnessRef::Added(added)), Some(&id)),
        Err(ResumeUnavailable::NoCommand)
    );
    assert_eq!(
        resume_command(None, Some(&id)),
        Err(ResumeUnavailable::NoCommand)
    );
}
