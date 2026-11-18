// Settings › Diagnostics (ADR 0111, TRACKER 567), as Paseo's
// (`settings-screen.tsx` `DiagnosticsSection`, `app-diagnostic-sheet.tsx`):
// one row, "App diagnostic", whose Run opens a dialog that asks each
// connected machine for its report, shows a row per step while it runs,
// then the whole text with Copy and Refresh.

import { useEffect, useRef, useState } from "react";

import { useFleet, useFleetState } from "../../context";
import {
  appDiagnosticText,
  diagnosticSteps,
  type MachineDiagnostic,
  NODE_DIAGNOSTICS_PROTOCOL,
} from "../../data/app-diagnostic";
import { nodeDiagnostics } from "../../data/calls";
import type { FleetState } from "../../data/fleet";
import type { NodeKey } from "../../wire/bridge";
import { nodeLabel } from "../../data/nodes";
import { Button, Row, RowList, Spinner, StatusDot } from "../../ui/kit";
import { SettingsPage, SettingsSection } from "../settings/settings-page";
import { DiagnosticTextDialog } from "./text-dialog";

/** Paseo's words, with Vessel's nouns. */
export const APP_DIAGNOSTIC_WORDS = {
  title: "App diagnostic",
  hint: "Collect connection, machine, node and agent details",
  run: "Run",
} as const;

export function DiagnosticsSection() {
  const [open, setOpen] = useState(false);
  return (
    <SettingsPage lead="What this page and each machine say of themselves, to copy into a report.">
      <SettingsSection title="Diagnostics">
        <RowList label="Diagnostics">
          <Row
            title={APP_DIAGNOSTIC_WORDS.title}
            detail={{ description: APP_DIAGNOSTIC_WORDS.hint }}
            control={
              <Button
                size="sm"
                data-testid="app-diagnostic-run"
                onClick={() => {
                  setOpen(true);
                }}
              >
                {APP_DIAGNOSTIC_WORDS.run}
              </Button>
            }
          />
        </RowList>
      </SettingsSection>
      {open ? (
        <AppDiagnosticDialog
          onClose={() => {
            setOpen(false);
          }}
        />
      ) : null}
    </SettingsPage>
  );
}

/** Each machine of the book as the run starts: running where it can be
 * asked, settled where it cannot. */
function starting(state: FleetState): MachineDiagnostic[] {
  return (state.book ?? []).map((node) => {
    const status = state.statuses[node.key];
    const label = nodeLabel(node, status);
    if (status?.state !== "connected") {
      return { label, state: "not-connected" };
    }
    const facts = {
      label,
      protocol: status.protocol,
      version: status.node.version,
    };
    return status.protocol < NODE_DIAGNOSTICS_PROTOCOL
      ? { ...facts, state: "unsupported" }
      : { ...facts, state: "running" };
  });
}

/** One run: when it started, the book's keys in order, and each
 * machine's part as it starts. */
interface Run {
  id: number;
  collectedAt: string;
  keys: readonly NodeKey[];
  first: MachineDiagnostic[];
}

function begin(state: FleetState, id: number): Run {
  return {
    id,
    collectedAt: new Date().toISOString(),
    keys: (state.book ?? []).map((node) => node.key),
    first: starting(state),
  };
}

function AppDiagnosticDialog({ onClose }: { onClose: () => void }) {
  const fleet = useFleet();
  const link = useFleetState((state) => state.link);
  // A run starts when the dialog opens and on Refresh (an event, so no
  // state is set from the effect); the effect asks its machines.
  const [run, setRun] = useState<Run>(() => begin(fleet.store.get(), 0));
  const [answers, setAnswers] = useState<
    ReadonlyMap<number, MachineDiagnostic>
  >(() => new Map());
  const current = useRef(run.id);
  useEffect(() => {
    current.current = run.id;
    run.first.forEach((machine, index) => {
      const key = run.keys[index];
      if (machine.state !== "running" || key === undefined) return;
      void nodeDiagnostics(fleet, key).then((outcome) => {
        if (current.current !== run.id) return;
        const answer: MachineDiagnostic = outcome.ok
          ? { ...machine, state: "done", report: outcome.value }
          : { ...machine, state: "failed", message: outcome.problem.message };
        setAnswers((now) => new Map(now).set(index, answer));
      });
    });
    return () => {
      current.current = -1;
    };
  }, [fleet, run]);
  const machines = run.first.map(
    (machine, index) => answers.get(index) ?? machine,
  );
  const collectedAt = run.collectedAt;
  const steps = diagnosticSteps(machines);
  const running = steps.some((step) => step.state === "running");
  const text = running
    ? null
    : appDiagnosticText(
        {
          collectedAt,
          version: link.kind === "open" ? link.version : "not connected",
          platform: navigator.userAgent,
          machines: machines.length,
        },
        machines,
      );
  return (
    <DiagnosticTextDialog
      title={APP_DIAGNOSTIC_WORDS.title}
      text={text}
      running={running}
      onRefresh={() => {
        setAnswers(new Map());
        setRun(begin(fleet.store.get(), run.id + 1));
      }}
      onClose={onClose}
      progress={
        <ul className="diagnostic-steps" data-testid="app-diagnostic-steps">
          {steps.map((step, index) => (
            <li
              key={`${String(index)}-${step.label}`}
              className="diagnostic-step"
            >
              {step.state === "running" ? (
                <Spinner />
              ) : (
                <StatusDot
                  tone={step.state === "done" ? "ok" : "danger"}
                  label={step.state}
                />
              )}
              {step.label}: {step.state}
            </li>
          ))}
        </ul>
      }
    />
  );
}
