//! `fake-acp`'s questions (protocol 10): the prompts `ask` and `ask
//! withdraw`, which send an `elicitation/create` in form mode when the
//! client advertised form elicitation in `initialize`.
//!
//! The form, `Which way?`: `name` (text, default `vessel`), `way` (a
//! required single choice, `left` or `right`, titled) and `sure` (a
//! boolean). After the client's answer, one message chunk `answered:
//! <action>`, followed by the accepted content as JSON (`answered: accept
//! {"name":"vessel","sure":true,"way":"left"}`), or `answered: error
//! <message>` when the client refused the request. `ask withdraw` cancels
//! its own request right after sending it (`$/cancel_request`, the SDK's
//! request cancellation), then reports the answer the client still owes
//! (`answered: cancel`). Without the capability, one message chunk `no
//! questions`. `ask link` asks for a link to be opened (URL mode, which a
//! client advertises apart), whatever the client advertised, and reports
//! the answer the same way. The turn ends `cancelled` if it was cancelled
//! meanwhile, `end_turn` otherwise.

use agent_client_protocol::v1::schema::{
    BooleanPropertySchema, ContentChunk, CreateElicitationRequest, CreateElicitationResponse,
    ElicitationAction, ElicitationFormMode, ElicitationSchema, ElicitationSessionScope,
    ElicitationUrlMode, EnumOption, SessionId, SessionNotification, SessionUpdate, StopReason,
    StringPropertySchema,
};
use agent_client_protocol::{Client, Error};
use tokio_util::sync::CancellationToken;

/// A question prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ask {
    /// `ask`: a form, answered.
    Form,
    /// `ask withdraw`: a form, withdrawn at once.
    Withdraw,
    /// `ask link`: a link to open.
    Link,
}

/// The question prompt `prompt` is, if it is one.
pub(crate) fn wanted(prompt: &str) -> Option<Ask> {
    match prompt {
        "ask" => Some(Ask::Form),
        "ask withdraw" => Some(Ask::Withdraw),
        "ask link" => Some(Ask::Link),
        _ => None,
    }
}

/// Run a question prompt's turn. `form` is whether the client advertised
/// form elicitation.
pub(crate) async fn turn(
    cx: &agent_client_protocol::v1::ConnectionTo<Client>,
    session: &SessionId,
    form: bool,
    ask: Ask,
    turn: &CancellationToken,
) -> Result<StopReason, Error> {
    let say = |text: &str| {
        cx.send_notification(SessionNotification::new(
            session.clone(),
            SessionUpdate::AgentMessageChunk(ContentChunk::new(super::text(text))),
        ))
    };
    let scope = ElicitationSessionScope::new(session.clone());
    if ask == Ask::Link {
        let link = ElicitationUrlMode::new(scope, "link-1", "https://example.com/sign-in");
        let sent = cx.send_request(CreateElicitationRequest::new(link, "Sign in"));
        say(&answered(sent.block_task().await))?;
        return Ok(StopReason::EndTurn);
    }
    if !form {
        say("no questions")?;
        return Ok(StopReason::EndTurn);
    }
    let schema = ElicitationSchema::new()
        .property(
            "name",
            StringPropertySchema::new()
                .title("Name")
                .default_value("vessel"),
            false,
        )
        .property(
            "way",
            StringPropertySchema::new().title("Which way").one_of(vec![
                EnumOption::new("left", "Left"),
                EnumOption::new("right", "Right"),
            ]),
            true,
        )
        .property("sure", BooleanPropertySchema::new().title("Sure"), false);
    let request =
        CreateElicitationRequest::new(ElicitationFormMode::new(scope, schema), "Which way?");
    let sent = cx.send_request(request);
    if ask == Ask::Withdraw {
        sent.cancel()?;
    }
    say(&answered(sent.block_task().await))?;
    Ok(if turn.is_cancelled() {
        StopReason::Cancelled
    } else {
        StopReason::EndTurn
    })
}

/// The line reporting the client's answer.
fn answered(answer: Result<CreateElicitationResponse, Error>) -> String {
    match answer {
        Ok(response) => match response.action {
            ElicitationAction::Accept(accepted) => format!(
                "answered: accept {}",
                serde_json::to_string(&accepted.content).unwrap_or_default()
            ),
            ElicitationAction::Decline => "answered: decline".to_owned(),
            ElicitationAction::Cancel => "answered: cancel".to_owned(),
            ElicitationAction::Other(_) | _ => "answered: other".to_owned(),
        },
        Err(error) => format!("answered: error {}", error.message),
    }
}
