//! Tool calls and permission requests, ACP to Vessel (and a permission's
//! answer back): every ACP kind, status and content mapped, an unknown one
//! to the most cautious known value.

use agent_client_protocol::v1::schema as acp;
use vessel_proto::{
    AgentEvent, PermissionOption, PermissionOptionId, PermissionOptionKind, PermissionOutcome,
    ToolCall, ToolCallContent, ToolCallId, ToolCallStatus, ToolCallUpdate, ToolKind, ToolLocation,
};

use super::{content, opaque, raw_json};

/// Map an announced tool call.
pub(crate) fn tool_call(call: acp::ToolCall) -> ToolCall {
    ToolCall {
        id: ToolCallId::new(call.tool_call_id.0),
        title: call.title,
        kind: tool_kind(call.kind),
        status: tool_status(call.status),
        content: call.content.into_iter().map(tool_content).collect(),
        locations: call.locations.into_iter().map(location).collect(),
        raw_input: call.raw_input.as_ref().and_then(raw_json),

        detail: None,
        failure: None,
        agent_message: None,
    }
}

/// Map a tool call update.
pub(crate) fn tool_call_update(update: acp::ToolCallUpdate) -> ToolCallUpdate {
    let fields = update.fields;
    ToolCallUpdate {
        id: ToolCallId::new(update.tool_call_id.0),
        title: fields.title,
        kind: fields.kind.map(tool_kind),
        status: fields.status.map(tool_status),
        content: fields
            .content
            .map(|content| content.into_iter().map(tool_content).collect()),
        locations: fields
            .locations
            .map(|locations| locations.into_iter().map(location).collect()),
        raw_input: fields.raw_input.as_ref().and_then(raw_json),
        raw_output: fields.raw_output.as_ref().and_then(raw_json),

        detail: None,
        failure: None,
        agent_message: None,
    }
}

/// The tool call a permission request is about, as a timeline event.
///
/// ACP lets a permission request describe a tool call the agent never
/// announced, so clients would have nothing to show. An unknown call is
/// announced here; for a known one, any fields the request carries become
/// an update. `None` when there is nothing new to say.
pub(crate) fn permission_tool_call(update: acp::ToolCallUpdate, known: bool) -> Option<AgentEvent> {
    if known {
        let update = tool_call_update(update);
        let changed = update.title.is_some()
            || update.kind.is_some()
            || update.status.is_some()
            || update.content.is_some()
            || update.locations.is_some()
            || update.raw_input.is_some()
            || update.raw_output.is_some();
        return changed.then_some(AgentEvent::ToolCallUpdate(update));
    }
    let fields = update.fields;
    Some(AgentEvent::ToolCall(ToolCall {
        id: ToolCallId::new(update.tool_call_id.0),
        title: fields.title.unwrap_or_default(),
        kind: fields.kind.map_or(ToolKind::Other, tool_kind),
        status: fields.status.map_or(ToolCallStatus::Pending, tool_status),
        content: fields
            .content
            .unwrap_or_default()
            .into_iter()
            .map(tool_content)
            .collect(),
        locations: fields
            .locations
            .unwrap_or_default()
            .into_iter()
            .map(location)
            .collect(),
        raw_input: fields.raw_input.as_ref().and_then(raw_json),

        detail: None,
        failure: None,
        agent_message: None,
    }))
}

/// Map the options of a permission request.
pub(crate) fn permission_options(options: Vec<acp::PermissionOption>) -> Vec<PermissionOption> {
    options
        .into_iter()
        .map(|option| PermissionOption {
            id: PermissionOptionId::new(option.option_id.0),
            name: option.name,
            kind: permission_kind(option.kind),

            action: None,
        })
        .collect()
}

/// A Vessel permission answer in ACP form.
pub(crate) fn permission_outcome(outcome: &PermissionOutcome) -> acp::RequestPermissionOutcome {
    match outcome {
        PermissionOutcome::Selected { option } => acp::RequestPermissionOutcome::Selected(
            acp::SelectedPermissionOutcome::new(option.as_str().to_owned()),
        ),
        PermissionOutcome::Cancelled => acp::RequestPermissionOutcome::Cancelled,
    }
}

pub(super) fn tool_kind(kind: acp::ToolKind) -> ToolKind {
    match kind {
        acp::ToolKind::Read => ToolKind::Read,
        acp::ToolKind::Edit => ToolKind::Edit,
        acp::ToolKind::Delete => ToolKind::Delete,
        acp::ToolKind::Move => ToolKind::Move,
        acp::ToolKind::Search => ToolKind::Search,
        acp::ToolKind::Execute => ToolKind::Execute,
        acp::ToolKind::Think => ToolKind::Think,
        acp::ToolKind::Fetch => ToolKind::Fetch,
        acp::ToolKind::SwitchMode => ToolKind::SwitchMode,
        acp::ToolKind::Other | _ => ToolKind::Other,
    }
}

/// A future status is reported as still running: it is neither known to
/// have finished nor to have failed.
pub(super) fn tool_status(status: acp::ToolCallStatus) -> ToolCallStatus {
    match status {
        acp::ToolCallStatus::Pending => ToolCallStatus::Pending,
        acp::ToolCallStatus::Completed => ToolCallStatus::Completed,
        acp::ToolCallStatus::Failed => ToolCallStatus::Failed,
        acp::ToolCallStatus::InProgress | _ => ToolCallStatus::InProgress,
    }
}

/// Terminal content is forwarded verbatim: Vessel declares no terminal
/// capability, so agents should not send it, but nothing is lost if one does.
pub(super) fn tool_content(content: acp::ToolCallContent) -> ToolCallContent {
    match content {
        acp::ToolCallContent::Content(inner) => ToolCallContent::Content {
            content: self::content(inner.content),
        },
        acp::ToolCallContent::Diff(diff) => ToolCallContent::Diff {
            path: diff.path,
            old_text: diff.old_text,
            new_text: diff.new_text,
        },
        other @ (acp::ToolCallContent::Terminal(_) | _) => ToolCallContent::Content {
            content: opaque(&other),
        },
    }
}

pub(super) fn location(location: acp::ToolCallLocation) -> ToolLocation {
    ToolLocation {
        path: location.path,
        line: location.line,
    }
}

/// A future option kind is treated as a rejection: a client must never
/// show an unknown choice as "allow".
pub(super) fn permission_kind(kind: acp::PermissionOptionKind) -> PermissionOptionKind {
    match kind {
        acp::PermissionOptionKind::AllowOnce => PermissionOptionKind::AllowOnce,
        acp::PermissionOptionKind::AllowAlways => PermissionOptionKind::AllowAlways,
        acp::PermissionOptionKind::RejectAlways => PermissionOptionKind::RejectAlways,
        acp::PermissionOptionKind::RejectOnce | _ => PermissionOptionKind::RejectOnce,
    }
}
