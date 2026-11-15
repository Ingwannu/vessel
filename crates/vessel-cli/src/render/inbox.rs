//! Inbox text shares the CLI render boundary; content stays out of diagnostics.
use std::fmt::Write as _;
use vessel_proto::inbox::{InboxEntry, InboxItem, InboxRead};

pub(crate) fn inbox_entry(entry: &InboxEntry) -> String {
    let mut text = String::new();
    let state = match entry.read {
        InboxRead::Read => "read",
        InboxRead::Unread => "unread",
    };
    let kind = match &entry.item {
        InboxItem::Permission { .. } => "approval",
        InboxItem::Question { .. } => "question",
        InboxItem::InSession { .. } | InboxItem::Unknown => "open session",
        InboxItem::Finished => "finished",
        InboxItem::Failed => "failed",
        InboxItem::Limit => "limit",
    };
    let _written = writeln!(
        text,
        "{}  {}  {}  {}  session={}  seq={}",
        entry.cursor.get(),
        state,
        kind,
        entry.at.format("%Y-%m-%d %H:%M:%S UTC"),
        entry.key.session,
        entry.key.seq
    );
    if let Some(title) = &entry.title {
        let _written = writeln!(text, "  {title}");
    }
    if let Some(project) = &entry.project_name {
        let _written = writeln!(text, "  {project}");
    } else if let Some(project) = entry.project {
        let _written = writeln!(text, "  project={project}");
    }
    match &entry.item {
        InboxItem::Permission {
            request,
            tool,
            detail,
        } => {
            if let Some(tool) = tool {
                tool_detail(&mut text, tool);
            }
            if *detail == vessel_proto::inbox::InboxDetail::InSession {
                text.push_str("  Some tool details are omitted here. Open the session to inspect retained history.\n");
            }
            for option in &request.options {
                let _written = writeln!(text, "  {}: {}", option.id, option.name);
            }
        }
        InboxItem::Question { question } => {
            let _written = writeln!(text, "  {}", crate::render::question_form(question));
        }
        InboxItem::InSession { .. }
        | InboxItem::Finished
        | InboxItem::Failed
        | InboxItem::Limit
        | InboxItem::Unknown => {}
    }
    if entry.item.waiting() {
        let _written = writeln!(
            text,
            "  answer with `vessel inbox answer {} ...`",
            entry.cursor.get()
        );
    }
    text
}

fn tool_detail(text: &mut String, tool: &vessel_proto::ToolCall) {
    use vessel_proto::{ContentBlock, ToolCallContent};
    let _written = writeln!(text, "  {}", tool.title);
    if let Some(input) = &tool.raw_input {
        let _written = writeln!(text, "  {}", input.as_text());
    }
    for location in &tool.locations {
        let _written = writeln!(text, "  {}", location.path.display());
    }
    for content in &tool.content {
        match content {
            ToolCallContent::Content { content } => match content {
                ContentBlock::Text { text: content } => {
                    let _written = writeln!(text, "  {content}");
                }
                ContentBlock::ResourceLink { uri, name } => {
                    let _written = writeln!(text, "  {name}: {uri}");
                }
                ContentBlock::Opaque { raw } => {
                    let _written = writeln!(text, "  {}", raw.as_text());
                }
                ContentBlock::Attachment { attachment } => {
                    let _written = writeln!(text, "  file: {}", attachment.name);
                }
            },
            ToolCallContent::Diff {
                path,
                old_text,
                new_text,
            } => {
                let _written = writeln!(text, "  {}", path.display());
                if let Some(before) = old_text {
                    for line in before.lines() {
                        let _written = writeln!(text, "-{line}");
                    }
                }
                for line in new_text.lines() {
                    let _written = writeln!(text, "+{line}");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn entry(item: InboxItem) -> InboxEntry {
        InboxEntry {
            cursor: vessel_proto::inbox::InboxCursor::new(17),
            key: vessel_proto::inbox::InboxKey {
                session: "00000000-0000-4000-8000-000000000001".parse().unwrap(),
                seq: vessel_proto::EventSeq::new(1),
            },
            title: None,
            project: None,
            project_name: None,
            at: chrono::DateTime::UNIX_EPOCH,
            read: InboxRead::Unread,
            item,
        }
    }
    #[test]
    fn question_keeps_its_field_and_option_details_and_inbox_answer_hint() {
        let question = serde_json::from_str(r#"{
            "id":"00000000-0000-4000-8000-000000000011", "message":"Choose a route.",
            "fields":[{"name":"route","title":"Route","description":"Choose the route for this run.","required":true,"kind":"choice","multiple":false,
            "options":[{"value":"local","title":"Local","description":"Keep the run on this machine."}]}]
        }"#).unwrap();
        let entry = entry(InboxItem::Question { question });
        assert_eq!(
            inbox_entry(&entry),
            concat!(
                "17  unread  question  1970-01-01 00:00:00 UTC  session=00000000-0000-4000-8000-000000000001  seq=1\n",
                "  [question 00000000-0000-4000-8000-000000000011] Choose a route.\n",
                "  route (required): Route (choose)\n",
                "    Choose the route for this run.\n",
                "    local: Local\n",
                "      Keep the run on this machine.\n",
                "  answer with `vessel inbox answer 17 ...`\n"
            )
        );
    }
    #[test]
    fn omitted_details_never_promise_that_pruned_content_can_be_opened() {
        let request = vessel_proto::PermissionRequest {
            plan_proposal: None,
            delivery_state: vessel_proto::timeline::PermissionDeliveryState::Pending,
            tool_scope: None,
            id: "00000000-0000-4000-8000-000000000012".parse().unwrap(),
            tool_call: Some(vessel_proto::ToolCallId::new("missing")),
            options: vec![vessel_proto::PermissionOption {
                id: vessel_proto::PermissionOptionId::new("allow"),
                name: "Allow once".to_owned(),
                kind: vessel_proto::PermissionOptionKind::AllowOnce,

                action: None,
            }],

            presentation: None,
        };
        let entry = entry(InboxItem::Permission {
            request,
            tool: None,
            detail: vessel_proto::inbox::InboxDetail::InSession,
        });
        assert_eq!(
            inbox_entry(&entry),
            concat!(
                "17  unread  approval  1970-01-01 00:00:00 UTC  session=00000000-0000-4000-8000-000000000001  seq=1\n",
                "  Some tool details are omitted here. Open the session to inspect retained history.\n",
                "  allow: Allow once\n",
                "  answer with `vessel inbox answer 17 ...`\n"
            )
        );
    }
}
