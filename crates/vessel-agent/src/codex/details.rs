//! Native item fields map directly into canonical tool bodies, without reparsing titles.
use super::protocol::ThreadItem;
use vessel_proto::timeline::{
    EditDetail, SearchDetail, SearchSource, ShellDetail, ToolCallDetail, ToolExitCode,
};

pub(super) fn detail(item: &ThreadItem) -> Option<Box<ToolCallDetail>> {
    let body = match item {
        ThreadItem::CommandExecution {
            command,
            cwd,
            aggregated_output,
            exit_code,
            ..
        } => ToolCallDetail::Shell(ShellDetail {
            command: command.clone(),
            cwd: cwd.as_ref().map(Into::into),
            output: aggregated_output.clone(),
            exit_code: exit_code.map(ToolExitCode::new),
        }),
        ThreadItem::FileChange { changes, .. } => {
            let [change] = changes.as_slice() else {
                return None;
            };
            ToolCallDetail::Edit(EditDetail {
                file_path: (&change.path).into(),
                old_string: None,
                new_string: None,
                unified_diff: Some(change.diff.clone()),
            })
        }
        ThreadItem::WebSearch { query, .. } => ToolCallDetail::Search(SearchDetail {
            query: query.clone(),
            tool_name: Some(SearchSource::WebSearch),
            content: None,
            file_paths: None,
            web_results: None,
            annotations: None,
            num_files: None,
            num_matches: None,
            duration: None,
            truncated: None,
            mode: None,
        }),
        ThreadItem::McpToolCall { arguments, .. } => {
            crate::canonical::unknown(Some(arguments), None)
        }
        ThreadItem::AgentMessage { .. } | ThreadItem::Reasoning { .. } | ThreadItem::Other => {
            return None;
        }
    };
    Some(Box::new(body))
}
