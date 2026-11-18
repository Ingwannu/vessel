// An agent's "Diagnostic" (ADR 0111, TRACKER 568 and 67), as Paseo's
// provider sheet's footer button and its sheet (`provider-diagnostic-
// sheet.tsx`): asked when the dialog opens and again on Refresh; each ask
// probes the agent anew, and `harness.options` then reads that probe.

import { useCallback, useEffect, useRef, useState } from "react";

import { useFleet, useFleetState } from "../../context";
import { agentDiagnosticText } from "../../data/agent-diagnostic";
import { agentDiagnostic } from "../../data/calls";
import type { NodeKey } from "../../wire/bridge";
import { DIAGNOSTICS_WIRE_PROTOCOL, type HarnessRef } from "../../wire/proto";
import { Button } from "../../ui/kit";
import { DIAGNOSTIC_WORDS, DiagnosticTextDialog } from "./text-dialog";

/** "Diagnostic", on a machine speaking the protocol; nothing otherwise. */
export function AgentDiagnosticButton({
  machine,
  agent,
}: {
  machine: NodeKey;
  agent: HarnessRef;
}) {
  const status = useFleetState((state) => state.statuses[machine]);
  const [open, setOpen] = useState(false);
  if (
    status?.state !== "connected" ||
    status.protocol < DIAGNOSTICS_WIRE_PROTOCOL
  ) {
    return null;
  }
  return (
    <>
      <Button
        size="sm"
        icon="file"
        onClick={() => {
          setOpen(true);
        }}
      >
        Diagnostic
      </Button>
      {open ? (
        <AgentDiagnosticDialog
          machine={machine}
          agent={agent}
          onClose={() => {
            setOpen(false);
          }}
        />
      ) : null}
    </>
  );
}

function AgentDiagnosticDialog({
  machine,
  agent,
  onClose,
}: {
  machine: NodeKey;
  agent: HarnessRef;
  onClose: () => void;
}) {
  const fleet = useFleet();
  const [text, setText] = useState<string | null>(null);
  const [running, setRunning] = useState(true);
  // The agent this dialog opened for; a newer ask supersedes an older one.
  const asked = useRef(agent);
  const run = useRef(0);
  const fetchText = useCallback(() => {
    run.current += 1;
    const id = run.current;
    void agentDiagnostic(fleet, machine, asked.current, null).then(
      (outcome) => {
        if (run.current !== id) return;
        setText(
          outcome.ok
            ? agentDiagnosticText(outcome.value)
            : outcome.problem.message === ""
              ? DIAGNOSTIC_WORDS.failed
              : outcome.problem.message,
        );
        setRunning(false);
      },
    );
  }, [fleet, machine]);
  useEffect(() => {
    fetchText();
    return () => {
      run.current += 1;
    };
  }, [fetchText]);
  return (
    <DiagnosticTextDialog
      title="Diagnostic"
      text={text}
      running={running}
      onRefresh={() => {
        setRunning(true);
        fetchText();
      }}
      onClose={onClose}
    />
  );
}
