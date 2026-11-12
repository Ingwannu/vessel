// A machine's detail, below the list when its row is chosen (what the
// Fleet page's inspector was): what it is, how it is reached, its agents
// and its live sessions. Literal values (an address, an id) are in
// detail rows; everything else is in the page's words.

import { Link } from "@tanstack/react-router";

import { useFleet, useFleetState } from "../../context";
import { agentHeadline, agentsOn } from "../../data/agent-status";
import { agentName } from "../../data/agents";
import { retryMachine } from "../../data/book";
import { isLive, shortId, statusWord, when } from "../../data/format";
import { isHere, machineState, platform, reachWord } from "../../data/machines";
import { nodeLabel } from "../../data/nodes";
import { sessionName } from "../../data/titles";
import type { BookNode } from "../../wire/bridge";
import { Badge, Button, Row, RowList } from "../../ui/kit";
import { toast } from "../../ui/toasts";
import { FactValue, SettingsSection } from "../settings/settings-page";

/** Live sessions listed here before "all on the board". */
const SESSIONS_SHOWN = 6;

export function MachineDetail({ node }: { node: BookNode }) {
  const fleet = useFleet();
  const status = useFleetState((state) => state.statuses[node.key]);
  const harnesses = useFleetState((state) => state.harnesses[node.key]);
  const sessions = useFleetState((state) => state.sessions[node.key]);
  const label = nodeLabel(node, status);
  const state = machineState(status);
  const info = status?.state === "connected" ? status.node : null;
  const down = status?.state === "failed" || status?.state === "reconnecting";
  const live = (sessions ?? []).filter(isLive);
  return (
    <>
      <SettingsSection
        title={label}
        action={
          down ? (
            <Button
              size="xs"
              variant="ghost"
              onClick={() => {
                void retryMachine(fleet, node.key).then((outcome) => {
                  if (!outcome.ok) {
                    toast(outcome.problem.message, "error");
                  }
                });
              }}
            >
              Try again now
            </Button>
          ) : undefined
        }
      >
        <RowList label={`About ${label}`}>
          <Row
            title="State"
            {...(status?.state === "failed" || status?.state === "reconnecting"
              ? { detail: { description: status.message } }
              : {})}
            control={<Badge tone={state.tone}>{state.word}</Badge>}
          />
          <Row
            title="Reached"
            detail={{
              description: isHere(node.key)
                ? "The machine this page was opened from."
                : reachWord(node),
            }}
            control={<FactValue mono>{node.address}</FactValue>}
          />
          {info === null ? null : (
            <>
              <Row
                title="Runs on"
                detail={{ description: info.hostname }}
                control={<FactValue>{platform(status)}</FactValue>}
              />
              <Row
                title="Vessel"
                detail={{
                  description: `Started ${when(info.started_at)}`,
                }}
                control={
                  <FactValue mono>
                    {info.version} · protocol{" "}
                    {status?.state === "connected" ? status.protocol : ""}
                  </FactValue>
                }
              />
            </>
          )}
          <Row
            title="Id"
            control={
              <FactValue mono>
                {info?.id ?? node.id ?? "not seen yet"}
              </FactValue>
            }
          />
        </RowList>
      </SettingsSection>
      {harnesses === undefined ? null : (
        <SettingsSection
          title="Agents"
          action={
            <Link
              className="section-link"
              to="/settings/$section"
              params={{ section: "agents" }}
              search={{ machine: node.key }}
            >
              What they offer
            </Link>
          }
        >
          <RowList label={`Agents on ${label}`}>
            {agentsOn(harnesses).map(({ kind, status: agent }) => {
              const shown = agentHeadline(agent);
              return (
                <Row
                  key={kind}
                  title={
                    <Link
                      to="/settings/$section"
                      params={{ section: "agents" }}
                      search={{ machine: node.key, agent: kind }}
                    >
                      {agentName(kind)}
                    </Link>
                  }
                  detail={{
                    facts: [
                      agent.kind === "not-found"
                        ? ""
                        : (agent.harness.version ?? "version unknown"),
                      shown.detail ?? "",
                    ],
                  }}
                  control={<Badge tone={shown.tone}>{shown.headline}</Badge>}
                />
              );
            })}
          </RowList>
        </SettingsSection>
      )}
      {sessions === undefined ? null : (
        <SettingsSection
          title="Live sessions"
          action={
            <Link
              className="section-link"
              to="/sessions"
              search={{ node: node.key }}
            >
              Open the board
            </Link>
          }
        >
          {live.length === 0 ? (
            <p className="settings-empty">Nothing is running on {label}.</p>
          ) : (
            <RowList label={`Live sessions on ${label}`}>
              {live.slice(0, SESSIONS_SHOWN).map((session) => (
                <Row
                  key={session.id}
                  title={
                    <Link
                      to="/sessions/$node/$id"
                      params={{ node: node.key, id: session.id }}
                    >
                      {sessionName(session)}
                    </Link>
                  }
                  detail={{
                    facts: [
                      agentName(session.spec.harness),
                      shortId(session.id),
                      statusWord(session.status),
                    ],
                  }}
                />
              ))}
            </RowList>
          )}
        </SettingsSection>
      )}
    </>
  );
}
