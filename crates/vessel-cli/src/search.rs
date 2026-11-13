//! `vessel search` (protocol 12, ADR 0037): one page of a search over every
//! session's messages, one line a hit, then how to go on.

use std::fmt::Write as _;

use vessel_proto::search::{MessageRole, SearchHit, SearchMessages, SearchPage};

use crate::args::{Search, Target};
use crate::connect::connect;
use crate::error::CliError;
use crate::io;

/// `vessel search <words>`.
pub(crate) async fn search(target: &Target, search: Search) -> Result<(), CliError> {
    let query = search.query()?;
    let client = connect(target).await?.client;
    let session = match &search.session {
        Some(selector) => Some(selector.resolve(&client).await?.id),
        None => None,
    };
    let page = client
        .call(SearchMessages {
            query,
            session,
            before: search.before,
            limit: Some(search.limit),
        })
        .await?;
    Ok(io::out(text(&page).as_bytes()).await?)
}

/// The page as lines: the session's short id, the record, the time, who
/// spoke, and the snippet with each match in `[` `]`.
fn text(page: &SearchPage) -> String {
    let mut text = String::new();
    if page.hits.is_empty() {
        text.push_str("no message matches");
        text.push_str(if page.next.is_some() {
            " in this stretch\n"
        } else {
            "\n"
        });
    }
    for hit in &page.hits {
        line(&mut text, hit);
    }
    if let Some(next) = page.next {
        let _written = writeln!(text, "more: --before {next}");
    }
    text
}

fn line(text: &mut String, hit: &SearchHit) {
    let session = hit.session.to_string();
    let short = session.get(..8).unwrap_or(&session);
    let who = match hit.role {
        MessageRole::Prompt => "prompt",
        MessageRole::Reply => "reply",
    };
    let _written = write!(
        text,
        "{short} #{} {} {who}: {}",
        hit.seq.get(),
        hit.at.format("%Y-%m-%d %H:%M"),
        if hit.snippet.cut_before { "…" } else { "" },
    );
    for part in &hit.snippet.parts {
        if part.matched {
            let _written = write!(text, "[{}]", part.text);
        } else {
            text.push_str(&part.text);
        }
    }
    if hit.snippet.cut_after {
        text.push('…');
    }
    text.push('\n');
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use vessel_proto::search::{SearchCursor, Snippet, SnippetPart};
    use vessel_proto::{EventSeq, SessionId};

    use super::*;

    #[test]
    fn a_hit_is_one_line_with_its_matches_marked() {
        let session: SessionId = "0b5a2c1e-0000-4000-8000-000000000000".parse().unwrap();
        let page = SearchPage {
            hits: vec![SearchHit {
                session,
                seq: EventSeq::new(12),
                at: DateTime::from_timestamp(1_700_000_000, 0).unwrap(),
                role: MessageRole::Reply,
                snippet: Snippet {
                    parts: vec![
                        SnippetPart {
                            text: "the ".to_owned(),
                            matched: false,
                        },
                        SnippetPart {
                            text: "parser".to_owned(),
                            matched: true,
                        },
                    ],
                    cut_before: true,
                    cut_after: true,
                },
            }],
            next: Some(SearchCursor::new(42)),
            reached: None,
        };
        assert_eq!(
            text(&page),
            "0b5a2c1e #12 2023-11-14 22:13 reply: …the [parser]…\nmore: --before 42\n"
        );
    }
}
