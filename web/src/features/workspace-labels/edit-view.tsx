// The label manager's edit view (Paseo's WorkspaceLabelEditView): the
// name and the colour, changed together and sent once by the footer's
// Save; Enter in the name saves too. A failure shows under them, and the
// draft stays as typed.

import type { Draft } from "../../data/workspace-labels/manager";
import type { LabelColor } from "../../wire/proto";
import { Field, FieldGroup, TextField } from "../../ui/kit";
import { LabelSwatches } from "./chip";

export function LabelEditView({
  draft,
  disabled,
  error,
  onName,
  onColor,
  onSubmit,
}: {
  draft: Draft;
  disabled: boolean;
  error: string | null;
  onName: (typed: string) => void;
  onColor: (color: LabelColor) => void;
  onSubmit: () => void;
}) {
  return (
    <>
      <Field label="Name">
        <TextField
          value={draft.typed}
          onChange={onName}
          onKeyDown={(event) => {
            if (event.key === "Enter" && !event.nativeEvent.isComposing) {
              event.preventDefault();
              onSubmit();
            }
          }}
          disabled={disabled}
          spellCheck={false}
          autoFocus
        />
      </Field>
      <FieldGroup label="Color">
        <LabelSwatches
          value={draft.color}
          onChange={onColor}
          disabled={disabled}
        />
      </FieldGroup>
      {error === null ? null : <p className="label-note error">{error}</p>}
    </>
  );
}
