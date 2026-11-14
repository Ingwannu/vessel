// Paseo usage/usage-section.tsx's distinct loading/error/empty states and
// usage/card.tsx's facts, within Vessel's existing Settings rows. This
// first surface reads reported accounting only: account windows are not
// guessed from context occupancy or added before their wire is merged.
import { useEffect, useMemo, useState } from "react";

import { useFleet, useFleetState } from "../../context";
import { machineFor, machineState } from "../../data/machines";
import { nodeLabel } from "../../data/nodes";
import { nextStep } from "../../data/outcome";
import { useStore } from "../../data/store";
import { readUsage, type UsageRange } from "../../data/usage/calls";
import { usageFacts, usageTime } from "../../data/usage/facts";
import { UsageReader } from "../../data/usage/reader";
import { Button, Row, RowList, Select, Skeleton } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import { MachinePicker } from "../machines/machine-picker";
import {
  FactValue,
  SettingsPage,
  SettingsSection,
} from "../settings/settings-page";

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
    <SettingsPage lead={LEAD}>
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
  const fleet = useFleet();
  const status = useFleetState((state) => state.statuses[node]);
  const connected = status?.state === "connected";
  const [period, setPeriod] = useState<UsageRange>("day");
  const reader = useMemo(
    () =>
      new UsageReader(({ node: machine, period: range }) =>
        readUsage(fleet, machine, range),
      ),
    [fleet],
  );
  const state = useStore(reader.store, (value) => value);
  useEffect(() => {
    if (connected) reader.request({ node, period });
    return () => {
      reader.stop();
    };
  }, [reader, node, period, connected]);
  const current =
    state.kind !== "idle" &&
    state.selection.node === node &&
    state.selection.period === period;
  const ready =
    connected && current && state.kind === "ready" ? state.report : null;
  const problem =
    connected && current && state.kind === "failed" ? state.problem : null;
  const loading = connected && ready === null && problem === null;
  const locale = navigator.language;
  return (
    <>
      <SettingsSection
        title={`Reported usage on ${label}`}
        action={<MachinePicker section="usage" machine={node} />}
      >
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
                <Button
                  size="sm"
                  disabled={!connected || loading}
                  onClick={() => {
                    reader.request({ node, period });
                  }}
                >
                  Refresh
                </Button>
              </>
            }
          />
        </RowList>
      </SettingsSection>
      {!connected ? (
        <p className="settings-empty">
          {label} is {machineState(status).word.toLowerCase()}; usage is
          available when it reconnects.
        </p>
      ) : problem !== null ? (
        <p role="alert" className="settings-empty">
          {problem.message} {nextStep(problem)}
        </p>
      ) : ready === null ? (
        <Skeleton />
      ) : (
        <>
          <SettingsSection title="Reported totals">
            <RowList label="Reported usage">
              {usageFacts(ready, locale).map((fact) => (
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
          </SettingsSection>
          <SettingsSection title="Report coverage">
            <RowList label="Accounting coverage">
              <Row
                title="Observed"
                control={<FactValue>{usageTime(ready.at, locale)}</FactValue>}
              />
              <Row
                title="Accounting began"
                detail={{ description: "Earlier history is not backfilled." }}
                control={
                  <FactValue>
                    {usageTime(ready.accounting_started, locale)}
                  </FactValue>
                }
              />
              <Row
                title="Retained since"
                detail={{
                  description: "Only retained reported samples are counted.",
                }}
                control={
                  <FactValue>
                    {usageTime(ready.retained_from, locale)}
                  </FactValue>
                }
              />
            </RowList>
          </SettingsSection>
        </>
      )}
    </>
  );
}
