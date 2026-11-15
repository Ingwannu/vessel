//! Plain-text renderings of node, harness and session descriptions. Pure
//! functions returning text, so they are tested without a terminal.

use std::fmt::Write as _;

mod conversation;
mod inbox;
pub(crate) use inbox::inbox_entry;
mod worktree;

pub(crate) use conversation::{
    commands, delivered, question, question_form, question_outcome, queue, queue_entries, usage,
};
pub(crate) use worktree::{clone_stage, deleted, refusal, removal, setup_failure};

use vessel_proto::{
    EnvName, Harness, ModelApi, ModelCall, ModelRoute, ModelStop, NodeInfo, NodeState, ProfileInfo,
    ProfileName, SessionInfo, SessionMode, SessionOption, SessionStatus, StructuredSupport,
};

/// A session's status in a few words.
pub(crate) fn status(status: &SessionStatus) -> String {
    match status {
        SessionStatus::Starting => "starting".to_owned(),
        SessionStatus::Running => "running".to_owned(),
        SessionStatus::Exited { code: Some(code) } => format!("exited ({code})"),
        SessionStatus::Exited { code: None } => "exited (killed)".to_owned(),
        // A fork that never held a conversation (protocol 14): nothing to
        // resume, only to remove.
        SessionStatus::Failed {
            message,
            empty_fork: true,
        } => format!("failed: {message} (an empty fork: `vessel session delete` removes it)"),
        SessionStatus::Failed { message, .. } => format!("failed: {message}"),
        SessionStatus::Interrupted => "interrupted".to_owned(),
    }
}

/// One model call: the route and model, the tokens, how it ended and how
/// long it took (protocol 7). Never content: the record holds none.
pub(crate) fn model_call(call: &ModelCall) -> String {
    let ended = call.failure.map_or_else(
        || stop(call.stop).to_owned(),
        |failure| format!("failed: {}", failure.as_str()),
    );
    let usage = &call.usage;
    let mut text = format!(
        "[model {} {}: {} in",
        call.route,
        call.model,
        usage.input.get()
    );
    if usage.cache_read.get() > 0 || usage.cache_write.get() > 0 {
        let _infallible = write!(
            text,
            " (+{} cached, +{} to cache)",
            usage.cache_read.get(),
            usage.cache_write.get()
        );
    }
    let _infallible = write!(
        text,
        ", {} out, {ended}, {:.1?}]",
        usage.output.get(),
        call.duration_ms.as_duration()
    );
    text
}

/// A session's options as its agent reports them (protocol 9), in one
/// line: the protocol's own summary, which an older client also reads.
pub(crate) fn options(options: &[SessionOption]) -> String {
    SessionOption::summary(options)
}

const fn stop(stop: ModelStop) -> &'static str {
    match stop {
        ModelStop::EndTurn => "end turn",
        ModelStop::ToolUse => "tool use",
        ModelStop::MaxTokens => "max tokens",
        ModelStop::StopSequence => "stop sequence",
        ModelStop::Refusal => "refused",
        ModelStop::Pause => "paused",
        ModelStop::Error => "error",
    }
}

/// A profile's name, or `default` for the harness's implicit one.
pub(crate) fn profile(profile: Option<&ProfileName>) -> &str {
    profile.map_or("default", ProfileName::as_str)
}

/// `terminal` or `structured`, plus what matters about the mode.
pub(crate) fn mode(mode: &SessionMode) -> String {
    match mode {
        SessionMode::Terminal { args, .. } if args.is_empty() => "terminal".to_owned(),
        SessionMode::Terminal { args, .. } => format!("terminal {}", args.join(" ")),
        SessionMode::Structured {
            model: Some(model), ..
        } => format!("structured ({model})"),
        SessionMode::Structured { model: None, .. } => "structured".to_owned(),
    }
}

pub(crate) fn node(node: &NodeInfo) -> String {
    format!(
        "node      {} ({})\nhost      {} {}/{}\nversion   {} (protocol {}..={})\nstarted   {}\ntimezone  {}\n",
        node.name,
        node.id,
        node.hostname,
        node.os,
        node.arch,
        node.version,
        node.protocol.min,
        node.protocol.max,
        node.started_at.to_rfc3339(),
        node.timezone
            .as_ref()
            .map_or("unknown", |zone| zone.as_str()),
    )
}

/// `vessel node status`: the node, whether it takes new sessions, how many
/// are live, and whether it runs this client's version.
/// `vessel node settings`: the settings in force (protocol 12).
pub(crate) fn node_settings(settings: &vessel_proto::settings::NodeSettings) -> String {
    use vessel_proto::parking::ParkAfter;
    let park_after = match settings.park_after {
        ParkAfter::Never => String::from("never"),
        ParkAfter::Idle { minutes } => format!("after {} minutes idle", minutes.get()),
        ParkAfter::Unknown => String::from("a newer node's choice"),
    };
    let drivers = [
        vessel_proto::HarnessKind::ClaudeCode,
        vessel_proto::HarnessKind::Codex,
        vessel_proto::HarnessKind::OpenCode,
    ]
    .into_iter()
    .filter_map(|kind| {
        let driver = match settings.drivers.get(kind)? {
            vessel_proto::settings::AgentDriver::Direct => "its own protocol",
            vessel_proto::settings::AgentDriver::Acp
            | vessel_proto::settings::AgentDriver::Unknown => "ACP",
        };
        Some(format!("{kind} over {driver}"))
    })
    .collect::<Vec<_>>()
    .join(", ");
    format!("park an idle agent: {park_after}\nnew sessions: {drivers}\n")
}

pub(crate) fn node_status(node: &NodeInfo, sessions: &[SessionInfo], client: &str) -> String {
    let state = match node.state {
        NodeState::Serving => "serving",
        NodeState::Draining => "draining (refuses new sessions; stops when the last one ends)",
    };
    let live = sessions
        .iter()
        .filter(|session| session.status.is_live())
        .count();
    let mut text = self::node(node);
    let _infallible = writeln!(text, "state     {state}");
    if let Some(key) = &node.public_key {
        // What a person compares with the fingerprint shown when pairing.
        let _infallible = writeln!(text, "key       {}", vessel_client::fingerprint(key));
    }
    let _infallible = writeln!(text, "sessions  {live} live, {} in all", sessions.len());
    if node.version != client {
        let _infallible = writeln!(
            text,
            "update    this client is {client}; `vessel node update` installs it there"
        );
    }
    text
}

pub(crate) fn harnesses(harnesses: &[Harness]) -> String {
    if harnesses.is_empty() {
        return "no harnesses found on the node's PATH\n".to_owned();
    }
    let mut text = String::new();
    for harness in harnesses {
        let structured = match &harness.structured {
            StructuredSupport::Available => "structured: yes".to_owned(),
            StructuredSupport::Unavailable { reason } => format!("structured: no ({reason})"),
        };
        // Writing to a `String` cannot fail.
        let _infallible = writeln!(
            text,
            "{:<12} {:<28} {}  {}",
            harness.kind,
            harness.version.as_deref().unwrap_or("-"),
            harness.path.display(),
            structured,
        );
    }
    text
}

/// One profile per line: name, harness, its model route (`-` for the
/// harness's own provider), the variables it sets (names only; the node
/// never sends values), and its label.
pub(crate) fn profiles(profiles: &[ProfileInfo]) -> String {
    if profiles.is_empty() {
        return "no profiles\n".to_owned();
    }
    let mut text = String::new();
    for profile in profiles {
        let sets = if profile.sets.is_empty() {
            "-".to_owned()
        } else {
            profile
                .sets
                .iter()
                .map(EnvName::as_str)
                .collect::<Vec<_>>()
                .join(",")
        };
        let route = profile
            .model
            .as_ref()
            .map_or_else(|| "-".to_owned(), ToString::to_string);
        let _infallible = writeln!(
            text,
            "{:<20} {:<12} {:<16} {:<32} {}",
            profile.name,
            profile.harness,
            route,
            sets,
            profile.label.as_deref().unwrap_or(""),
        );
    }
    text
}

/// One model route per line: name, provider, API, model, and its small
/// model when it has one. Never a URL or a secret name: the node sends
/// neither.
pub(crate) fn models(routes: &[ModelRoute]) -> String {
    if routes.is_empty() {
        return "no model routes (the node's models.toml defines them)\n".to_owned();
    }
    let mut text = String::new();
    for route in routes {
        let _infallible = write!(
            text,
            "{:<20} {:<16} {:<24} {}",
            route.name,
            route.provider,
            api(route.api),
            route.model,
        );
        if let Some(small) = &route.small_model {
            let _infallible = write!(text, " (small: {small})");
        }
        text.push('\n');
    }
    text
}

pub(crate) const fn api(api: ModelApi) -> &'static str {
    match api {
        ModelApi::AnthropicMessages => "anthropic-messages",
        ModelApi::OpenAiResponses => "openai-responses",
        ModelApi::OpenAiChatCompletions => "openai-chat-completions",
    }
}

pub(crate) fn sessions(sessions: &[SessionInfo]) -> String {
    if sessions.is_empty() {
        return "no sessions\n".to_owned();
    }
    let mut text = String::new();
    for session in sessions {
        let _infallible = write!(
            text,
            "{}  {:<12} {:<18} {:<16} {}",
            session.id,
            session.spec.harness,
            status(&session.status),
            mode(&session.spec.mode),
            session.created_at.format("%Y-%m-%d %H:%M:%S"),
        );
        let marks = conversation::marks(&session.marks);
        // Protocol 12: a running session whose agent is parked says so.
        if session.parked.is_some() {
            text.push_str("  (parked)");
        }
        if !marks.is_empty() {
            let _infallible = write!(text, "  [{marks}]");
        }
        text.push('\n');
    }
    text
}

#[cfg(test)]
mod tests;
