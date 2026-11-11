// One entry of the timeline (DESIGN.md §6.5): a log, left-aligned, not
// two columns of chat bubbles. A permission request is one quiet line
// that becomes its outcome; the prompt to answer it is docked over the
// composer (permission-dock.tsx).

import type { ReactNode } from "react";

import { settled, settledLine } from "../../data/approval";
import { when } from "../../data/format";
import { Markdown } from "../../data/markdown";
import { apiName, callParts } from "../../data/models";
import type { Entry } from "../../data/timeline-model";
import { Icon, Tooltip } from "../../ui/kit";
import { ToolCard } from "./tool-card";

function Meta({
  className = "t-item meta-item",
  children,
}: {
  className?: string;
  children: ReactNode;
}) {
  return (
    <div className={className}>
      <span className="glyph" aria-hidden="true" />
      {children}
    </div>
  );
}

export function TimelineItem({ entry }: { entry: Entry }) {
  switch (entry.kind) {
    case "ready":
      return (
        <Meta>
          <p className="t-meta">Agent ready</p>
        </Meta>
      );
    case "prompt":
      return (
        <div className="t-item user" data-entry={entry.key} data-kind="prompt">
          <span className="glyph" aria-hidden="true" />
          <span className="when">
            {entry.replayed ? "replayed" : `turn ${String(entry.turn)}`}
          </span>
          <div className="prompt">{entry.text}</div>
        </div>
      );
    case "message":
      return (
        <div className="t-item" data-entry={entry.key} data-kind="message">
          <span className="glyph" aria-hidden="true" />
          <div className="md">
            <Markdown>{entry.text}</Markdown>
          </div>
        </div>
      );
    case "thought":
      return (
        <div className="t-item">
          <span className="glyph" aria-hidden="true" />
          <details className="thought">
            <summary>
              <Icon name="chevron" size={12} /> Thought
            </summary>
            <p>{entry.text}</p>
          </details>
        </div>
      );
    case "tool":
      return (
        <div className="t-item tool">
          <span className="glyph" aria-hidden="true" />
          <ToolCard tool={entry.tool} />
        </div>
      );
    case "plan": {
      const done = entry.entries.filter(
        (item) => item.status === "completed",
      ).length;
      return (
        <Meta>
          <p className="t-meta">
            Plan · {done} of {entry.entries.length} done
          </p>
        </Meta>
      );
    }
    case "permission": {
      const state = settled(entry.request.options, entry.outcome);
      return (
        <Meta className={`t-item meta-item perm ${state.state}`}>
          <p className="t-meta">{settledLine(entry.title, state)}</p>
        </Meta>
      );
    }
    case "turn_ended":
      return (
        <Meta>
          <p className="t-meta">
            Turn ended · {entry.reason.replace("_", " ")}
          </p>
        </Meta>
      );
    case "turn_failed":
      return (
        <Meta>
          <p className="t-meta error">Turn failed · {entry.message}</p>
        </Meta>
      );
    case "log":
      return (
        <Meta>
          <p className="t-meta">
            <code>{entry.line}</code>
          </p>
        </Meta>
      );
    case "status":
      return (
        <Meta>
          <p className="t-meta">
            Session {entry.status.state} · {when(entry.at)}
          </p>
        </Meta>
      );
    case "profile":
      return (
        <Meta>
          <p className="t-meta">
            Profile changed from {entry.from ?? "default"} to{" "}
            {entry.to ?? "default"}
          </p>
        </Meta>
      );
    case "model": {
      const call = entry.call;
      const failed = call.failure !== undefined && call.failure !== null;
      return (
        <Meta className="t-item meta-item model-call">
          <Tooltip label={`${apiName(call.api)} · ${when(entry.at)}`}>
            <p className={failed ? "t-meta error" : "t-meta"}>
              Model call · <span className="mono">{call.route}</span> ·{" "}
              <span className="mono">{call.model}</span> ·{" "}
              {callParts(call).join(" · ")}
            </p>
          </Tooltip>
        </Meta>
      );
    }
  }
}
