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

#[test]
fn quotes_an_id_a_shell_would_read_as_more_than_a_word() {
    let claude = HarnessRef::Known(HarnessKind::ClaudeCode);
    // Ordinary ids go in bare, exactly as Paseo copies them.
    let plain = HarnessSessionId::new("0f3a-9b:x_y.z");
    assert_eq!(
        resume_command(Some(&claude), Some(&plain)).as_deref(),
        Ok("claude --resume 0f3a-9b:x_y.z")
    );
    // A space, a quote and a command substitution: one quoted word that
    // runs nothing on paste.
    let odd = HarnessSessionId::new("a b'c$(touch x)");
    assert_eq!(
        resume_command(Some(&claude), Some(&odd)).as_deref(),
        Ok(r"claude --resume 'a b'\''c$(touch x)'")
    );
    let empty = HarnessSessionId::new("");
    assert_eq!(
        resume_command(Some(&claude), Some(&empty)).as_deref(),
        Ok("claude --resume ''")
    );
}
