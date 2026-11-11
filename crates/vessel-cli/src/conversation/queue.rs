//! `vessel session queue`: a structured session's prompts waiting their
//! turn (protocol 10), shown, or changed as `session.queue` changes them.
//! An entry is named by its id or a unique prefix of it, among the entries
//! there are when the command reads the queue.

use vessel_client::NodeClient;
use vessel_client::compose;
use vessel_proto::{PromptQueue, QueueChange, QueuedPrompt, SessionId};

use crate::args::QueueCommand;
use crate::error::CliError;
use crate::selector::IdPrefix;
use crate::{io, render};

/// What an entry is called in an error.
const ENTRY: &str = "waiting prompt";

/// Show `session`'s queue, after making `change` if there is one; for an
/// entry sent now or a queue let move, say what became of the prompt.
pub(super) async fn queue(
    client: &NodeClient,
    session: SessionId,
    change: Option<QueueCommand>,
) -> Result<(), CliError> {
    let Some(command) = change else {
        let waiting = client.conversation(session).await?.queue;
        return Ok(io::out(render::queue_entries(&waiting).as_bytes()).await?);
    };
    let change = self::change(client, session, command).await?;
    let changed = client.change_queue(session, change).await?;
    let mut text = render::queue_entries(&changed.queue);
    if let Some(delivered) = &changed.delivered {
        text.push_str(&render::delivered(delivered));
        text.push('\n');
    }
    Ok(io::out(text.as_bytes()).await?)
}

/// The change `command` asks for, its entries found in the queue as it
/// is now.
async fn change(
    client: &NodeClient,
    session: SessionId,
    command: QueueCommand,
) -> Result<QueueChange, CliError> {
    Ok(match command {
        QueueCommand::Hold => QueueChange::Hold,
        QueueCommand::Resume => QueueChange::Resume,
        QueueCommand::Clear => QueueChange::Clear,
        QueueCommand::Edit {
            entry,
            text,
            revision,
        } => {
            let waiting = waiting(client, session).await?;
            let found = find(&waiting, &entry)?;
            QueueChange::Edit {
                entry: found.id,
                revision: revision.unwrap_or(found.revision),
                content: compose::with_text(&found.content, text),
            }
        }
        QueueCommand::Move { entry, before } => {
            let waiting = waiting(client, session).await?;
            QueueChange::Move {
                entry: find(&waiting, &entry)?.id,
                before: before
                    .map(|before| find(&waiting, &before).map(|found| found.id))
                    .transpose()?,
            }
        }
        QueueCommand::Remove { entry } => QueueChange::Remove {
            entry: find(&waiting(client, session).await?, &entry)?.id,
        },
        QueueCommand::SendNow { entry, how } => QueueChange::SendNow {
            entry: find(&waiting(client, session).await?, &entry)?.id,
            delivery: how.delivery(),
        },
    })
}

async fn waiting(client: &NodeClient, session: SessionId) -> Result<PromptQueue, CliError> {
    Ok(client.conversation(session).await?.queue)
}

fn find<'a>(waiting: &'a PromptQueue, entry: &IdPrefix) -> Result<&'a QueuedPrompt, CliError> {
    entry.pick(&waiting.entries, |queued| queued.id, ENTRY)
}
