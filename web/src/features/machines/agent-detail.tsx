// An agent's card, opened from its row (Settings › Agents): its status,
// version and path, then what it offers, asked of the machine when the
// card opens (`harness.options`). Opening a card starts the agent once on
// that machine to ask, so it is never asked for every agent at once, and
// the card says so while it waits.

import { useEffect, useState } from "react";

import { useFleet } from "../../context";
import {
  agentHeadline,
  agentName,
  type AgentStatus,
  loadOffer,
  offerGroupLabel,
  offerGroups,
} from "../../data/agents";
import { ago } from "../../data/format";
import { nextStep, type Outcome } from "../../data/outcome";
import type { NodeKey } from "../../wire/bridge";
import type { HarnessKind, HarnessOptions } from "../../wire/proto";
import { Badge, Banner, Row, RowList, Skeleton } from "../../ui/kit";
import { FactValue, SettingsSection } from "../settings/settings-page";

export function AgentDetail({
  machine,
  label,
  kind,
  status,
}: {
  machine: NodeKey;
  /** The machine's name. */
  label: string;
  kind: HarnessKind;
  status: AgentStatus;
}) {
  const shown = agentHeadline(status);
  const name = agentName(kind);
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
          key={`${machine}/${kind}`}
          machine={machine}
          label={label}
          kind={kind}
        />
      ) : null}
    </>
  );
}

/** What the agent offers, asked when this mounts. */
function Offer({
  machine,
  label,
  kind,
}: {
  machine: NodeKey;
  label: string;
  kind: HarnessKind;
}) {
  const fleet = useFleet();
  const [answer, setAnswer] = useState<Outcome<HarnessOptions> | null>(null);
  useEffect(() => {
    let current = true;
    void loadOffer(fleet, machine, kind).then((outcome) => {
      if (current) {
        setAnswer(outcome);
      }
    });
    return () => {
      current = false;
    };
  }, [fleet, machine, kind]);
  const name = agentName(kind);
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
          {name} did not say: {answer.problem.message}{" "}
          {nextStep(answer.problem)}
        </Banner>
      </SettingsSection>
    );
  }
  const groups = offerGroups(answer.value.options);
  if (groups.length === 0) {
    return (
      <SettingsSection title="What it offers">
        <p className="settings-empty">
          {name} offers no choices of model, effort or permissions.
        </p>
      </SettingsSection>
    );
  }
  return (
    <>
      {groups.map(({ group, rows }, index) => (
        <SettingsSection
          key={group}
          title={offerGroupLabel(group)}
          {...(index === 0
            ? {
                action: (
                  <span className="section-note">
                    Asked {ago(answer.value.probed_at)} ago
                  </span>
                ),
              }
            : {})}
        >
          <RowList label={`${name}: ${offerGroupLabel(group)}`}>
            {rows.map((row) => (
              <Row
                key={row.id}
                title={row.name}
                detail={{ facts: [row.values.join(", ")] }}
                control={<FactValue>Now {row.current}</FactValue>}
              />
            ))}
          </RowList>
        </SettingsSection>
      ))}
    </>
  );
}
