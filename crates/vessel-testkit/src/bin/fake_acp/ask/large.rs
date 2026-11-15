//! A valid bounded form larger than the inbox's inline payload budget.
//! Later tool records evict snapshots and prune its source before an answer.
use agent_client_protocol::v1::schema::{
    ContentChunk, CreateElicitationRequest, ElicitationFormMode, ElicitationSchema,
    ElicitationSessionScope, EnumOption, SessionId, SessionNotification, SessionUpdate, StopReason,
    StringPropertySchema, ToolCall,
};
use agent_client_protocol::{Client, Error};
pub(super) async fn turn(
    cx: &agent_client_protocol::v1::ConnectionTo<Client>,
    session: &SessionId,
) -> Result<StopReason, Error> {
    let mut schema = ElicitationSchema::new();
    for field in 0..32 {
        let options: Vec<EnumOption> = (0..64)
            .map(|option| {
                EnumOption::new(
                    format!("choice-{option}"),
                    format!("Choice {option}: {}", "long description ".repeat(50)),
                )
            })
            .collect();
        schema = schema.property(
            format!("field-{field}"),
            StringPropertySchema::new()
                .title(format!("Field {field}"))
                .one_of(options),
            true,
        );
    }
    let request = CreateElicitationRequest::new(
        ElicitationFormMode::new(ElicitationSessionScope::new(session.clone()), schema),
        "Choose every field, including the final option.",
    );
    let sent = cx.send_request(request);
    for index in 0..130 {
        cx.send_notification(SessionNotification::new(
            session.clone(),
            SessionUpdate::ToolCall(ToolCall::new(
                format!("evict-{index}"),
                "Cache eviction fixture",
            )),
        ))?;
    }
    cx.send_notification(SessionNotification::new(
        session.clone(),
        SessionUpdate::AgentMessageChunk(ContentChunk::new(super::super::text(
            "form source may now be pruned",
        ))),
    ))?;
    let answer = super::answered(sent.block_task().await);
    cx.send_notification(SessionNotification::new(
        session.clone(),
        SessionUpdate::AgentMessageChunk(ContentChunk::new(super::super::text(&answer))),
    ))?;
    Ok(StopReason::EndTurn)
}
