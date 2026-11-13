//! The sole view-to-kit artwork adapter. The view owns identity decisions;
//! this module only supplies embedded data and drawing controls to areas.
use vessel_app_kit::{
    Tokens,
    agent_mark::{self, Artwork, Size, Space},
    ui::Div,
};
use vessel_app_view::agent_mark::AgentMark;
use vessel_proto::HarnessRef;

pub(crate) const fn artwork(mark: AgentMark) -> Option<Artwork> {
    match mark {
        AgentMark::ClaudeCode => Some(agent_mark::CLAUDE),
        AgentMark::Codex => Some(agent_mark::CODEX),
        AgentMark::Gemini => Some(agent_mark::GEMINI),
        AgentMark::OpenCode => Some(agent_mark::OPENCODE),
        AgentMark::Copilot => Some(agent_mark::COPILOT),
        AgentMark::Generic => None,
    }
}
pub(crate) const fn of(agent: &HarnessRef) -> Option<Artwork> {
    artwork(AgentMark::of(agent))
}
pub(crate) fn leading(agent: &HarnessRef) -> vessel_app_kit::Leading {
    vessel_app_kit::Leading::AgentMark(of(agent))
}
pub(crate) fn mark(agent: Option<&HarnessRef>, size: Size, space: Space, tokens: &Tokens) -> Div {
    agent_mark::agent_mark(agent.and_then(of), size, space, tokens)
}
