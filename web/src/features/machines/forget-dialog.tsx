// Forget a machine, after saying what that does and what it does not
// (GOAT's "Forget host…", Paseo's "Remove host"): the machine leaves this
// page and the CLI's book and its connection closes; nothing on it
// changes.

import "./forget-dialog.css";

import { useState } from "react";

import { useFleet } from "../../context";
import { forgetMachine } from "../../data/book";
import { nextStep, type Problem } from "../../data/outcome";
import type { BookNode } from "../../wire/bridge";
import { Banner, Button, Dialog } from "../../ui/kit";
import { toast } from "../../ui/toasts";

export function ForgetDialog({
  node,
  onClose,
}: {
  node: BookNode;
  onClose: () => void;
}) {
  const fleet = useFleet();
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<Problem | null>(null);
  return (
    <Dialog
      title={`Forget ${node.label}?`}
      onClose={onClose}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button
            variant="danger-solid"
            busy={busy}
            onClick={() => {
              setBusy(true);
              void forgetMachine(fleet, node.key).then((outcome) => {
                setBusy(false);
                if (outcome.ok) {
                  toast(`Forgot ${node.label}`);
                  onClose();
                } else {
                  setProblem(outcome.problem);
                }
              });
            }}
          >
            Forget
          </Button>
        </>
      }
    >
      <div className="dialog-prose">
        <p>
          {node.label} leaves this page and the list the CLI uses (
          <code>vessel nodes</code>), and its connection closes.
        </p>
        <p>
          Nothing on {node.label} changes: its sessions keep running and Vessel
          stays installed there. To use it from here again, add it again with a
          new link.
        </p>
      </div>
      {problem === null ? null : (
        <Banner tone="error">
          {problem.message} {nextStep(problem)}
        </Banner>
      )}
    </Dialog>
  );
}
