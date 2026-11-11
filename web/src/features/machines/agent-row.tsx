// One agent on a machine (Settings › Agents): GOAT's provider row. The
// agent's letter, its name and version on the first line, a status dot
// and the headline (with the machine's reason when it needs attention)
// on the second. The name opens its card below the list.

import "./agent-row.css";

import { Link } from "@tanstack/react-router";

import { agentHeadline, type AgentStatus } from "../../data/agent-status";
import { agentMark, agentName } from "../../data/agents";
import type { HarnessKind } from "../../wire/proto";
import { Avatar, Row, StatusDot } from "../../ui/kit";

export function AgentRow({
  machine,
  kind,
  status,
  selected,
}: {
  machine: string;
  kind: HarnessKind;
  status: AgentStatus;
  selected: boolean;
}) {
  const shown = agentHeadline(status);
  const version = status.kind === "not-found" ? null : status.harness.version;
  return (
    <Row
      selected={selected}
      mark={{
        node: <Avatar letter={agentMark(kind)} label={agentName(kind)} />,
      }}
      title={
        <span className="agent-title">
          <Link
            to="/settings/$section"
            params={{ section: "agents" }}
            search={selected ? { machine } : { machine, agent: kind }}
            aria-current={selected ? "true" : undefined}
          >
            {agentName(kind)}
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
