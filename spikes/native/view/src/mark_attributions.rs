//! The shipped agent artwork's source and terms, kept separate from crate
//! licences: a software licence does not grant trademark rights.
use crate::agent_mark::AgentMark;

/// Attribution for one official mark embedded by the native kit.
#[derive(Debug, Clone, Copy)]
pub struct MarkAttribution {
    /// The artwork's semantic identity.
    pub mark: AgentMark,
    /// The agent or vendor identified by it.
    pub name: &'static str,
    /// The trademark owner.
    pub owner: &'static str,
    /// The owner's source page, not an icon collection.
    pub source: &'static str,
    /// The owner's terms.
    pub terms: &'static str,
    /// A redistributed original's software licence, separate from its trademark terms.
    pub licence: Option<&'static str>,
}
/// All five marks shipped in the kit, in the same order as the view's set.
pub const MARKS: [MarkAttribution; 5] = [
    MarkAttribution {
        mark: AgentMark::ClaudeCode,
        name: "Claude Code",
        owner: "Anthropic",
        source: "https://www.anthropic.com/news",
        terms: "https://www.anthropic.com/legal/trademark-guidelines",
        licence: None,
    },
    MarkAttribution {
        mark: AgentMark::Codex,
        name: "Codex / OpenAI",
        owner: "OpenAI",
        source: "https://github.com/openai/openai-agents-python/blob/main/docs/assets/logo.svg",
        terms: "https://openai.com/brand/",
        licence: None,
    },
    MarkAttribution {
        mark: AgentMark::Gemini,
        name: "Gemini CLI",
        owner: "Google",
        // Google's own file, as the Gemini app (gemini.google.com) serves it
        // from Google's CDN, unchanged; Google publishes no licence with it.
        // The owner's word 30 (2026-11-14): the Gemini mark may be used.
        source: "https://www.gstatic.com/lamda/images/gemini_sparkle_aurora_33f86dc0c0257da337c63.svg",
        terms: "https://about.google/brand-resource-center/brand-elements/",
        licence: None,
    },
    MarkAttribution {
        mark: AgentMark::OpenCode,
        name: "OpenCode",
        owner: "Anomaly",
        source: "https://opencode.ai/brand",
        terms: "https://opencode.ai/brand",
        licence: None,
    },
    MarkAttribution {
        mark: AgentMark::Copilot,
        name: "GitHub Copilot",
        owner: "GitHub",
        source: "https://brand.github.com/brand-identity/copilot",
        terms: "https://brand.github.com/foundations/logo",
        licence: None,
    },
];
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_bundled_brand_is_attributed_once() {
        let marks: Vec<AgentMark> = MARKS.iter().map(|entry| entry.mark).collect();
        assert_eq!(marks, AgentMark::BRANDS);
        for mark in MARKS {
            assert!(mark.source.starts_with("https://"));
            assert!(mark.terms.starts_with("https://"));
        }
    }
}
