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
//!
//! `ask questions` and `ask one` send what Claude's adapter sends for its
//! `AskUserQuestion` tool (`claude-agent-acp`'s
//! `askUserQuestionsToCreateRequest`), so screens are checked against the
//! real shape: per question a choice `question_N` titled by its header
//! (described by its text when there are several; one of them chooses
//! several) and a free-text `question_N_custom` titled "Other", nothing
//! required; the message is the lone question's text, else "Please answer
//! the following questions.". `ask questions` asks three, one in Korean;
//! `ask one` asks one.

#[path = "ask/large.rs"]
mod large;

use agent_client_protocol::v1::schema::{
    BooleanPropertySchema, ContentChunk, CreateElicitationRequest, CreateElicitationResponse,
    ElicitationAction, ElicitationFormMode, ElicitationSchema, ElicitationSessionScope,
    ElicitationUrlMode, EnumOption, MultiSelectPropertySchema, SessionId, SessionNotification,
    SessionUpdate, StopReason, StringPropertySchema,
};
use agent_client_protocol::{Client, Error};
use tokio_util::sync::CancellationToken;

/// A question prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ask {
    /// `ask large`: a bounded valid form that exceeds inline inbox budgets.
    Large,
    /// `ask`: a form, answered.
    Form,
    /// `ask withdraw`: a form, withdrawn at once.
    Withdraw,
    /// `ask link`: a link to open.
    Link,
    /// `ask questions`: Claude's `AskUserQuestion` with three questions.
    Questions,
    /// `ask one`: Claude's `AskUserQuestion` with one question.
    One,
}

/// The question prompt `prompt` is, if it is one.
pub(crate) fn wanted(prompt: &str) -> Option<Ask> {
    match prompt {
        "ask large" => Some(Ask::Large),
        "ask" => Some(Ask::Form),
        "ask withdraw" => Some(Ask::Withdraw),
        "ask link" => Some(Ask::Link),
        "ask questions" => Some(Ask::Questions),
        "ask one" => Some(Ask::One),
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
    if ask == Ask::Large {
        return large::turn(cx, session).await;
    }
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
    if matches!(ask, Ask::Questions | Ask::One) {
        let (message, schema) = claude_questions(ask == Ask::One);
        let request =
            CreateElicitationRequest::new(ElicitationFormMode::new(scope, schema), message);
        say(&answered(cx.send_request(request).block_task().await))?;
        return Ok(if turn.is_cancelled() {
            StopReason::Cancelled
        } else {
            StopReason::EndTurn
        });
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

/// One of Claude's questions: its header, its text, its options (label
/// and description), whether several may be chosen.
type ClaudeQuestion = (
    &'static str,
    &'static str,
    &'static [(&'static str, &'static str)],
    bool,
);

/// Three questions as a real agent asks them: long option descriptions,
/// one choosing several, one in Korean.
const QUESTIONS: [ClaudeQuestion; 3] = [
    (
        "Approach",
        "Which approach should the fold take when a batch ends inside a message?",
        &[
            (
                "Extend the last entry",
                "Keep the entry open across batches and append the next chunk to it, so a reply that arrives in pieces stays one entry",
            ),
            (
                "Start a new entry",
                "Close the entry at the batch's end; the next chunk opens another, and the screen joins them",
            ),
            (
                "Buffer the batch",
                "Hold the batch's last chunk until the next record arrives",
            ),
        ],
        false,
    ),
    (
        "Checks",
        "Which checks should run before this merges?",
        &[
            ("Unit tests", "The view's own tests"),
            (
                "The page's replayed cases",
                "Every case the page's tests hold, run against the view",
            ),
            (
                "Clippy with every lint",
                "The workspace's lints, warnings as errors",
            ),
            (
                "A 20,000-entry fold",
                "Folding a long session in under ten seconds",
            ),
        ],
        true,
    ),
    (
        "언어",
        "답을 어느 언어로 쓸까요?",
        &[
            ("한국어", "모든 답과 요약을 한국어로 씁니다"),
            ("English", "Every answer and summary in English"),
        ],
        false,
    ),
];

/// Claude's adapter's form for `AskUserQuestion`: the message and the
/// schema, for one question or all three.
fn claude_questions(one: bool) -> (&'static str, ElicitationSchema) {
    let asked: &[ClaudeQuestion] = if one {
        QUESTIONS.get(..1).unwrap_or_default()
    } else {
        &QUESTIONS
    };
    let mut schema = ElicitationSchema::new();
    for (index, (header, question, options, multiple)) in asked.iter().enumerate() {
        let options: Vec<EnumOption> = options
            .iter()
            .map(|(label, description)| EnumOption::new(*label, *label).description(*description))
            .collect();
        // A lone question's text is the message, not the field's.
        let description = (!one).then(|| (*question).to_owned());
        let key = format!("question_{index}");
        schema = if *multiple {
            schema.property(
                &key,
                MultiSelectPropertySchema::titled(options)
                    .title(*header)
                    .description(description),
                false,
            )
        } else {
            schema.property(
                &key,
                StringPropertySchema::new()
                    .title(*header)
                    .description(description)
                    .one_of(options),
                false,
            )
        };
        let other = if *multiple {
            "Type your own answer to add to your selection above (optional)."
        } else {
            "Type your own answer, or add a note to the option you chose above (optional)."
        };
        schema = schema.property(
            format!("{key}_custom"),
            StringPropertySchema::new()
                .title("Other")
                .description(other),
            false,
        );
    }
    let message = match asked {
        [(_, question, _, _)] => *question,
        _ => "Please answer the following questions.",
    };
    (message, schema)
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
