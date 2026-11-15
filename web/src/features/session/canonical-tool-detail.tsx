// Dedicated semantic bodies; opaque payloads are only serialized for inspection.
import { Markdown } from "../../data/markdown";
import type { ToolCallDetail } from "../../wire/proto";

function Text({
  label,
  value,
}: {
  label: string;
  value: string | number | null | undefined;
}) {
  return value === undefined || value === null ? null : (
    <section>
      <div className="io-label">{label}</div>
      <pre>{value}</pre>
    </section>
  );
}
export function CanonicalToolDetail({ detail }: { detail: ToolCallDetail }) {
  switch (detail.type) {
    case "shell":
      return (
        <>
          <Text label="Command" value={detail.command} />
          <Text label="Directory" value={detail.cwd} />
          <Text label="Output" value={detail.output} />
          <Text label="Exit code" value={detail.exit_code} />
        </>
      );
    case "read":
      return (
        <>
          <Text label="File" value={detail.file_path} />
          <Text label="Offset" value={detail.offset} />
          <Text label="Limit" value={detail.limit} />
          <Text label="Content" value={detail.content} />
        </>
      );
    case "write":
      return (
        <>
          <Text label="File" value={detail.file_path} />
          <Text label="Content" value={detail.content} />
        </>
      );
    case "edit":
      return (
        <>
          <Text label="File" value={detail.file_path} />
          <Text label="Before" value={detail.old_string} />
          <Text label="After" value={detail.new_string} />
          <Text label="Diff" value={detail.unified_diff} />
        </>
      );
    case "search":
      return (
        <>
          <Text label="Query" value={detail.query} />
          <Text label="Content" value={detail.content} />
          <Text label="Files" value={detail.file_paths?.join("\n")} />
          <Text label="Matched files" value={detail.num_files} />
          <Text label="Matches" value={detail.num_matches} />
          {detail.web_results?.map((result, index) => (
            <p key={index}>
              {result.title} · {result.url}
            </p>
          ))}
        </>
      );
    case "fetch":
      return (
        <>
          <Text label="URL" value={detail.url} />
          <Text label="Instructions" value={detail.prompt} />
          <Text label="Result" value={detail.result} />
          <Text label="Status" value={detail.code} />
          <Text label="Status text" value={detail.code_text} />
          <Text label="Bytes" value={detail.bytes} />
        </>
      );
    case "worktree_setup":
      return (
        <>
          <Text label="Worktree" value={detail.worktree_path} />
          <Text label="Branch" value={detail.branch_name} />
          <Text label="Log" value={detail.log} />
          {detail.commands.map((command) => (
            <section key={command.index}>
              <Text label={command.status} value={command.command} />
              <Text label="Directory" value={command.cwd} />
              <Text label="Output" value={command.log} />
              <Text label="Exit code" value={command.exit_code} />
            </section>
          ))}
        </>
      );
    case "sub_agent":
      return (
        <>
          <Text label="Agent type" value={detail.sub_agent_type} />
          <Text label="Task" value={detail.description} />
          <Text label="Provider conversation" value={detail.child_session_id} />
          <Text label="Log" value={detail.log} />
          {detail.actions?.map((action) => (
            <Text
              key={action.index}
              label={action.tool_name}
              value={action.summary}
            />
          ))}
        </>
      );
    case "plain_text":
      return <Text label={detail.label ?? "Output"} value={detail.text} />;
    case "plan":
      return (
        <div className="md">
          <Markdown>{detail.text}</Markdown>
        </div>
      );
    case "unknown":
      return (
        <>
          <Text
            label="Input"
            value={
              detail.input === undefined
                ? undefined
                : JSON.stringify(detail.input, null, 2)
            }
          />
          <Text
            label="Output"
            value={
              detail.output === undefined
                ? undefined
                : JSON.stringify(detail.output, null, 2)
            }
          />
        </>
      );
  }
}
