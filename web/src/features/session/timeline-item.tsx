// One entry of the timeline (DESIGN.md §6.5): a log, left-aligned, not
// two columns of chat bubbles. A permission request is one quiet line
// that becomes its outcome; the prompt to answer it is docked over the
// composer (permission-dock.tsx).

import { memo, type ReactNode } from "react";

import { commandStatus } from "../../data/native-command";
import { settled, settledLine } from "../../data/approval";
import { turnTokens } from "../../data/context-meter";
import { when } from "../../data/format";
import { Markdown } from "../../data/markdown";
import { apiName, callParts } from "../../data/models";
import type { Entry } from "../../data/timeline-model";
import { isInternalInput } from "../../data/timeline-input";
import { Icon, Tooltip } from "../../ui/kit";
import { PromptFiles } from "./prompt-files";
import { QuestionEntry } from "./question-entry";
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

/** One entry, drawn again only when its entry changes: the reducer keeps
 * the object of every entry a batch does not touch (data/timeline.ts), so
 * a batch re-renders the entries it changed, not the whole log
 * (docs/PERFORMANCE.md). */
export const TimelineItem = memo(function TimelineItem({
  entry,
}: {
  entry: Entry;
}) {
  return <Item entry={entry} />;
});

function Item({ entry }: { entry: Entry }) {
  if (isInternalInput(entry)) return null;
  switch (entry.kind) {
    case "command":
      return (
        <Meta>
          <p>{entry.text}</p>
          <p className="t-meta">{commandStatus(entry.item.status)}</p>
        </Meta>
      );
    case "ready":
      return (
        <Meta>
          <p className="t-meta">Agent ready</p>
        </Meta>
      );
    case "prompt":
      return (
        <div
          className="t-item user"
          data-entry={entry.key}
          data-kind="prompt"
          // The outline's jump gives a prompt focus (reveal.ts); Tab
          // does not stop here.
          tabIndex={-1}
        >
          <span className="glyph" aria-hidden="true" />
          <span className="when">
            {entry.sending === true
              ? "sending"
              : entry.replayed
                ? "replayed"
                : `turn ${String(entry.turn)}`}
          </span>
          <PromptFiles files={entry.attachments} />
          {entry.text === "" ? null : (
            <div className="prompt">{entry.text}</div>
          )}
        </div>
      );
    case "steer":
      return (
        <div className="t-item user steer" data-entry={entry.key}>
          <span className="glyph" aria-hidden="true" />
          <span className="when">steered</span>
          <PromptFiles files={entry.attachments} />
          {entry.text === "" ? null : (
            <div className="prompt">{entry.text}</div>
          )}
        </div>
      );
    case "question":
      return <QuestionEntry entry={entry} />;
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
          <ul>
            {entry.entries.map((item, index) => (
              <li key={item.id ?? index}>
                {item.status === "in_progress"
                  ? (item.active_form ?? item.content)
                  : item.content}{" "}
                · {item.status.replaceAll("_", " ")}
              </li>
            ))}
          </ul>
        </Meta>
      );
    }
    case "permission": {
      if (entry.available === false && entry.outcome === null)
        return (
          <Meta>
            <p className="t-meta">Request no longer pending · {entry.title}</p>
          </Meta>
        );
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
            {entry.usage === null ? null : (
              <span className="turn-tokens"> · {turnTokens(entry.usage)}</span>
            )}
          </p>
        </Meta>
      );
    case "turn_failed":
      return (
        <Meta>
          <p className="t-meta error">Turn failed · {entry.message}</p>
        </Meta>
      );
    case "notification":
      return (
        <Meta>
          <p
            className={`t-meta ${entry.item.level === "error" ? "error" : ""}`}
          >
            {entry.item.message}
          </p>
        </Meta>
      );
    case "error":
      return (
        <Meta>
          <p className="t-meta error">{entry.message}</p>
        </Meta>
      );
    case "compaction":
      return (
        <Meta>
          <p className="t-meta">
            {entry.item.status === "loading"
              ? "Compacting context…"
              : "Context compacted"}
            {entry.item.trigger === undefined || entry.item.trigger === null
              ? ""
              : ` · ${entry.item.trigger}`}
            {entry.item.pre_tokens === undefined ||
            entry.item.pre_tokens === null
              ? ""
              : ` · ${String(entry.item.pre_tokens)} tokens before`}
          </p>
        </Meta>
      );
    case "plugin":
      return (
        <Meta>
          <details>
            <summary>
              {entry.item.plugin} · {entry.item.kind} · v{entry.item.version}
            </summary>
            <pre>{JSON.stringify(entry.item.data, null, 2)}</pre>
          </details>
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
