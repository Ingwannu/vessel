//! CLI inbox: page the projection, then use the existing form/answer paths.
//!
//! A selected `show` recovers one compact form after retention. Expanding
//! every list page would fan out large bodies and obscure the bounded list;
//! explicit inspection keeps the single-form response budget meaningful.
use crate::args::{InboxCommand, Target};
use crate::connect::connect;
use crate::conversation::answer::{Waiting, reply};
use crate::error::CliError;
use vessel_client::NodeClient;
use vessel_proto::inbox::{InboxCursor, InboxEntry, InboxItem, InboxRead, ListInbox};

pub(crate) async fn command(target: &Target, command: InboxCommand) -> Result<(), CliError> {
    let client = connect(target).await?.client;
    match command {
        InboxCommand::List => {
            let node = client.node_info().await?;
            crate::io::out(format!("Machine: {}\n", node.name).as_bytes()).await?;
            let mut page = client.inbox(ListInbox::default()).await?;
            loop {
                for entry in page.entries {
                    crate::io::out(crate::render::inbox_entry(&entry).as_bytes()).await?;
                }
                let Some(before) = page.next else {
                    break;
                };
                page = client
                    .inbox(ListInbox {
                        before: Some(before),
                    })
                    .await?;
            }
            Ok(())
        }
        InboxCommand::Show { entry } => {
            let entry = current_entry(&client, entry).await?;
            if matches!(entry.item, InboxItem::InSession { .. }) {
                return Err(CliError::Usage(
                    "the full request cannot fit in one reply; open its session or use --decline",
                ));
            }
            let node = client.node_info().await?;
            crate::io::out(format!("Machine: {}\n", node.name).as_bytes()).await?;
            crate::io::out(crate::render::inbox_entry(&entry).as_bytes()).await?;
            Ok(())
        }
        InboxCommand::Read { entry } => mark(&client, entry, InboxRead::Read).await,
        InboxCommand::Unread { entry } => mark(&client, entry, InboxRead::Unread).await,
        InboxCommand::Answer {
            entry,
            values,
            decline,
        } => {
            let entry = current_entry(&client, entry).await?;
            match &entry.item {
                InboxItem::Permission { request, .. } => {
                    reply(
                        &client,
                        entry.key.session,
                        Waiting::Permission(request),
                        &values,
                        decline,
                    )
                    .await
                }
                InboxItem::Question { question } => {
                    reply(
                        &client,
                        entry.key.session,
                        Waiting::Question(question),
                        &values,
                        decline,
                    )
                    .await
                }
                InboxItem::InSession { request } if decline => {
                    decline_compact(&client, entry.key.session, *request).await
                }
                InboxItem::InSession { .. } => Err(CliError::Usage(
                    "the full request cannot fit in one reply; open its session or use --decline",
                )),
                InboxItem::Finished | InboxItem::Failed | InboxItem::Limit | InboxItem::Unknown => {
                    Err(CliError::Usage(
                        "this entry does not ask for an answer; open its session instead",
                    ))
                }
            }
        }
    }
}
async fn current_entry(client: &NodeClient, cursor: InboxCursor) -> Result<InboxEntry, CliError> {
    let entry = find(client, cursor).await?;
    client
        .inbox_request(entry.key)
        .await?
        .ok_or(CliError::Usage(
            "this inbox entry has moved on; run `vessel inbox list` again",
        ))
}

async fn find(client: &NodeClient, cursor: InboxCursor) -> Result<InboxEntry, CliError> {
    let mut page = client.inbox(ListInbox::default()).await?;
    loop {
        if let Some(entry) = page
            .entries
            .into_iter()
            .find(|entry| entry.cursor == cursor)
        {
            return Ok(entry);
        }
        let Some(before) = page.next else {
            return Err(CliError::Usage(
                "this inbox entry has moved on; run `vessel inbox list` again",
            ));
        };
        page = client
            .inbox(ListInbox {
                before: Some(before),
            })
            .await?;
    }
}
async fn mark(client: &NodeClient, cursor: InboxCursor, read: InboxRead) -> Result<(), CliError> {
    let entry = find(client, cursor).await?;
    Ok(client.mark_inbox(entry.key, read).await?)
}

async fn decline_compact(
    client: &NodeClient,
    session: vessel_proto::SessionId,
    request: vessel_proto::inbox::InboxRequest,
) -> Result<(), CliError> {
    match request {
        vessel_proto::inbox::InboxRequest::Permission { id } => Ok(client
            .send(
                session,
                vessel_proto::SessionInput::AnswerPermission {
                    id,
                    outcome: vessel_proto::PermissionOutcome::Cancelled,
                },
            )
            .await?),
        vessel_proto::inbox::InboxRequest::Question { id } => Ok(client
            .answer_question(session, id, vessel_proto::QuestionAnswer::Decline)
            .await?),
    }
}
