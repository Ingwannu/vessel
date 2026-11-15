// One step of a question (question-dock.tsx): its field drawn by its
// kind with a kit control, and Claude's companion text as the step's
// "Other" row, in the agent's own words.

import type { Draft, Drafts, Step } from "../../data/question-form";
import { Checkbox, OptionList, TextField } from "../../ui/kit";
import type { FieldName, QuestionField as Field } from "../../wire/proto";

function textOf(draft: Draft | undefined): string {
  return draft?.kind === "text" ? draft.text : "";
}

function Control({
  field,
  draft,
  set,
  label,
}: {
  field: Field;
  draft: Draft | undefined;
  set: (draft: Draft) => void;
  label: string;
}) {
  switch (field.kind) {
    case "choice":
      return (
        <OptionList
          label={label}
          options={field.options.map((option) => ({
            value: option.value,
            label: option.title,
            description:
              [option.description, option.preview]
                .filter(
                  (value) =>
                    value !== undefined && value !== null && value !== "",
                )
                .join("\n") || null,
          }))}
          picked={draft?.kind === "picked" ? draft.values : []}
          multiple={field.multiple}
          onChange={(values) => {
            set({ kind: "picked", values });
          }}
        />
      );
    case "boolean":
      return (
        <Checkbox
          label={field.title ?? label}
          checked={draft?.kind === "boolean" && draft.on === true}
          onChange={(on) => {
            set({ kind: "boolean", on });
          }}
        />
      );
    case "text":
    case "number":
    case "integer":
      return (
        <TextField
          size="sm"
          aria-label={field.title ?? label}
          placeholder={
            field.kind === "text"
              ? (field.presentation?.placeholder ??
                field.description ??
                "Your answer")
              : field.kind === "integer"
                ? "A whole number"
                : "A number"
          }
          value={textOf(draft)}
          onChange={(text) => {
            set({ kind: "text", text });
          }}
        />
      );
    case "unknown":
      return (
        <p className="question-note">
          This asks for something this page cannot fill in. Skip it, or answer
          in the agent&apos;s own window.
        </p>
      );
  }
}

export function QuestionStep({
  step,
  drafts,
  prompt,
  setDraft,
}: {
  step: Step;
  drafts: Drafts;
  /** What the step asks, for its controls' names. */
  prompt: string;
  setDraft: (name: FieldName, draft: Draft) => void;
}) {
  const other = step.other;
  return (
    <div className="question-step">
      <Control
        field={step.field}
        draft={drafts[step.field.name]}
        label={prompt}
        set={(draft) => {
          setDraft(step.field.name, draft);
        }}
      />
      {other === null ? null : (
        <div className="question-other">
          <span className="option-badge" aria-hidden="true">
            0
          </span>
          <TextField
            size="sm"
            aria-label={other.title ?? "Other"}
            placeholder={`${other.title ?? "Other"}…`}
            value={textOf(drafts[other.name])}
            onChange={(text) => {
              setDraft(other.name, { kind: "text", text });
            }}
          />
        </div>
      )}
      {other?.description === undefined || other.description === null ? null : (
        <p className="question-other-note">{other.description}</p>
      )}
    </div>
  );
}
