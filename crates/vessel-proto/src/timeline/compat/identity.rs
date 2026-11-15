//! An old raw-id fold must still distinguish two scoped native calls.
//! The reserved namespace is escaped for legacy ids too, so a provider cannot
//! choose an unscoped id equal to another call's encoded scoped identity.
use super::super::ToolCallScope;
use crate::ToolCallId;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

const LEGACY_SCOPE_NAMESPACE: &str = "vessel-timeline:v1:";

pub(super) fn needs_escape(id: &ToolCallId) -> bool {
    id.as_str().starts_with(LEGACY_SCOPE_NAMESPACE)
}

pub(super) fn legacy_id(id: &ToolCallId, scope: Option<&ToolCallScope>) -> ToolCallId {
    let encode = |value: &str| URL_SAFE_NO_PAD.encode(value.as_bytes());
    let key = match scope {
        Some(ToolCallScope::ProviderTurn { thread, turn }) => format!(
            "{LEGACY_SCOPE_NAMESPACE}p:{}:{}:{}",
            encode(thread.as_str()),
            encode(turn.as_str()),
            encode(id.as_str())
        ),
        Some(ToolCallScope::SessionTurn { started }) => format!(
            "{LEGACY_SCOPE_NAMESPACE}s:{}:{}",
            started.get(),
            encode(id.as_str())
        ),
        Some(ToolCallScope::Record { seq }) => format!(
            "{LEGACY_SCOPE_NAMESPACE}r:{}:{}",
            seq.get(),
            encode(id.as_str())
        ),
        None if needs_escape(id) => format!("{LEGACY_SCOPE_NAMESPACE}l:{}", encode(id.as_str())),
        None => return id.clone(),
    };
    ToolCallId::new(key)
}
