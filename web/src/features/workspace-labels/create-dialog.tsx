// "Create label…" from a workspace row's Labels menu: a name and a
// colour, then Create, which makes the label on the workspace's machine
// and puts it on that workspace (Paseo's create page).
//
// [Decision Log]
// - Reference: Paseo's create page creates only from the one row that
//   says so, only once the name is not empty after normalising, the first
//   colour chosen to start with; a failure keeps the name and colour as
//   typed and says what happened.
// - Choice: the same, in a dialog (picker.tsx says why). Assignment
//   creates the label (`workspace.label.assign` with its colour); an
//   existing name is simply assigned in the colour the machine keeps.

import { useState } from "react";

import type { WorkspaceLabels } from "../../data/workspace-labels/hub";
import {
  LABEL_COLORS,
  normalizeLabelName,
} from "../../data/workspace-labels/names";
import { failureText } from "../../data/workspace-labels/picker";
import { Button, Dialog, Field, FieldGroup, TextField } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type { LabelColor, WorkspaceId } from "../../wire/proto";
import { LabelSwatches } from "./chip";

export function CreateLabelDialog({
  hub,
  node,
  workspace,
  onClose,
}: {
  hub: WorkspaceLabels;
  node: NodeKey;
  workspace: WorkspaceId;
  onClose: () => void;
}) {
  const [name, setName] = useState("");
  const [color, setColor] = useState<LabelColor>(LABEL_COLORS[0]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const typed = normalizeLabelName(name);
  const create = () => {
    if (typed === "" || busy) {
      return;
    }
    setBusy(true);
    setError(null);
    void hub
      .assign(node, workspace, { name: typed, color }, true)
      .then((outcome) => {
        setBusy(false);
        if (outcome.ok) {
          onClose();
        } else {
          setError(failureText(outcome));
        }
      });
  };
  return (
    <Dialog
      title="Create label"
      onClose={onClose}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button
            variant="primary"
            disabled={typed === ""}
            busy={busy}
            onClick={create}
          >
            Create
          </Button>
        </>
      }
    >
      <Field label="Name" error={error}>
        <TextField
          value={name}
          onChange={setName}
          placeholder="Label name"
          disabled={busy}
          autoFocus
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              create();
            }
          }}
        />
      </Field>
      <FieldGroup label="Color">
        <LabelSwatches value={color} onChange={setColor} disabled={busy} />
      </FieldGroup>
    </Dialog>
  );
}
