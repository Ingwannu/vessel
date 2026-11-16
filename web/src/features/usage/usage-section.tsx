// Paseo usage/usage-section.tsx's distinct read/error states and
// usage/card.tsx's facts, within Vessel's existing Settings rows. No
// account windows are guessed or drawn before their wire is merged.
import "./usage.css";

import { useState } from "react";

import { useFleetState } from "../../context";
import { machineFor, machineState } from "../../data/machines";
import { nodeLabel } from "../../data/nodes";
import { nextStep } from "../../data/outcome";
import type { UsageRange } from "../../data/usage/calls";
import {
  usageCoverage,
  usageFacts,
  type UsageFact,
} from "../../data/usage/facts";
import type { UsageRead } from "../../data/usage/reader";
import {
  MACHINE_USAGE,
  usageTargetFor,
  type UsageTarget,
} from "../../data/usage/scope";
import { Button, Row, RowList, Select, Skeleton } from "../../ui/kit";
import type { NodeKey, NodeStatus } from "../../wire/bridge";
import { MachinePicker } from "../machines/machine-picker";
import {
  FactValue,
  SettingsPage,
  SettingsSection,
} from "../settings/settings-page";
import { PlanUsage } from "./plan-usage";
import { UsageScopePicker } from "./scope-picker";
import { useUsage } from "./use-usage";

const LEAD =
  "Tokens and cost the agents reported on this machine. Context size and model-proxy counts are separate; unreported usage is never estimated.";
const RANGES: readonly { value: UsageRange; label: string }[] = [
  { value: "day", label: "Past 24 hours" },
  { value: "week", label: "Past 7 days" },
];

export function UsageSection({ machine }: { machine: NodeKey | undefined }) {
  const book = useFleetState((state) => state.book);
  const statuses = useFleetState((state) => state.statuses);
  const selected = useFleetState((state) => state.selected);
  const node = machineFor(book ?? [], machine, selected);
  return (
    <SettingsPage lead={node === null ? LEAD : undefined}>
      {node === null ? (
        book === null ? (
          <Skeleton />
        ) : (
          <p className="settings-empty">No machines yet.</p>
        )
      ) : (
        <MachineUsage
          node={node.key}
          label={nodeLabel(node, statuses[node.key])}
        />
      )}
    </SettingsPage>
  );
}

function MachineUsage({ node, label }: { node: NodeKey; label: string }) {
  const [period, setPeriod] = useState<UsageRange>("day");
  const [target, setTarget] = useState<UsageTarget>({
    node,
    scope: MACHINE_USAGE,
  });
  const next = usageTargetFor(target, node);
  // Persist the reset before effects: an effect-only reset could issue
  // the old profile query when returning to the previous machine.
  if (target.node !== node) setTarget(next);
  const scope = next.scope;
  const { status, read, refresh, names } = useUsage(node, period, scope);
  const pending =
    status?.state !== "connected" ||
    read.kind === "idle" ||
    read.kind === "loading";
  return (
    <div className="usage-report">
      <PlanUsage node={node} label={label} />
      <SettingsSection
        title={`Reported usage on ${label}`}
        action={<MachinePicker section="usage" machine={node} />}
      >
        {/* The ledger's sentence sits with the ledger: Paseo's plan usage
            comes first on this page. */}
        <p className="usage-ledger-lead">{LEAD}</p>
        <RowList label="Usage interval">
          <Row
            title="Period"
            control={
              <>
                <Select
                  aria-label="Usage period"
                  size="sm"
                  width="fit"
                  value={period}
                  options={RANGES}
                  onChange={setPeriod}
                />
                <Button size="sm" disabled={pending} onClick={refresh}>
                  Refresh
                </Button>
              </>
            }
          />
        </RowList>
        <UsageScopePicker
          node={node}
          scope={scope}
          names={names}
          onChange={(chosen) => {
            setTarget({ node, scope: chosen });
          }}
        />
      </SettingsSection>
      <UsageBody read={read} status={status} label={label} />
    </div>
  );
}

function UsageBody({
  read,
  status,
  label,
}: {
  read: UsageRead;
  status: NodeStatus | undefined;
  label: string;
}) {
  if (status?.state !== "connected") {
    return (
      <p className="settings-empty">
        {label} is {machineState(status).word.toLowerCase()}; usage is available
        when it reconnects.
      </p>
    );
  }
  switch (read.kind) {
    case "idle":
    case "loading":
      return <Skeleton />;
    case "failed":
      return (
        <p role="alert" className="settings-empty">
          {read.problem.message} {nextStep(read.problem)}
        </p>
      );
    case "ready":
      return (
        <>
          <SettingsSection title="Reported totals">
            <Facts
              label="Reported usage"
              rows={usageFacts(read.report, navigator.language)}
            />
          </SettingsSection>
          <SettingsSection title="Report coverage">
            <Facts
              label="Accounting coverage"
              rows={usageCoverage(read.report, navigator.language)}
            />
          </SettingsSection>
        </>
      );
  }
}

function Facts({ label, rows }: { label: string; rows: readonly UsageFact[] }) {
  return (
    <RowList label={label}>
      {rows.map((fact) => (
        <Row
          key={fact.label}
          title={fact.label}
          {...(fact.detail === null
            ? {}
            : { detail: { description: fact.detail } })}
          control={<FactValue>{fact.value}</FactValue>}
        />
      ))}
    </RowList>
  );
}
