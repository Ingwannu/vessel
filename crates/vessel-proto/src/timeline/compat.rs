//! Peer-specific optional-field masking, including fields on legacy event kinds.
//! Durable records stay complete; only the outgoing copy is adapted.

pub(super) mod identity;
use super::ReferenceContext;
use identity::needs_escape;

use crate::inbox::{InboxEntry, InboxItem};
use crate::{
    AgentEvent, Conversation, PermissionRequest, PromptQueue, ProtocolVersion, Question, ToolCall,
    ToolCallStatus, ToolCallUpdate,
};

use super::{QuestionDeliveryState, QuestionLifetime};

fn supported(protocol: ProtocolVersion, assigned: Option<ProtocolVersion>) -> bool {
    assigned.is_some_and(|since| protocol >= since)
}

pub(crate) fn tool(call: &ToolCall, context: ReferenceContext) -> ToolCall {
    let mut adapted = call.clone();
    adapted.id = context.project(&call.id, call.scope.as_deref());
    adapted.scope = None;
    adapted.detail = None;
    adapted.failure = None;
    adapted.agent_message = None;
    if adapted.status == ToolCallStatus::Canceled {
        adapted.status = ToolCallStatus::Failed;
    }
    adapted
}

fn update(call: &ToolCallUpdate, context: ReferenceContext) -> ToolCallUpdate {
    let mut adapted = call.clone();
    adapted.id = context.project(&call.id, call.scope.as_deref());
    adapted.scope = None;
    adapted.detail = None;
    adapted.failure = None;
    adapted.agent_message = None;
    if adapted.status == Some(ToolCallStatus::Canceled) {
        adapted.status = Some(ToolCallStatus::Failed);
    }
    adapted
}

fn permission(request: &PermissionRequest, context: ReferenceContext) -> PermissionRequest {
    let mut adapted = request.clone();
    adapted.tool_call = request
        .tool_call
        .as_ref()
        .map(|id| context.project(id, request.tool_scope.as_deref()));
    adapted.tool_scope = None;
    adapted.presentation = None;
    for option in &mut adapted.options {
        option.action = None;
    }
    adapted
}

fn question_changed(request: &Question) -> bool {
    request.tool_call.as_ref().is_some_and(needs_escape)
        || request.tool_scope.is_some()
        || request.lifetime != QuestionLifetime::Turn
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
    !request.legacy_actionable()
        || request.tool_call.as_ref().is_some_and(needs_escape)
        || request.tool_scope.is_some()
        || request.presentation.is_some()
        || request.options.iter().any(|option| option.action.is_some())
}

fn tool_changed(call: &ToolCall) -> bool {
    needs_escape(&call.id)
        || call.scope.is_some()
        || call.detail.is_some()
        || call.failure.is_some()
        || call.agent_message.is_some()
        || call.status == ToolCallStatus::Canceled
}

fn question(request: &Question, context: ReferenceContext) -> Option<Question> {
    if request.lifetime == QuestionLifetime::Session
        || request.delivery_state != QuestionDeliveryState::Pending
    {
        return None;
    }
    let mut adapted = request.clone();
    adapted.tool_call = request
        .tool_call
        .as_ref()
        .map(|id| context.project(id, request.tool_scope.as_deref()));
    adapted.tool_scope = None;
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
        .any(|entry| entry.client_message_id.is_some() || entry.provenance.is_some())
    {
        return None;
    }
    let mut adapted = queue.clone();
    for entry in &mut adapted.entries {
        if entry.provenance.take().is_some() {
            entry.content.clear();
        }
        entry.client_message_id = None;
    }
    Some(adapted)
}

#[cfg(test)]
pub(crate) fn event(
    event: &AgentEvent,
    protocol: ProtocolVersion,
    assigned: Option<ProtocolVersion>,
) -> Option<AgentEvent> {
    event_with_reference(
        event,
        protocol,
        ReferenceContext {
            origin: None,
            assigned,
        },
    )
}

pub(crate) fn event_with_reference(
    event: &AgentEvent,
    protocol: ProtocolVersion,
    context: ReferenceContext,
) -> Option<AgentEvent> {
    let assigned = context.assigned;
    if supported(protocol, assigned) {
        return None;
    }
    let adapted = match event {
        AgentEvent::ToolCall(call) => {
            if !tool_changed(call) {
                return None;
            }
            AgentEvent::ToolCall(tool(call, context))
        }
        AgentEvent::ToolCallUpdate(call) => return update_event(call, context),
        AgentEvent::PermissionRequested(request) => {
            if !request.legacy_actionable() {
                return Some(AgentEvent::HarnessLog {
                    line: "vessel: this approval requires a newer client".to_owned(),
                });
            }
            if !permission_changed(request) {
                return None;
            }
            AgentEvent::PermissionRequested(permission(request, context))
        }
        AgentEvent::QuestionAsked(request) => {
            if !question_changed(request) {
                return None;
            }
            match question(request, context) {
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
            provenance,
        } => {
            if client_message_id.is_none() && provenance.is_none() {
                return None;
            }
            AgentEvent::TurnStarted {
                content: if provenance.is_some() {
                    Vec::new()
                } else {
                    content.clone()
                },
                client_message_id: None,
                provenance: None,
            }
        }
        AgentEvent::Steered {
            content,
            client_message_id,
            provenance,
        } => {
            if client_message_id.is_none() && provenance.is_none() {
                return None;
            }
            AgentEvent::Steered {
                content: if provenance.is_some() {
                    Vec::new()
                } else {
                    content.clone()
                },
                client_message_id: None,
                provenance: None,
            }
        }
        AgentEvent::UserMessageChunk { provenance, .. } => {
            let _local_association = provenance.as_ref()?;
            AgentEvent::UserMessageChunk {
                content: crate::ContentBlock::Text {
                    text: String::new(),
                },
                provenance: None,
            }
        }
        AgentEvent::Ready { .. }
        | AgentEvent::MessageChunk { .. }
        | AgentEvent::ThoughtChunk { .. }
        | AgentEvent::PermissionResolved { .. }
        | AgentEvent::TurnEnded { .. }
        | AgentEvent::TurnFailed { .. }
        | AgentEvent::Notification(_)
        | AgentEvent::Compaction(_)
        | AgentEvent::Plugin(_)
        | AgentEvent::CommandAccepted { .. }
        | AgentEvent::Error { .. }
        | AgentEvent::HarnessLog { .. }
        | AgentEvent::QuestionResolved { .. } => return None,
    };
    (adapted != *event).then_some(adapted)
}

fn update_event(call: &ToolCallUpdate, context: ReferenceContext) -> Option<AgentEvent> {
    if matches!(
        call.scope.as_deref(),
        Some(super::ToolCallScope::Record { .. })
    ) {
        return Some(AgentEvent::HarnessLog {
            line: "vessel: unassociated tool update requires a newer client".to_owned(),
        });
    }
    if !needs_escape(&call.id)
        && call.scope.is_none()
        && call.detail.is_none()
        && call.failure.is_none()
        && call.agent_message.is_none()
        && call.status != Some(ToolCallStatus::Canceled)
    {
        return None;
    }
    let adapted = update(call, context);
    (adapted != *call).then_some(AgentEvent::ToolCallUpdate(adapted))
}

#[cfg(test)]
pub(crate) fn conversation(
    value: &Conversation,
    protocol: ProtocolVersion,
    assigned: Option<ProtocolVersion>,
) -> Option<Conversation> {
    conversation_with_reference(
        value,
        protocol,
        ReferenceContext {
            origin: None,
            assigned,
        },
    )
}

pub(crate) fn conversation_with_reference(
    value: &Conversation,
    protocol: ProtocolVersion,
    context: ReferenceContext,
) -> Option<Conversation> {
    let assigned = context.assigned;
    let composer = value.composer_for_protocol(protocol);
    let has_timeline_change = !supported(protocol, assigned)
        && (value.through.is_some()
            || value.permissions.iter().any(permission_changed)
            || value.questions.iter().any(question_changed)
            || value
                .queue
                .entries
                .iter()
                .any(|entry| entry.client_message_id.is_some() || entry.provenance.is_some()));
    if composer.is_none() && !has_timeline_change {
        return None;
    }
    let mut adapted = composer.unwrap_or_else(|| value.clone());
    if !supported(protocol, assigned) {
        adapted.through = None;
        adapted.permissions = value
            .permissions
            .iter()
            .filter(|value| value.legacy_actionable())
            .map(|value| permission(value, context))
            .collect();
        adapted.questions = value
            .questions
            .iter()
            .filter_map(|value| question(value, context))
            .collect();
        if let Some(queue) = queue(&value.queue, protocol, assigned) {
            adapted.queue = queue;
        }
    }
    (adapted != *value).then_some(adapted)
}

pub(crate) fn inbox_with_reference(
    entry: &InboxEntry,
    protocol: ProtocolVersion,
    context: ReferenceContext,
) -> Option<InboxEntry> {
    let assigned = context.assigned;
    if supported(protocol, assigned) {
        return None;
    }
    let changed = match &entry.item {
        InboxItem::Permission { request, tool, .. } => {
            permission_changed(request) || tool.as_deref().is_some_and(tool_changed)
        }
        InboxItem::Question { question } => question_changed(question),
        InboxItem::InSession {
            permission: Some(_),
            ..
        }
        | InboxItem::InSession {
            question: Some(_), ..
        } => true,
        InboxItem::InSession {
            question: None,
            permission: None,
            ..
        }
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
        InboxItem::Permission { request, .. } if !request.legacy_actionable() => InboxItem::Unknown,
        InboxItem::Permission {
            request,
            tool: body,
            detail,
        } => InboxItem::Permission {
            request: permission(request, context),
            tool: body.as_deref().map(|value| Box::new(tool(value, context))),
            detail: *detail,
        },
        InboxItem::Question { question: request } => match question(request, context) {
            Some(question) => InboxItem::Question { question },
            None => InboxItem::Unknown,
        },
        InboxItem::InSession {
            permission: Some(_),
            ..
        } => InboxItem::Unknown,
        InboxItem::InSession {
            request,
            question: Some(reference),
            permission: None,
        } => {
            if reference.lifetime == QuestionLifetime::Session
                || reference.delivery_state != QuestionDeliveryState::Pending
            {
                InboxItem::Unknown
            } else {
                InboxItem::InSession {
                    request: *request,
                    question: None,
                    permission: None,
                }
            }
        }
        same @ (InboxItem::InSession {
            question: None,
            permission: None,
            ..
        }
        | InboxItem::Finished
        | InboxItem::Failed
        | InboxItem::Limit
        | InboxItem::Unknown) => same.clone(),
    };
    (adapted != *entry).then_some(adapted)
}

#[cfg(test)]
mod tests;
