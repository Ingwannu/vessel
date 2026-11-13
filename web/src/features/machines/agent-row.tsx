// One agent on a machine (Settings › Agents): GOAT's provider row. The
// agent's letter, its name and version on the first line, a status dot
// and the headline (with the machine's reason when it needs attention)
// on the second. The name opens its card below the list.

import "./agent-row.css";

import { Link } from "@tanstack/react-router";

import { agentHeadline, type AgentStatus } from "../../data/agent-status";
import { agentKey } from "../../data/agents";
import type { HarnessRef } from "../../wire/proto";
import { Avatar, Row, StatusDot } from "../../ui/kit";

export function AgentRow({
  machine,
  agent,
  name,
  mark,
  status,
  selected,
}: {
  machine: string;
  agent: HarnessRef;
  /** What the page calls it (an added agent's label), and its letter. */
  name: string;
  mark: string;
  status: AgentStatus;
  selected: boolean;
}) {
  const shown = agentHeadline(status);
  const version = status.kind === "not-found" ? null : status.harness.version;
  return (
    <Row
      selected={selected}
      mark={{
        node: <Avatar letter={mark} label={name} />,
      }}
      title={
        <span className="agent-title">
          <Link
            to="/settings/$section"
            params={{ section: "agents" }}
            search={
              selected ? { machine } : { machine, agent: agentKey(agent) }
            }
            aria-current={selected ? "true" : undefined}
          >
            {name}
          </Link>
          {version === null ? null : (
            <code className="agent-version">{version}</code>
          )}
        </span>
      }
      detail={{
        description: (
          <span className="agent-status">
            <StatusDot tone={shown.tone} label={shown.headline} />
            <span>
              {shown.headline}
              {shown.detail === null ? "" : ` · ${shown.detail}`}
            </span>
          </span>
        ),
      }}
    />
  );
}
