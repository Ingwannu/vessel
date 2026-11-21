//! A prompt's context and review blocks (ADR 0124) as its message shows
//! them: Paseo's sent attachments (`components/message.tsx` 480-510 through
//! `getAgentAttachmentPillContent`), a context as its title over "Text"
//! (earlier conversation: "Previous conversation"), a review as "Review"
//! over its count of comments. They open nothing, as Paseo's sent pills do
//! (the coordinator's P6b ruling).

use vessel_proto::ContentBlock;
use vessel_proto::prompt_context::ContextKind;

/// One context or review a prompt carried.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SentContext {
    /// A `context` block.
    Context {
        /// Its pill's first line.
        title: String,
        /// Earlier conversation carried in (Paseo's "Previous conversation"
        /// where any other context says "Text").
        history: bool,
    },
    /// A `review` block.
    Review {
        /// How many comments it carries.
        comments: usize,
    },
}

/// The context and review blocks of `content`, in order.
#[must_use]
pub fn sent_context(content: &[ContentBlock]) -> Vec<SentContext> {
    content
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Context(attached) => Some(SentContext::Context {
                title: attached.title.as_str().to_owned(),
                history: attached.kind == ContextKind::ChatHistory,
            }),
            ContentBlock::Review(review) => Some(SentContext::Review {
                comments: review.comments.len(),
            }),
            ContentBlock::Text { .. }
            | ContentBlock::ResourceLink { .. }
            | ContentBlock::Opaque { .. }
            | ContentBlock::Attachment { .. } => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_prompt_s_context_and_review_are_its_sent_pills_in_order() {
        let content: Vec<ContentBlock> = serde_json::from_value(serde_json::json!([
            { "type": "text", "text": "Fix these." },
            {
                "type": "context", "kind": "pull_check", "title": "test (linux)",
                "text": "GitHub pull request check",
            },
            {
                "type": "review", "root": "/work/fleet-agent", "mode": "uncommitted",
                "comments": [{
                    "path": "src/lib.rs", "side": "new", "line": 3, "body": "Why?",
                    "hunk_header": "@@ -1,2 +1,3 @@",
                    "target": { "new": 3, "kind": "add", "content": "x" },
                    "lines": [{ "new": 3, "kind": "add", "content": "x" }],
                }],
            },
            {
                "type": "context", "kind": "chat_history", "title": "Earlier",
                "text": "user: hello",
            },
        ]))
        .unwrap();
        assert_eq!(
            sent_context(&content),
            vec![
                SentContext::Context {
                    title: "test (linux)".to_owned(),
                    history: false,
                },
                SentContext::Review { comments: 1 },
                SentContext::Context {
                    title: "Earlier".to_owned(),
                    history: true,
                },
            ]
        );
    }
}
