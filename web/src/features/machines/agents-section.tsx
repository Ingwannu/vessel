// Settings › Agents: per machine, every agent Vessel can drive as GOAT's
// provider rows (status, version), and the chosen agent's card under the
// list with what it offers. The machine is the address's, else the
// default (`machineFor`); a picker at the heading changes it.

import { useFleetState } from "../../context";
import { agentKindOf, agentsOn } from "../../data/agents";
import { machineFor, machineState } from "../../data/machines";
import { nodeLabel } from "../../data/nodes";
import { RowList, Skeleton } from "../../ui/kit";
import { SettingsPage, SettingsSection } from "../settings/settings-page";
import { AgentDetail } from "./agent-detail";
import { AgentRow } from "./agent-row";
import { MachinePicker } from "./machine-picker";

export function AgentsSection({
  machine,
  agent,
}: {
  machine: string | undefined;
  agent: string | undefined;
}) {
  const book = useFleetState((state) => state.book);
  const statuses = useFleetState((state) => state.statuses);
  const harnessLists = useFleetState((state) => state.harnesses);
  const node = machineFor(book ?? [], statuses, machine);
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
  const open = agentKindOf(agent);
  const agents = harnesses === undefined ? null : agentsOn(harnesses);
  const chosen = agents?.find((entry) => entry.kind === open) ?? null;
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
                key={entry.kind}
                machine={node.key}
                kind={entry.kind}
                status={entry.status}
                selected={entry.kind === chosen?.kind}
              />
            ))}
          </RowList>
        )}
      </SettingsSection>
      {chosen === null ? null : (
        <AgentDetail
          machine={node.key}
          label={label}
          kind={chosen.kind}
          status={chosen.status}
        />
      )}
    </SettingsPage>
  );
}
