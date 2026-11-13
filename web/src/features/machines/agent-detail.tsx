// An agent's card, opened from its row (Settings › Agents): its status,
// version and path, then what it offers, asked of the machine when the
// card opens (`harness.options`). Opening a card starts the agent once on
// that machine to ask, so it is never asked for every agent at once, and
// the card says so while it waits.

import { useEffect, useRef, useState } from "react";

import { useAgents, useFleet } from "../../context";
import { refusedStart } from "../../data/agent-choices";
import { nodeError } from "../../data/workspace/root";
import {
  agentHeadline,
  type AgentStatus,
  askedWhen,
  offerRows,
} from "../../data/agent-status";
import { probeOptions } from "../../data/calls";
import { controlsOf } from "../../data/options";
import { nextStep, type Outcome } from "../../data/outcome";
import type { NodeKey } from "../../wire/bridge";
import { agentKey } from "../../data/agents";
import type { HarnessOptions, HarnessRef } from "../../wire/proto";
import { Badge, Banner, Row, RowList, Skeleton } from "../../ui/kit";
import { FactValue, SettingsSection } from "../settings/settings-page";

export function AgentDetail({
  machine,
  label,
  agent,
  name,
  status,
}: {
  machine: NodeKey;
  /** The machine's name. */
  label: string;
  agent: HarnessRef;
  /** What the page calls the agent (an added agent's label). */
  name: string;
  status: AgentStatus;
}) {
  const shown = agentHeadline(status);
  return (
    <>
      <SettingsSection title={`${name} on ${label}`}>
        <RowList label={`${name} on ${label}`}>
          <Row
            title="Status"
            {...(shown.detail === null
              ? {}
              : { detail: { description: shown.detail } })}
            control={<Badge tone={shown.tone}>{shown.headline}</Badge>}
          />
          {status.kind === "not-found" ? null : (
            <>
              <Row
                title="Version"
                control={
                  <FactValue mono>
                    {status.harness.version ?? "did not say"}
                  </FactValue>
                }
              />
              <Row
                title="Program"
                control={<FactValue mono>{status.harness.path}</FactValue>}
              />
            </>
          )}
        </RowList>
      </SettingsSection>
      {status.kind === "available" ? (
        <Offer
          key={`${machine}/${agentKey(agent)}`}
          machine={machine}
          label={label}
          agent={agent}
          name={name}
        />
      ) : null}
    </>
  );
}

/** What the agent offers, asked when this mounts. */
function Offer({
  machine,
  label,
  agent,
  name,
}: {
  machine: NodeKey;
  label: string;
  agent: HarnessRef;
  name: string;
}) {
  const fleet = useFleet();
  const agents = useAgents(machine);
  const [answer, setAnswer] = useState<Outcome<HarnessOptions> | null>(null);
  // The agent this card was mounted for (its parent keys it by the
  // agent's key): an added agent is a new object each time the machine's
  // lists are read, which must not start the agent there again.
  const asked = useRef(agent);
  useEffect(() => {
    let current = true;
    void probeOptions(fleet, machine, asked.current, null).then((outcome) => {
      if (current) {
        setAnswer(outcome);
      }
    });
    return () => {
      current = false;
    };
  }, [fleet, machine]);
  if (answer === null) {
    return (
      <SettingsSection title="What it offers">
        <p className="settings-empty" role="status">
          Asking {name} on {label}. This starts it once there; no prompt is
          sent.
        </p>
        <Skeleton />
      </SettingsSection>
    );
  }
  if (!answer.ok) {
    return (
      <SettingsSection title="What it offers">
        <Banner tone="error">
          {nodeError(answer.problem) === "conflict"
            ? refusedStart(agent, agents)
            : `${name} did not say: ${answer.problem.message} ${nextStep(answer.problem)}`}
        </Banner>
      </SettingsSection>
    );
  }
  const rows = offerRows(controlsOf(answer.value.options, name));
  return (
    <SettingsSection
      title="What it offers"
      action={
        <span className="section-note">
          {askedWhen(answer.value.probed_at)}
        </span>
      }
    >
      {rows.length === 0 ? (
        <p className="settings-empty">
          {name} offers no choices of model, effort or permissions.
        </p>
      ) : (
        <RowList label={`What ${name} offers`}>
          {rows.map((row) => (
            <Row
              key={row.id}
              title={row.label}
              detail={{ facts: [row.values.join(", ")] }}
              control={<FactValue>Now {row.current}</FactValue>}
            />
          ))}
        </RowList>
      )}
    </SettingsSection>
  );
}
