//! Inter-agent delivery presentation takes priority without granting navigation
//! or node authority. Only the summary is normalized; the full body stays literal.

use super::{DetailContents, ToolDisplay, ToolLabel, is_space};
use std::borrow::Cow;
use vessel_proto::timeline::{AgentMessageEvent, AgentMessageProvenance, ToolIcon};

/// Paseo summary-text.ts bounds both the output and its whitespace scan in UTF-16.
const SUMMARY_UNITS: usize = 120;
const SCAN_UNITS: usize = 480;

/// The provider sender title falls back to its external id; neither is a route.
#[must_use]
pub fn agent_message_display(message: &AgentMessageProvenance) -> ToolDisplay<'_> {
    let sender = message
        .sender
        .title
        .as_ref()
        .map_or_else(|| message.sender.id.as_str(), |title| title.as_str());
    ToolDisplay {
        label: ToolLabel::AgentMessage(message.event),
        summary: summary(sender).map(Cow::Owned),
        // The native renderer uses its shared host mark for deliveries. Bot is
        // the protocol's legacy fallback; no new wire icon variant is invented.
        icon: ToolIcon::Bot,
        contents: DetailContents::Present,
    }
}

fn summary(text: &str) -> Option<String> {
    let scanned = utf16_prefix(text, SCAN_UNITS);
    let partial = scanned.len() < text.len();
    let normalized = scanned
        .split(is_space)
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if normalized.is_empty() {
        return None;
    }
    if normalized.encode_utf16().count() <= SUMMARY_UNITS && !partial {
        return Some(normalized);
    }
    let kept = utf16_prefix(&normalized, SUMMARY_UNITS.saturating_sub(1));
    Some(format!("{}…", kept.trim_end()))
}

/// Rust scalar boundaries also avoid the source's dangling-high-surrogate edge.
fn utf16_prefix(text: &str, limit: usize) -> &str {
    let mut units = 0_usize;
    let mut end = 0_usize;
    for (offset, character) in text.char_indices() {
        // Both counters stop at the named scan budget; saturating keeps this
        // helper safe even when called on an unusually large provider value.
        let next = units.saturating_add(character.len_utf16());
        if next > limit {
            break;
        }
        units = next;
        end = offset.saturating_add(character.len_utf8());
    }
    text.get(..end).unwrap_or_default()
}

impl<'a> crate::localization::TimelineTool<'a> {
    /// One exhaustive adapter for all five independent delivery events.
    #[must_use]
    pub fn agent_message(self, event: AgentMessageEvent) -> &'a str {
        match event {
            AgentMessageEvent::Message => self.message_received(),
            AgentMessageEvent::Finished => self.agent_finished(),
            AgentMessageEvent::Errored => self.agent_errored(),
            AgentMessageEvent::PermissionRequired => self.agent_permission(),
            AgentMessageEvent::Closed => self.agent_closed(),
        }
    }
}

#[cfg(test)]
mod tests;
