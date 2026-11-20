// Rename a machine: the book's name rules checked as typed, the reason
// under the field; the bridge's own refusal (a name taken meanwhile by
// the CLI, say) shown in the dialog, not lost in a toast.

import { useState } from "react";

import { useFleet, useFleetState } from "../../context";
import { renameMachine } from "../../data/book";
import { nameProblem } from "../../data/machines";
import { withNextStep, type Problem } from "../../data/outcome";
import type { BookNode } from "../../wire/bridge";
import { Banner, Button, Dialog, Field, TextField } from "../../ui/kit";
import { toast } from "../../ui/toasts";

export function RenameDialog({
  node,
  onClose,
  onRenamed,
}: {
  node: BookNode;
  onClose: () => void;
  /** After the book took the new name. */
  onRenamed: (name: string) => void;
}) {
  const fleet = useFleet();
  const book = useFleetState((state) => state.book);
  const [name, setName] = useState(node.label);
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<Problem | null>(null);
  const invalid = nameProblem(name, book ?? [], node.key);
  const unchanged = name === node.label;
  const submit = () => {
    if (invalid !== null || unchanged || busy) {
      return;
    }
    setBusy(true);
    void renameMachine(fleet, node.key, name).then((outcome) => {
      setBusy(false);
      if (outcome.ok) {
        toast(`Renamed ${node.label} to ${name}`);
        onRenamed(name);
      } else {
        setProblem(outcome.problem);
      }
    });
  };
  return (
    <Dialog
      title={`Rename ${node.label}`}
      onClose={onClose}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button
            variant="primary"
            busy={busy}
            disabled={invalid !== null || unchanged}
            onClick={submit}
          >
            Rename
          </Button>
        </>
      }
    >
      <Field
        label="Name"
        hint="How this page and the CLI's vessel nodes name it."
        error={unchanged ? null : invalid}
      >
        <TextField
          value={name}
          onChange={(next) => {
            setName(next);
            setProblem(null);
          }}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              submit();
            }
          }}
          spellCheck={false}
        />
      </Field>
      {problem === null ? null : (
        <Banner tone="error">{withNextStep(problem)}</Banner>
      )}
    </Dialog>
  );
}
