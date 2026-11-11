// Field (DESIGN.md §6.2): a label over any control, a hint under it, and
// the error in place of the hint when there is one. The wiring is Base
// UI's Field: the label names the control (a `<label for>` on a text
// field, a select's trigger, a combobox's input), the hint and the error
// describe it (`aria-describedby`), and an error marks it invalid
// (`aria-invalid`, `data-invalid` on the shell for its red edge).

import "./field.css";

import { Field as Base } from "@base-ui/react/field";
import { type ReactNode, useId } from "react";

export function Field({
  label,
  hint,
  error,
  disabled = false,
  children,
}: {
  /** Sentence case, never inside the control (DESIGN.md §6.2). */
  label: string;
  /** One line that says what the control cannot. Not a restatement of the
   * label (DESIGN.md §1, principle 9). */
  hint?: ReactNode;
  /** Shown in place of the hint, in the danger colour. */
  error?: string | null;
  disabled?: boolean;
  /** One kit control. */
  children: ReactNode;
}) {
  const invalid = error !== undefined && error !== null && error !== "";
  return (
    <Base.Root className="field" invalid={invalid} disabled={disabled}>
      <Base.Label className="field-label">{label}</Base.Label>
      {children}
      {invalid ? (
        <Base.Error className="field-error" match={true}>
          {error}
        </Base.Error>
      ) : hint === undefined ? null : (
        <Base.Description className="field-hint">{hint}</Base.Description>
      )}
    </Base.Root>
  );
}

/** A label over a group of controls that name themselves (a Segmented's
 * radios, a row of checkboxes): Base UI's Field would give its one label
 * to every control inside, so a group is labelled as a group instead. */
export function FieldGroup({
  label,
  hint,
  error,
  children,
}: {
  label: string;
  hint?: ReactNode;
  error?: string | null;
  children: ReactNode;
}) {
  const id = useId();
  const invalid = error !== undefined && error !== null && error !== "";
  return (
    <div
      className="field"
      role="group"
      aria-labelledby={`${id}-label`}
      data-invalid={invalid ? "" : undefined}
    >
      <span className="field-label" id={`${id}-label`}>
        {label}
      </span>
      {children}
      {invalid ? (
        <p className="field-error" role="alert">
          {error}
        </p>
      ) : hint === undefined ? null : (
        <p className="field-hint">{hint}</p>
      )}
    </div>
  );
}
