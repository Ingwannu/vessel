//! The command that resumes a session in its agent's own CLI, for the tab
//! menu's "Copy resume command" (TRACKER 797).
//!
//! [Decision Log]
//! - Purpose: Paseo's agent tab menu copies the command that reopens the
//!   conversation outside Paseo, in the provider's CLI, with the provider's
//!   own session id (`handleCopyResumeCommand`, workspace-screen.tsx).
//! - Reference: Paseo's client table (`utils/provider-command-templates.ts`):
//!   `claude --resume {sessionId}`, `codex resume {sessionId}`,
//!   `opencode --session {sessionId}`; no entry for Gemini. With no provider
//!   id it says "Resume ID not available", with no template "Resume command
//!   not available", in that order.
//! - Alternatives: ask the node for the argv its harness module builds (the
//!   node knows each CLI, but has no such call, and the coordinator ruled the
//!   client templates, as Paseo's, 2026-11-18).
//! - Choice: the client table, keyed by Vessel's four harnesses; an added
//!   agent has no template.
//! - Cost: a CLI that renames its resume flag needs this table changed, as
//!   Paseo's does.
//! - The id is the node's opaque word, pasted into a shell. Paseo puts it
//!   in bare; an id holding a space, a quote or `$( )` would then run on
//!   paste (app review of TRACKER 797, ruled into 800). So an id of only
//!   `[A-Za-z0-9._:-]` goes in bare, copied exactly as Paseo's, and any
//!   other in POSIX single quotes, each `'` written `'\''`.

use std::borrow::Cow;

use vessel_proto::{HarnessKind, HarnessRef, HarnessSessionId};

/// Why there is no resume command to copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResumeUnavailable {
    /// The agent has not named its own session yet (Paseo's
    /// `resumeIdUnavailable`).
    NoId,
    /// Its CLI has no resume command in the table (Paseo's
    /// `resumeCommandUnavailable`).
    NoCommand,
}

/// The command that resumes `session` in `harness`'s own CLI.
///
/// # Errors
///
/// [`ResumeUnavailable::NoId`] without the harness's session id, then
/// [`ResumeUnavailable::NoCommand`] for a harness with no template.
pub fn resume_command(
    harness: Option<&HarnessRef>,
    session: Option<&HarnessSessionId>,
) -> Result<String, ResumeUnavailable> {
    let session = session.ok_or(ResumeUnavailable::NoId)?;
    let command = match harness {
        Some(HarnessRef::Known(HarnessKind::ClaudeCode)) => "claude --resume",
        Some(HarnessRef::Known(HarnessKind::Codex)) => "codex resume",
        Some(HarnessRef::Known(HarnessKind::OpenCode)) => "opencode --session",
        Some(HarnessRef::Known(HarnessKind::Gemini) | HarnessRef::Added(_)) | None => {
            return Err(ResumeUnavailable::NoCommand);
        }
    };
    Ok(format!("{command} {}", shell_word(session.as_str())))
}

/// `id` as one POSIX shell word: bare when it holds only
/// `[A-Za-z0-9._:-]`, else single-quoted.
fn shell_word(id: &str) -> Cow<'_, str> {
    let plain = !id.is_empty()
        && id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || ".:_-".contains(character));
    if plain {
        Cow::Borrowed(id)
    } else {
        Cow::Owned(format!("'{}'", id.replace('\'', r"'\''")))
    }
}
