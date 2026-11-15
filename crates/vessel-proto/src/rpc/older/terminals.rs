//! Attention is optional on every terminal projection, including nested
//! fleet replies and subscription acknowledgements. None is unassigned.
use crate::{
    ProtocolVersion,
    terminals::{TerminalInfo, TerminalList, TerminalSubscribed},
};
fn supported(protocol: ProtocolVersion) -> bool {
    crate::notifications::ATTENTION_NOTIFICATIONS_PROTOCOL.is_some_and(|since| protocol >= since)
}
pub(super) fn info(info: &TerminalInfo, protocol: ProtocolVersion) -> Option<TerminalInfo> {
    info.for_protocol(protocol)
}
pub(super) fn list(list: &TerminalList, protocol: ProtocolVersion) -> Option<TerminalList> {
    if supported(protocol) || list.terminals.iter().all(|info| info.attention.is_none()) {
        return None;
    }
    Some(TerminalList {
        terminals: list
            .terminals
            .iter()
            .map(|terminal| info(terminal, protocol).unwrap_or_else(|| terminal.clone()))
            .collect(),
    })
}
pub(super) fn subscribed(
    subscribed: &TerminalSubscribed,
    protocol: ProtocolVersion,
) -> Option<TerminalSubscribed> {
    info(&subscribed.terminal, protocol).map(|terminal| TerminalSubscribed {
        subscription: subscribed.subscription,
        terminal,
    })
}

pub(super) fn scripts(
    list: &crate::scripts::ScriptList,
    protocol: ProtocolVersion,
) -> Option<crate::scripts::ScriptList> {
    let commands = list.commands.for_protocol(protocol);
    let changed = commands.is_some()
        || list
            .runs
            .iter()
            .any(|run| run.for_protocol(protocol).is_some());
    if !changed {
        return None;
    }
    let mut older = list.clone();
    if let Some(commands) = commands {
        older.commands = commands;
    }
    older.runs = list
        .runs
        .iter()
        .map(|run| run.for_protocol(protocol).unwrap_or_else(|| run.clone()))
        .collect();
    Some(older)
}
