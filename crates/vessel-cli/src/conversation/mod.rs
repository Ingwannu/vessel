//! The conversation's commands (protocol 10, milestone 11's Track A):
//! `vessel send` with a delivery and files, and `vessel session queue`,
//! `answer`, `resume`, `pin`, `unpin`, `archive`, `unarchive`, `read`,
//! `unread` and `delete`. Each resolves the session, makes its calls and
//! prints what the node answered (formatting is `render`'s); a refusal is
//! the node's typed error, printed as every command prints one.

mod answer;
mod queue;
mod send;

use std::io::IsTerminal as _;

use vessel_client::NodeClient;
use vessel_proto::{Mark, SessionInfo};

pub(crate) use send::send;

use crate::args::{ConversationCommand, Target};
use crate::connect::connect;
use crate::error::CliError;
use crate::selector::SessionSelector;
use crate::{io, render};

/// `vessel session <conversation command>`.
pub(crate) async fn command(target: &Target, command: ConversationCommand) -> Result<(), CliError> {
    let client = connect(target).await?.client;
    match command {
        ConversationCommand::Queue { session, change } => {
            let id = session.resolve(&client).await?.id;
            queue::queue(&client, id, change).await
        }
        ConversationCommand::Answer {
            session,
            request,
            values,
            decline,
        } => {
            let id = session.resolve(&client).await?.id;
            answer::answer(&client, id, &request, &values, decline).await
        }
        ConversationCommand::Resume { session } => {
            let id = session.resolve(&client).await?.id;
            show(&client.resume_session(id).await?).await
        }
        ConversationCommand::Pin { session } => mark(&client, &session, |_| Mark::Pin).await,
        ConversationCommand::Unpin { session } => mark(&client, &session, |_| Mark::Unpin).await,
        ConversationCommand::Archive { session } => {
            mark(&client, &session, |_| Mark::Archive).await
        }
        ConversationCommand::Unarchive { session } => {
            mark(&client, &session, |_| Mark::Unarchive).await
        }
        // Seen is what the session had logged when it was looked up: a
        // record landing after that stays unread.
        ConversationCommand::Read { session } => {
            mark(&client, &session, |info| Mark::Read {
                through: info.next_seq,
            })
            .await
        }
        ConversationCommand::Unread { session } => mark(&client, &session, |_| Mark::Unread).await,
        ConversationCommand::Delete { session, yes } => delete(&client, &session, yes).await,
    }
}

/// Set the mark `mark` makes of the session's info; show the session
/// after.
async fn mark(
    client: &NodeClient,
    session: &SessionSelector,
    mark: impl FnOnce(&SessionInfo) -> Mark,
) -> Result<(), CliError> {
    let info = session.resolve(client).await?;
    show(&client.mark(info.id, mark(&info)).await?).await
}

/// The session on one line, with its marks.
async fn show(info: &SessionInfo) -> Result<(), CliError> {
    let line = render::sessions(std::slice::from_ref(info));
    Ok(io::out(line.as_bytes()).await?)
}

/// `vessel session delete`: after a confirmation unless `yes`, as `vessel
/// node update --now` asks before it interrupts sessions.
async fn delete(client: &NodeClient, session: &SessionSelector, yes: bool) -> Result<(), CliError> {
    let info = session.resolve(client).await?;
    if !yes && !confirm(&info).await? {
        return Err(CliError::Usage("not confirmed; nothing was deleted"));
    }
    Ok(client.delete_session(info.id).await?)
}

/// Ask before deleting `info` for good; only a person at a terminal can
/// answer.
async fn confirm(info: &SessionInfo) -> Result<bool, CliError> {
    if !std::io::stdin().is_terminal() {
        return Err(CliError::Usage(
            "deleting a session cannot be undone; confirm with --yes when not at a terminal",
        ));
    }
    io::note(&format!(
        "delete session {} ({}) with its log and attachments, for good? [y/N] ",
        info.id,
        render::status(&info.status)
    ))
    .await?;
    let mut lines = io::input_lines()?;
    let answer = lines.recv().await.unwrap_or_default();
    Ok(matches!(answer.trim(), "y" | "Y" | "yes"))
}
