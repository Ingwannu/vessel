//! Peer-specific optional-field masking, including fields on legacy event kinds.
//! Durable records stay complete; only the outgoing copy is adapted.

use crate::inbox::{InboxEntry, InboxItem};
use crate::{
    AgentEvent, Conversation, PermissionRequest, PromptQueue, ProtocolVersion, Question, ToolCall,
    ToolCallStatus, ToolCallUpdate,
};

use super::{QuestionDeliveryState, QuestionLifetime};

fn supported(protocol: ProtocolVersion, assigned: Option<ProtocolVersion>) -> bool {
    assigned.is_some_and(|since| protocol >= since)
}

pub(crate) fn tool(call: &ToolCall) -> ToolCall {
    let mut adapted = call.clone();
    adapted.detail = None;
    adapted.failure = None;
    adapted.agent_message = None;
    if adapted.status == ToolCallStatus::Canceled {
        adapted.status = ToolCallStatus::Failed;
    }
    adapted
}

fn update(call: &ToolCallUpdate) -> ToolCallUpdate {
    let mut adapted = call.clone();
    adapted.detail = None;
    adapted.failure = None;
    adapted.agent_message = None;
    if adapted.status == Some(ToolCallStatus::Canceled) {
        adapted.status = Some(ToolCallStatus::Failed);
    }
    adapted
}

fn permission(request: &PermissionRequest) -> PermissionRequest {
    let mut adapted = request.clone();
    adapted.presentation = None;
    for option in &mut adapted.options {
        option.action = None;
    }
    adapted
}

fn question_changed(request: &Question) -> bool {
    request.lifetime != QuestionLifetime::Turn
        || request.delivery_state != QuestionDeliveryState::Pending
        || request.fields.iter().any(|field| {
            field.presentation.is_some()
                || match &field.kind {
                    crate::conversation::FieldKind::Choice { options, .. } => {
                        options.iter().any(|option| option.preview.is_some())
                    }
                    crate::conversation::FieldKind::Text { .. }
                    | crate::conversation::FieldKind::Number { .. }
                    | crate::conversation::FieldKind::Integer { .. }
                    | crate::conversation::FieldKind::Boolean { .. }
                    | crate::conversation::FieldKind::Unknown => false,
                }
        })
}

fn permission_changed(request: &PermissionRequest) -> bool {
    request.presentation.is_some() || request.options.iter().any(|option| option.action.is_some())
}

fn tool_changed(call: &ToolCall) -> bool {
    call.detail.is_some()
        || call.failure.is_some()
        || call.agent_message.is_some()
        || call.status == ToolCallStatus::Canceled
}

fn question(request: &Question) -> Option<Question> {
    if request.lifetime == QuestionLifetime::Session
        || request.delivery_state != QuestionDeliveryState::Pending
    {
        return None;
    }
    let mut adapted = request.clone();
    for field in &mut adapted.fields {
        field.presentation = None;
        match &mut field.kind {
            crate::conversation::FieldKind::Choice { options, .. } => {
                for option in options {
                    option.preview = None;
                }
            }
            crate::conversation::FieldKind::Text { .. }
            | crate::conversation::FieldKind::Number { .. }
            | crate::conversation::FieldKind::Integer { .. }
            | crate::conversation::FieldKind::Boolean { .. }
            | crate::conversation::FieldKind::Unknown => {}
        }
    }
    Some(adapted)
}

pub(crate) fn queue(
    queue: &PromptQueue,
    protocol: ProtocolVersion,
    assigned: Option<ProtocolVersion>,
) -> Option<PromptQueue> {
    if supported(protocol, assigned) {
        return None;
    }
    if !queue
        .entries
        .iter()
        .any(|entry| entry.client_message_id.is_some())
    {
        return None;
    }
    let mut adapted = queue.clone();
    for entry in &mut adapted.entries {
        entry.client_message_id = None;
    }
    Some(adapted)
}

pub(crate) fn event(
    event: &AgentEvent,
    protocol: ProtocolVersion,
    assigned: Option<ProtocolVersion>,
) -> Option<AgentEvent> {
    if supported(protocol, assigned) {
        return None;
    }
    let adapted = match event {
        AgentEvent::ToolCall(call) => {
            if !tool_changed(call) {
                return None;
            }
            AgentEvent::ToolCall(tool(call))
        }
        AgentEvent::ToolCallUpdate(call) => {
            if call.detail.is_none()
                && call.failure.is_none()
                && call.agent_message.is_none()
                && call.status != Some(ToolCallStatus::Canceled)
            {
                return None;
            }
            AgentEvent::ToolCallUpdate(update(call))
        }
        AgentEvent::PermissionRequested(request) => {
            if !permission_changed(request) {
                return None;
            }
            AgentEvent::PermissionRequested(permission(request))
        }
        AgentEvent::QuestionAsked(request) => {
            if !question_changed(request) {
                return None;
            }
            match question(request) {
                Some(request) => AgentEvent::QuestionAsked(request),
                None => AgentEvent::HarnessLog {
                    line: "vessel: this question requires a newer client".to_owned(),
                },
            }
        }
        AgentEvent::Plan { entries } => {
            if !entries
                .iter()
                .any(|entry| entry.id.is_some() || entry.active_form.is_some())
            {
                return None;
            }
            let mut entries = entries.clone();
            for entry in &mut entries {
                entry.id = None;
                entry.active_form = None;
            }
            AgentEvent::Plan { entries }
        }
        AgentEvent::TurnStarted {
            content,
            client_message_id,
        } => {
            let _identity = client_message_id.as_ref()?;
            AgentEvent::TurnStarted {
                content: content.clone(),
                client_message_id: None,
            }
        }
        AgentEvent::Steered {
            content,
            client_message_id,
        } => {
            let _identity = client_message_id.as_ref()?;
            AgentEvent::Steered {
                content: content.clone(),
                client_message_id: None,
            }
        }
        AgentEvent::Ready { .. }
        | AgentEvent::UserMessageChunk { .. }
        | AgentEvent::MessageChunk { .. }
        | AgentEvent::ThoughtChunk { .. }
        | AgentEvent::PermissionResolved { .. }
        | AgentEvent::TurnEnded { .. }
        | AgentEvent::TurnFailed { .. }
        | AgentEvent::Notification(_)
        | AgentEvent::Compaction(_)
        | AgentEvent::Plugin(_)
        | AgentEvent::Error { .. }
        | AgentEvent::HarnessLog { .. }
        | AgentEvent::QuestionResolved { .. } => return None,
    };
    (adapted != *event).then_some(adapted)
}

pub(crate) fn conversation(
    value: &Conversation,
    protocol: ProtocolVersion,
    assigned: Option<ProtocolVersion>,
) -> Option<Conversation> {
    let usage = value
        .usage
        .as_ref()
        .and_then(|usage| usage.for_protocol(protocol));
    let has_timeline_change = !supported(protocol, assigned)
        && (value.through.is_some()
            || value.permissions.iter().any(permission_changed)
            || value.questions.iter().any(question_changed)
            || value
                .queue
                .entries
                .iter()
                .any(|entry| entry.client_message_id.is_some()));
    if usage.is_none() && !has_timeline_change {
        return None;
    }
    let mut adapted = value.clone();
    if let Some(usage) = usage {
        adapted.usage = Some(usage);
    }
    if !supported(protocol, assigned) {
        adapted.through = None;
        adapted.permissions = value.permissions.iter().map(permission).collect();
        adapted.questions = value.questions.iter().filter_map(question).collect();
        if let Some(queue) = queue(&value.queue, protocol, assigned) {
            adapted.queue = queue;
        }
    }
    (adapted != *value).then_some(adapted)
}

pub(crate) fn inbox(
    entry: &InboxEntry,
    protocol: ProtocolVersion,
    assigned: Option<ProtocolVersion>,
) -> Option<InboxEntry> {
    if supported(protocol, assigned) {
        return None;
    }
    let changed = match &entry.item {
        InboxItem::Permission { request, tool, .. } => {
            permission_changed(request) || tool.as_deref().is_some_and(tool_changed)
        }
        InboxItem::Question { question } => question_changed(question),
        InboxItem::InSession {
            question: Some(question),
            ..
        } => {
            let _metadata = question;
            true
        }
        InboxItem::InSession { question: None, .. }
        | InboxItem::Finished
        | InboxItem::Failed
        | InboxItem::Limit
        | InboxItem::Unknown => false,
    };
    if !changed {
        return None;
    }
    let mut adapted = entry.clone();
    adapted.item = match &entry.item {
        InboxItem::Permission {
            request,
            tool: body,
            detail,
        } => InboxItem::Permission {
            request: permission(request),
            tool: body.as_deref().map(|value| Box::new(tool(value))),
            detail: *detail,
        },
        InboxItem::Question { question: request } => match question(request) {
            Some(question) => InboxItem::Question { question },
            None => InboxItem::Unknown,
        },
        InboxItem::InSession {
            request,
            question: Some(reference),
        } => {
            if reference.lifetime == QuestionLifetime::Session
                || reference.delivery_state != QuestionDeliveryState::Pending
            {
                InboxItem::Unknown
            } else {
                InboxItem::InSession {
                    request: *request,
                    question: None,
                }
            }
        }
        same @ (InboxItem::InSession { question: None, .. }
        | InboxItem::Finished
        | InboxItem::Failed
        | InboxItem::Limit
        | InboxItem::Unknown) => same.clone(),
    };
    (adapted != *entry).then_some(adapted)
}

#[cfg(test)]
mod tests;
