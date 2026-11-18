//! `vessel history` (protocol 17, ADR 0053): one page of an agent's own
//! conversations, one line each, then how to go on and how to import one.

use std::fmt::Write as _;

use vessel_proto::HarnessRef;
use vessel_proto::history::{AgentHistoryPage, GetAgentHistory, HistoryEntry};

use crate::args::{History, Target};
use crate::commands::node_cwd;
use crate::connect::connect_to;
use crate::error::CliError;
use crate::{io, nodes};

/// `vessel history <agent>`.
pub(crate) async fn history(target: &Target, history: History) -> Result<(), CliError> {
    let (resolved, book) = nodes::resolve(target)?;
    let cwd = history
        .cwd
        .map(|cwd| node_cwd(&resolved, cwd))
        .transpose()?;
    let client = connect_to(target, &resolved, &book).await?.client;
    let page = client
        .call(GetAgentHistory {
            harness: history.harness.clone(),
            profile: history.profile.clone(),
            cwd,
            cursor: history.more,
            limit: None,
            query: None,
        })
        .await?;
    let import = Import {
        harness: &history.harness,
        profile: history.profile.as_ref().map(ToString::to_string),
    };
    Ok(io::out(text(&page, &import).as_bytes()).await?)
}

/// What `vessel run` needs besides the entry to import it.
struct Import<'a> {
    harness: &'a HarnessRef,
    profile: Option<String>,
}

/// The page as lines: the agent's id, when it was last active, the folder,
/// the title, and the session already on it; then how to go on and how
/// to import one.
fn text(page: &AgentHistoryPage, import: &Import<'_>) -> String {
    let mut text = String::new();
    if page.conversations.is_empty() {
        text.push_str("no conversations");
        text.push_str(if page.next.is_some() {
            " on this page\n"
        } else {
            "\n"
        });
    }
    for entry in &page.conversations {
        line(&mut text, entry);
    }
    if let Some(next) = &page.next {
        let _written = writeln!(text, "more: --more {next}");
    }
    if !page.conversations.is_empty() {
        let profile = import
            .profile
            .as_ref()
            .map(|profile| format!(" --profile {profile}"))
            .unwrap_or_default();
        let _written = writeln!(
            text,
            "import: vessel run {} --structured{profile} --resume ID --cwd FOLDER [--title TITLE]",
            import.harness
        );
    }
    text
}

fn line(text: &mut String, entry: &HistoryEntry) {
    let _written = write!(
        text,
        "{} {} {}",
        entry.harness_session,
        entry.updated_at.map_or_else(
            || "-".to_owned(),
            |at| at.format("%Y-%m-%d %H:%M").to_string()
        ),
        entry.cwd.display(),
    );
    if let Some(title) = &entry.title {
        let _written = write!(text, " \"{title}\"");
    }
    if let Some(session) = entry.in_vessel {
        let session = session.to_string();
        let _written = write!(
            text,
            " (in vessel {})",
            session.get(..8).unwrap_or(&session)
        );
    }
    text.push('\n');
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::DateTime;
    use vessel_proto::{HarnessKind, HarnessSessionId, SessionId};

    use super::*;

    #[test]
    fn a_conversation_is_one_line_and_the_page_says_how_to_import() {
        let session: SessionId = "0b5a2c1e-0000-4000-8000-000000000000".parse().unwrap();
        let page = AgentHistoryPage {
            conversations: vec![
                HistoryEntry {
                    harness_session: HarnessSessionId::new("past-3"),
                    cwd: PathBuf::from("/work/app"),
                    title: Some("third".parse().unwrap()),
                    updated_at: DateTime::from_timestamp(1_700_000_000, 0),
                    in_vessel: Some(session),
                    first_prompt: None,
                    last_prompt: None,
                },
                HistoryEntry {
                    harness_session: HarnessSessionId::new("past-1"),
                    cwd: PathBuf::from("/work/app"),
                    title: None,
                    updated_at: None,
                    in_vessel: None,
                    first_prompt: None,
                    last_prompt: None,
                },
            ],
            next: Some("2~".to_owned()),
            already_imported: None,
        };
        let harness = HarnessRef::Known(HarnessKind::ClaudeCode);
        let import = Import {
            harness: &harness,
            profile: Some("work".to_owned()),
        };
        assert_eq!(
            text(&page, &import),
            "past-3 2023-11-14 22:13 /work/app \"third\" (in vessel 0b5a2c1e)\n\
             past-1 - /work/app\n\
             more: --more 2~\n\
             import: vessel run claude-code --structured --profile work --resume ID --cwd FOLDER \
             [--title TITLE]\n"
        );
        let empty = AgentHistoryPage {
            conversations: Vec::new(),
            next: None,
            already_imported: None,
        };
        assert_eq!(text(&empty, &import), "no conversations\n");
    }
}
