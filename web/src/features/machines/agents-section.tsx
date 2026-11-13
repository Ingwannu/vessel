// Settings › Agents: per machine, every agent Vessel can drive as GOAT's
// provider rows (status, version), and the chosen agent's card under the
// list with what it offers. The machine is the address's, else the
// default (`machineFor`); a picker at the heading changes it.

import { useEffect } from "react";

import { useAgents, useFleet, useFleetState } from "../../context";
import { agentsOn } from "../../data/agent-status";
import { agentKey, agentMark, agentName } from "../../data/agents";
import { machineFor, machineState } from "../../data/machines";
import { nodeLabel } from "../../data/nodes";
import { RowList, Skeleton } from "../../ui/kit";
import { SettingsPage, SettingsSection } from "../settings/settings-page";
import { AgentDetail } from "./agent-detail";
import { AgentRow } from "./agent-row";
import { MachinePicker } from "./machine-picker";
import type { NodeKey } from "../../wire/bridge";

export function AgentsSection({
  machine,
  agent,
}: {
  machine: NodeKey | undefined;
  agent: string | undefined;
}) {
  const book = useFleetState((state) => state.book);
  const statuses = useFleetState((state) => state.statuses);
  const harnessLists = useFleetState((state) => state.harnesses);
  const selected = useFleetState((state) => state.selected);
  const fleet = useFleet();
  const node = machineFor(book ?? [], machine, selected);
  const key = node?.key ?? null;
  const connected = key !== null && statuses[key]?.state === "connected";
  // The agents added on the machine are listed by `agent.list`, read here
  // (the fleet reads it elsewhere only once one shows).
  useEffect(() => {
    if (key !== null && connected) {
      void fleet.loadAgents(key);
    }
  }, [fleet, key, connected]);
  const added = useAgents(key);
  const lead =
    "The agents each machine can run, and what they offer: models, effort and permissions.";
  if (node === null) {
    return (
      <SettingsPage lead={lead}>
        {book === null ? (
          <Skeleton />
        ) : (
          <p className="settings-empty">No machines yet.</p>
        )}
      </SettingsPage>
    );
  }
  const label = nodeLabel(node, statuses[node.key]);
  const harnesses = harnessLists[node.key];
  const agents = harnesses === undefined ? null : agentsOn(harnesses, added);
  const chosen =
    agents?.find((entry) => agentKey(entry.agent) === agent) ?? null;
  return (
    <SettingsPage lead={lead}>
      <SettingsSection
        title={`Agents on ${label}`}
        action={<MachinePicker section="agents" machine={node.key} />}
      >
        {agents === null ? (
          statuses[node.key]?.state === "connected" ? (
            <Skeleton />
          ) : (
            <p className="settings-empty">
              {label} is {machineState(statuses[node.key]).word.toLowerCase()};
              its agents show once it is connected.
            </p>
          )
        ) : (
          <RowList label={`Agents on ${label}`}>
            {agents.map((entry) => (
              <AgentRow
                key={agentKey(entry.agent)}
                machine={node.key}
                agent={entry.agent}
                name={agentName(entry.agent, added)}
                mark={agentMark(entry.agent, added)}
                status={entry.status}
                selected={entry === chosen}
              />
            ))}
          </RowList>
        )}
      </SettingsSection>
      {chosen === null ? null : (
        <AgentDetail
          machine={node.key}
          label={label}
          agent={chosen.agent}
          name={agentName(chosen.agent, added)}
          status={chosen.status}
        />
      )}
    </SettingsPage>
  );
}
