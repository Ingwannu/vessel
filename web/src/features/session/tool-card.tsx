// A tool call in the timeline (DESIGN.md §6.6): a card whose head is the
// kind's icon, the title, its status and the file it touches; its body
// the input, the content (diffs line by line) and the output. Open while
// the call runs, closed once it has ended. The chevron turns; the
// browser's disclosure triangle is not drawn.

import "./tool-card.css";

import { toolTone } from "../../data/format";
import { blockText, type ToolState } from "../../data/timeline-model";
import { Badge, Icon, type IconName, Tooltip } from "../../ui/kit";
import type { ToolKind } from "../../wire/proto";

function toolIcon(kind: ToolKind): IconName {
  switch (kind) {
    case "read":
      return "read";
    case "edit":
    case "delete":
      return "edit";
    case "move":
      return "move";
    case "search":
      return "search";
    case "execute":
      // The terminal's mark, not a run triangle: beside the card's
      // chevron a triangle reads as the browser's disclosure marker.
      return "terminal";
    case "think":
      return "plan";
    case "fetch":
      return "link";
    case "switch_mode":
    case "other":
      return "more";
  }
}

function json(value: unknown): string {
  return typeof value === "string" ? value : JSON.stringify(value, null, 2);
}

export function ToolCard({ tool }: { tool: ToolState }) {
  const open = tool.status === "pending" || tool.status === "in_progress";
  const path = tool.locations[0]?.path;
  return (
    <details className="tool" open={open}>
      <summary>
        <Icon name={toolIcon(tool.kind)} size={14} />
        <span className="title">{tool.title}</span>
        <Badge tone={toolTone(tool.status)}>
          {tool.status.replace("_", " ")}
        </Badge>
        {path === undefined ? null : (
          <Tooltip label={path}>
            <span className="loc">
              <bdi>{path}</bdi>
            </span>
          </Tooltip>
        )}
        <span className="tool-chevron">
          <Icon name="chevron" size={12} />
        </span>
      </summary>
      <div className="body">
        {tool.input === null ? null : (
          <>
            <div className="io-label">Input</div>
            <pre>{json(tool.input)}</pre>
          </>
        )}
        {tool.content.map((content, index) =>
          content.type === "diff" ? (
            <div key={index}>
              <div className="io-label">Diff · {content.path}</div>
              <div
                className="diff"
                role="figure"
                aria-label={`Diff of ${content.path}`}
              >
                {(content.old_text ?? "").split("\n").map((line, at) =>
                  line === "" ? null : (
                    <div className="ln del" key={`d${String(at)}`}>
                      <span className="n">{at + 1}</span>
                      <span className="n" />
                      <span className="tx">- {line}</span>
                    </div>
                  ),
                )}
                {content.new_text.split("\n").map((line, at) => (
                  <div className="ln add" key={`a${String(at)}`}>
                    <span className="n" />
                    <span className="n">{at + 1}</span>
                    <span className="tx">+ {line}</span>
                  </div>
                ))}
              </div>
            </div>
          ) : (
            <div key={index}>
              <div className="io-label">Content</div>
              <pre>{blockText(content.content)}</pre>
            </div>
          ),
        )}
        {tool.output === null ? null : (
          <>
            <div className="io-label">Output</div>
            <pre>{json(tool.output)}</pre>
          </>
        )}
      </div>
    </details>
  );
}
