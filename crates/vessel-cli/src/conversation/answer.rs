//! `vessel session answer`: answer what a session's agent waits on, from
//! the command line: a question (protocol 10) with a value for each field,
//! or a permission request with an option. The requests waiting are read
//! first (`session.conversation`), so a prefix names one of them and a
//! question's own form says how to read each value.

use vessel_client::NodeClient;
use vessel_client::compose;
use vessel_proto::{
    PermissionOptionId, PermissionOutcome, PermissionRequest, Question, QuestionAnswer, SessionId,
    SessionInput,
};

use crate::error::CliError;
use crate::selector::IdPrefix;

/// What a request is called in an error.
const REQUEST: &str = "waiting question or permission request";

/// Something the agent waits on.
#[derive(Debug, Clone, Copy)]
enum Waiting<'a> {
    Question(&'a Question),
    Permission(&'a PermissionRequest),
}

impl Waiting<'_> {
    fn id(&self) -> String {
        match self {
            Self::Question(question) => question.id.to_string(),
            Self::Permission(request) => request.id.to_string(),
        }
    }
}

/// Answer the request `request` names in `session`: a question with
/// `field=value` pairs, a permission with one option's id, either declined
/// with `decline`.
pub(super) async fn answer(
    client: &NodeClient,
    session: SessionId,
    request: &IdPrefix,
    values: &[String],
    decline: bool,
) -> Result<(), CliError> {
    let conversation = client.conversation(session).await?;
    let requests: Vec<Waiting<'_>> = conversation
        .questions
        .iter()
        .map(Waiting::Question)
        .chain(conversation.permissions.iter().map(Waiting::Permission))
        .collect();
    match request.pick(&requests, Waiting::id, REQUEST)? {
        Waiting::Question(question) => {
            let answer = if decline {
                QuestionAnswer::Decline
            } else {
                compose::fill(question, pairs(values)?)?
            };
            Ok(client.answer_question(session, question.id, answer).await?)
        }
        Waiting::Permission(permission) => {
            let outcome = match (decline, values) {
                (true, _) => PermissionOutcome::Cancelled,
                (false, [option]) => PermissionOutcome::Selected {
                    option: PermissionOptionId::new(option.clone()),
                },
                (false, _) => {
                    return Err(CliError::Usage(
                        "a permission request takes one option's id, or --decline",
                    ));
                }
            };
            let input = SessionInput::AnswerPermission {
                id: permission.id,
                outcome,
            };
            Ok(client.send(session, input).await?)
        }
    }
}

/// `field=value` texts as pairs; a value may hold `=` itself.
fn pairs(values: &[String]) -> Result<Vec<(&str, &str)>, CliError> {
    values
        .iter()
        .map(|value| {
            value
                .split_once('=')
                .ok_or(CliError::Usage("give each field's value as `field=value`"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_value_is_split_at_its_first_equals_sign() {
        let given = ["way=left".to_owned(), "note=a=b".to_owned()];
        assert_eq!(pairs(&given).unwrap(), [("way", "left"), ("note", "a=b")]);
        assert!(matches!(
            pairs(&["left".to_owned()]),
            Err(CliError::Usage(_))
        ));
    }
}
