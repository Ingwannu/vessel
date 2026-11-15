//! Answer exactly once, then await durable session events before confirming
//! projection withdrawal. No sleeps, guessed delay or automatic resend.
use super::{
    Input,
    node::{get, problem, wire_key},
};
use vessel_app_view::inbox::{Entry, Key, Problem};
use vessel_client::NodeClient;
use vessel_proto::{PermissionOutcome, ReadEvents, SessionInput};

/// Long polls wait for real session events rather than spinning on the inbox.
const CONFIRM_WAIT_SECONDS: u32 = 10;
/// One returned event page is bounded; the outer request deadline bounds the
/// total stream while another turn is emitting unrelated output.
const CONFIRM_PAGE_RECORDS: u32 = 32;

pub(super) async fn send(
    client: &NodeClient,
    key: Key,
    input: Input,
) -> Result<Option<Entry>, Problem> {
    let Some(entry) = get(client, key).await? else {
        return Ok(None);
    };
    input.validate(&entry)?;
    let from = client.session(key.session).await.map_err(problem)?.next_seq;
    let input = match input {
        Input::Permission(id, option) => SessionInput::AnswerPermission {
            id,
            outcome: PermissionOutcome::Selected { option },
        },
        Input::Question(id, answer) => SessionInput::AnswerQuestion { id, answer },
    };
    client.send(key.session, input).await.map_err(problem)?;
    confirmed(client, key, from).await
}

async fn confirmed(
    client: &NodeClient,
    key: Key,
    mut from: vessel_proto::EventSeq,
) -> Result<Option<Entry>, Problem> {
    loop {
        if client
            .inbox_entry(wire_key(key))
            .await
            .map_err(problem)?
            .is_none()
        {
            return Ok(None);
        }
        // session.read waits on committed data. Inbox projection and event
        // append share the same transaction, so seeing a resolution record
        // cannot race ahead of the corresponding inbox withdrawal.
        let page = client
            .call(ReadEvents {
                session: key.session,
                from,
                limit: CONFIRM_PAGE_RECORDS,
                wait: Some(CONFIRM_WAIT_SECONDS),
            })
            .await
            .map_err(problem)?;
        if page.next <= from {
            return Err(Problem::Refused(vessel_proto::ErrorCode::Unavailable));
        }
        from = page.next;
    }
}
