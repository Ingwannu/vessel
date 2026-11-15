//! Known native names select typed argument codecs; display titles never do.
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;
use std::path::PathBuf;
use vessel_proto::timeline::{
    DetailCount, DetailOffset, EditDetail, FetchDetail, PlainTextDetail, PlanDetail, ReadDetail,
    SearchDetail, SearchMode, SearchSource, ShellDetail, TimelineName, ToolCallDetail, ToolIcon,
    UnknownToolDetail, WriteDetail,
};
use vessel_proto::{HttpUrl, RawJson, ToolKind};

#[derive(Deserialize)]
struct ShellInput {
    command: String,
    #[serde(default)]
    cwd: Option<PathBuf>,
}
#[derive(Deserialize)]
struct FileInput {
    #[serde(alias = "filePath", alias = "path")]
    file_path: PathBuf,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    offset: Option<DetailOffset>,
    #[serde(default)]
    limit: Option<DetailCount>,
    #[serde(default, alias = "oldString")]
    old_string: Option<String>,
    #[serde(default, alias = "newString")]
    new_string: Option<String>,
}
#[derive(Deserialize)]
struct SearchInput {
    #[serde(alias = "pattern")]
    query: String,
    #[serde(default, alias = "output_mode")]
    mode: Option<SearchMode>,
}
#[derive(Deserialize)]
struct FetchInput {
    url: HttpUrl,
    #[serde(default)]
    prompt: Option<String>,
}
#[derive(Deserialize)]
struct PlanInput {
    plan: String,
}
#[derive(Deserialize)]
struct SkillInput {
    skill: TimelineName,
}

fn read<T: DeserializeOwned>(value: &Value) -> Option<T> {
    serde_json::from_value(value.clone()).ok()
}

pub(crate) fn named(name: &str, input: &Value) -> ToolCallDetail {
    let detail = match name {
        "Task" | "Agent" => super::sub_agent_input(input).map(ToolCallDetail::SubAgent),
        "Bash" | "bash" | "shell" | "exec_command" => for_kind(ToolKind::Execute, input),
        "Read" | "read" | "read_file" | "view_file" => for_kind(ToolKind::Read, input),
        "Write" | "write" | "write_file" | "create_file" => read::<FileInput>(input).map(|args| {
            ToolCallDetail::Write(WriteDetail {
                file_path: args.file_path,
                content: args.content,
            })
        }),
        "Edit" | "MultiEdit" | "multi_edit" | "edit" | "apply_patch" | "apply_diff"
        | "str_replace_editor" => for_kind(ToolKind::Edit, input),
        "WebSearch" | "web_search" => search(input, SearchSource::WebSearch),
        "Grep" | "grep" => search(input, SearchSource::Grep),
        "Glob" | "glob" => search(input, SearchSource::Glob),
        "search" => search(input, SearchSource::Search),
        "WebFetch" | "web_fetch" | "WebFetchTool" | "web_fetch_tool" | "webfetch" => {
            for_kind(ToolKind::Fetch, input)
        }
        "ExitPlanMode" => read::<PlanInput>(input)
            .map(|args| ToolCallDetail::Plan(PlanDetail { text: args.plan })),
        "Skill" => read::<SkillInput>(input).map(|args| {
            ToolCallDetail::PlainText(PlainTextDetail {
                label: Some(args.skill),
                text: None,
                icon: Some(ToolIcon::Sparkles),
            })
        }),
        _ => None,
    };
    detail.unwrap_or_else(|| unknown(Some(input), None))
}

pub(crate) fn for_kind(kind: ToolKind, input: &Value) -> Option<ToolCallDetail> {
    match kind {
        ToolKind::Execute => read::<ShellInput>(input).map(|args| {
            ToolCallDetail::Shell(ShellDetail {
                command: args.command,
                cwd: args.cwd,
                output: None,
                exit_code: None,
            })
        }),
        ToolKind::Read => read::<FileInput>(input).map(|args| {
            ToolCallDetail::Read(ReadDetail {
                file_path: args.file_path,
                content: None,
                offset: args.offset,
                limit: args.limit,
            })
        }),
        ToolKind::Edit => read::<FileInput>(input).map(|args| {
            if args.content.is_some() && args.old_string.is_none() && args.new_string.is_none() {
                ToolCallDetail::Write(WriteDetail {
                    file_path: args.file_path,
                    content: args.content,
                })
            } else {
                ToolCallDetail::Edit(EditDetail {
                    file_path: args.file_path,
                    old_string: args.old_string,
                    new_string: args.new_string,
                    unified_diff: None,
                })
            }
        }),
        ToolKind::Search => search(input, SearchSource::Search),
        ToolKind::Fetch => read::<FetchInput>(input).map(|args| {
            ToolCallDetail::Fetch(FetchDetail {
                url: args.url,
                prompt: args.prompt,
                result: None,
                code: None,
                code_text: None,
                bytes: None,
                duration: None,
            })
        }),
        ToolKind::Delete
        | ToolKind::Move
        | ToolKind::Think
        | ToolKind::SwitchMode
        | ToolKind::Other => None,
    }
}

fn search(input: &Value, source: SearchSource) -> Option<ToolCallDetail> {
    let args: SearchInput = read(input)?;
    Some(ToolCallDetail::Search(SearchDetail {
        query: args.query,
        tool_name: Some(source),
        content: None,
        file_paths: None,
        web_results: None,
        annotations: None,
        num_files: None,
        num_matches: None,
        duration: None,
        truncated: None,
        mode: args.mode,
    }))
}

pub(crate) fn unknown(input: Option<&Value>, output: Option<&Value>) -> ToolCallDetail {
    ToolCallDetail::Unknown(UnknownToolDetail {
        input: input.and_then(|value| RawJson::from_serialize(value).ok()),
        output: output.and_then(|value| RawJson::from_serialize(value).ok()),
    })
}
