// The session's summary beside its timeline (agent-orchestrator's
// inspector, its Summary tab): collapsible sections for the plan, the
// files the agent touched and the model calls, then the identifiers
// behind "Details", closed, instead of a wall of fields.

import "./inspector.css";

import { agentName } from "../../data/agents";
import { startedModel } from "../../data/models";
import { shortId, when } from "../../data/format";
import { duration, tokens } from "../../data/models";
import type { Timeline } from "../../data/timeline-model";
import { Collapsible, Icon, Tooltip } from "../../ui/kit";
import type { SessionInfo } from "../../wire/proto";

function Plan({ timeline }: { timeline: Timeline }) {
  const plan = timeline.plan;
  if (plan === null) {
    return <p className="inspector-empty">No plan yet.</p>;
  }
  return (
    <ul className="plan-list">
      {plan.map((item, index) => (
        <li key={index} className={`plan-item ${item.status}`}>
          <span className="mark-box" aria-hidden="true">
            {item.status === "completed" ? (
              <Icon name="check" size={12} />
            ) : null}
          </span>
          <span>{item.content}</span>
        </li>
      ))}
    </ul>
  );
}

function Files({ files }: { files: readonly string[] }) {
  if (files.length === 0) {
    return <p className="inspector-empty">No files touched yet.</p>;
  }
  return (
    <ul className="file-list">
      {files.map((path) => (
        <li key={path}>
          <Tooltip label={path}>
            <span className="file-row">
              <Icon name="read" size={14} />
              <span className="file-name">{path.split("/").at(-1)}</span>
            </span>
          </Tooltip>
        </li>
      ))}
    </ul>
  );
}

/** Label and value rows. */
function Facts({ rows }: { rows: readonly [string, string][] }) {
  return (
    <dl className="facts">
      {rows.map(([label, value]) => (
        <div key={label} className="fact">
          <dt>{label}</dt>
          <dd>{value}</dd>
        </div>
      ))}
    </dl>
  );
}

/** A row only when `value` is there. */
function optional(
  label: string,
  value: string | null | undefined,
): [string, string][] {
  return value === null || value === undefined ? [] : [[label, value]];
}

function startedBy(session: SessionInfo): string {
  switch (session.created_by.kind) {
    case "agent":
      return `the agent of ${shortId(session.created_by.session)}`;
    case "device":
      return "a paired device";
    case "owner":
      return "you";
  }
}

export function Inspector({
  label,
  session,
  timeline,
}: {
  /** The machine's name. */
  label: string;
  session: SessionInfo;
  timeline: Timeline;
}) {
  const plan = timeline.plan;
  const usage = timeline.usage;
  const done =
    plan === null
      ? null
      : `${String(plan.filter((item) => item.status === "completed").length)} of ${String(plan.length)}`;
  const route = session.model?.route ?? null;
  return (
    <aside
      className="inspector"
      role="complementary"
      aria-label="Session summary"
    >
      <header className="inspector-head">Summary</header>
      <div className="inspector-body">
        <Collapsible title="Plan" {...(done === null ? {} : { end: done })}>
          <Plan timeline={timeline} />
        </Collapsible>
        <Collapsible
          title="Files"
          {...(timeline.files.length === 0
            ? {}
            : { end: String(timeline.files.length) })}
        >
          <Files files={timeline.files} />
        </Collapsible>
        {usage === null ? null : (
          <Collapsible title="Model calls" end={String(usage.calls)}>
            <Facts
              rows={[
                [
                  "Calls",
                  `${String(usage.calls)}${usage.failed === 0 ? "" : ` · ${String(usage.failed)} failed`}`,
                ],
                [
                  "Input",
                  `${tokens(usage.input + usage.cacheRead + usage.cacheWrite)}${usage.cacheRead + usage.cacheWrite === 0 ? "" : ` · ${tokens(usage.cacheRead)} cached`}`,
                ],
                [
                  "Output",
                  `${tokens(usage.output)}${usage.reasoning === 0 ? "" : ` · ${tokens(usage.reasoning)} reasoning`}`,
                ],
                ["Time", duration(usage.ms)],
                ["Models", usage.models.join(", ")],
              ]}
            />
          </Collapsible>
        )}
        <Collapsible title="Details" defaultOpen={false}>
          <Facts
            rows={[
              ["Machine", label],
              ["Agent", agentName(session.spec.harness)],
              ["Profile", session.profile ?? "Default"],
              ...optional("Model route", route),
              ...optional("Model", startedModel(session)),
              ["Folder", session.spec.cwd ?? "home"],
              ["Started", when(session.created_at)],
              ["Started by", startedBy(session)],
              ["Session", session.id],
              ...optional("Agent's session", session.harness_session),
              ["Records", String(timeline.next)],
            ]}
          />
        </Collapsible>
      </div>
    </aside>
  );
}
