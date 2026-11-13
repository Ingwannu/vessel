// A question the agent asks, as the dock over the composer draws and
// answers it (milestone 11, Track A-page): its fields as steps, what each
// holds while the person answers, what the node would refuse, and the
// answer sent.
//
// [Decision Log]
// - Purpose: agent-orchestrator's question dock walks a form one step at
//   a time ("Question · 1 of 3"), each choice a row with its description,
//   free text in an "Other" row; GOAT and Paseo do the same with tabs.
// - Constraint: ACP's crate keeps a form's fields by key, so Claude's
//   `question_10` comes before `question_2`; Claude's `AskUserQuestion`
//   adds `question_N_custom` after `question_N`, which its adapter reads
//   as extra picks for a multi-select and as a note (or the answer when
//   nothing is picked) for a single choice. The node checks every answer
//   (`Question::check`) and refuses one that does not fit.
// - Choice: a step per field, Claude's pairs as one step with the custom
//   field as its Other row, `question_N` ordered by N; values kept as the
//   person typed them (a number as its text) and read into the node's
//   types only when sent; the node's checks run here first, with the
//   node's words, so a refusal is rare and a refusal that comes names its
//   field.

import type {
  AnswerValue,
  FieldName,
  Question,
  QuestionAnswer,
  QuestionField,
} from "../wire/proto";

/** One step of the dock: a field, and Claude's free-text companion. */
export interface Step {
  field: QuestionField;
  other: QuestionField | null;
}

/** What one field holds while the person answers. */
export type Draft =
  | { kind: "text"; text: string }
  | { kind: "picked"; values: readonly string[] }
  | { kind: "boolean"; on: boolean | null };

export type Drafts = Readonly<Partial<Record<FieldName, Draft>>>;

const CLAUDE = /^question_(\d+)$/;

/** The number of Claude's `question_N`, or `null`. */
function claudeNumber(name: string): number | null {
  const match = CLAUDE.exec(name);
  return match?.[1] === undefined ? null : Number(match[1]);
}

/** `question`'s steps, in the order the dock shows them. */
export function stepsOf(question: Question): Step[] {
  const companionOf = (field: QuestionField): QuestionField | null =>
    claudeNumber(field.name) === null
      ? null
      : (question.fields.find(
          (candidate) => candidate.name === `${field.name}_custom`,
        ) ?? null);
  const companions = new Set<string>();
  for (const field of question.fields) {
    const other = companionOf(field);
    if (other !== null) {
      companions.add(other.name);
    }
  }
  const steps = question.fields
    .filter((field) => !companions.has(field.name))
    .map((field): Step => ({ field, other: companionOf(field) }));
  // Claude's numbered questions by their number; anything else keeps the
  // node's place (a stable sort over numbered steps only).
  const numbered = steps
    .filter((step) => claudeNumber(step.field.name) !== null)
    .sort(
      (one, two) =>
        (claudeNumber(one.field.name) ?? 0) -
        (claudeNumber(two.field.name) ?? 0),
    );
  let next = 0;
  return steps.map((step) =>
    claudeNumber(step.field.name) === null ? step : (numbered[next++] ?? step),
  );
}

/** Each field's starting draft: its default where it has one. */
export function startingDrafts(question: Question): Drafts {
  const drafts: Partial<Record<FieldName, Draft>> = {};
  for (const field of question.fields) {
    drafts[field.name] = startingDraft(field);
  }
  return drafts;
}

function startingDraft(field: QuestionField): Draft {
  switch (field.kind) {
    case "text":
      return { kind: "text", text: field.default ?? "" };
    case "number":
    case "integer":
      return {
        kind: "text",
        text:
          field.default === undefined || field.default === null
            ? ""
            : String(field.default),
      };
    case "boolean":
      return { kind: "boolean", on: field.default ?? null };
    case "choice":
      return { kind: "picked", values: field.default ?? [] };
    case "unknown":
      return { kind: "text", text: "" };
  }
}

/** A field's value as the node takes it, `null` when it holds none, or
 * why it does not fit (the node's checks, in the page's words). */
export type Read =
  | { kind: "value"; value: AnswerValue }
  | { kind: "empty" }
  | { kind: "wrong"; why: string };

/** The longest text answer, in UTF-8 bytes (the node's). */
const MAX_TEXT = 16 * 1024;

/** What `draft` holds for `field`. */
export function readField(
  field: QuestionField,
  draft: Draft | undefined,
): Read {
  switch (field.kind) {
    case "unknown":
      return { kind: "wrong", why: "This can't be answered here" };
    case "text": {
      const text = draft?.kind === "text" ? draft.text : "";
      if (text === "") {
        return { kind: "empty" };
      }
      const length = Array.from(text).length;
      if (new TextEncoder().encode(text).length > MAX_TEXT) {
        return { kind: "wrong", why: "Too long" };
      }
      if (
        field.min_length !== undefined &&
        field.min_length !== null &&
        length < field.min_length
      ) {
        return {
          kind: "wrong",
          why: `At least ${String(field.min_length)} characters`,
        };
      }
      if (
        field.max_length !== undefined &&
        field.max_length !== null &&
        length > field.max_length
      ) {
        return {
          kind: "wrong",
          why: `At most ${String(field.max_length)} characters`,
        };
      }
      return { kind: "value", value: { type: "text", value: text } };
    }
    case "number":
    case "integer": {
      const text = draft?.kind === "text" ? draft.text.trim() : "";
      if (text === "") {
        return { kind: "empty" };
      }
      const number = Number(text);
      if (!Number.isFinite(number) || !/^-?\d*\.?\d+$/.test(text)) {
        return { kind: "wrong", why: "Not a number" };
      }
      if (field.kind === "integer" && !Number.isInteger(number)) {
        return { kind: "wrong", why: "A whole number" };
      }
      const minimum = field.minimum ?? null;
      const maximum = field.maximum ?? null;
      if (minimum !== null && number < minimum) {
        return { kind: "wrong", why: `At least ${String(minimum)}` };
      }
      if (maximum !== null && number > maximum) {
        return { kind: "wrong", why: `At most ${String(maximum)}` };
      }
      return {
        kind: "value",
        value:
          field.kind === "integer"
            ? { type: "integer", value: number }
            : { type: "number", value: number },
      };
    }
    case "boolean": {
      const on = draft?.kind === "boolean" ? draft.on : null;
      return on === null
        ? { kind: "empty" }
        : { kind: "value", value: { type: "boolean", value: on } };
    }
    case "choice": {
      const values = draft?.kind === "picked" ? draft.values : [];
      if (!field.multiple) {
        const first = values[0];
        return first === undefined
          ? { kind: "empty" }
          : { kind: "value", value: { type: "choice", value: first } };
      }
      if (values.length === 0) {
        return { kind: "empty" };
      }
      const min = field.min_items ?? null;
      const max = field.max_items ?? null;
      if (min !== null && values.length < min) {
        return { kind: "wrong", why: `Choose at least ${String(min)}` };
      }
      if (max !== null && values.length > max) {
        return { kind: "wrong", why: `Choose at most ${String(max)}` };
      }
      return { kind: "value", value: { type: "choices", value: [...values] } };
    }
  }
}

/** Why `step` cannot be left yet, or `null` when it can: a value that
 * does not fit, or a required field with none (Claude's pair counts as
 * answered when either half is). */
export function stepProblem(step: Step, drafts: Drafts): string | null {
  const read = readField(step.field, drafts[step.field.name]);
  const other =
    step.other === null ? null : readField(step.other, drafts[step.other.name]);
  if (read.kind === "wrong") {
    return read.why;
  }
  if (other?.kind === "wrong") {
    return other.why;
  }
  if (read.kind === "empty" && step.field.required && other?.kind !== "value") {
    return step.field.kind === "choice"
      ? "Choose an answer."
      : "This field is required.";
  }
  return null;
}

/** The answer `drafts` make: every field that holds a value. */
export function answerOf(question: Question, drafts: Drafts): QuestionAnswer {
  const values: Partial<Record<FieldName, AnswerValue>> = {};
  for (const field of question.fields) {
    const read = readField(field, drafts[field.name]);
    if (read.kind === "value") {
      values[field.name] = read.value;
    }
  }
  return { answer: "accept", values: definedValues(values) };
}

/** `values` without its absent keys, as the wire's map type wants. */
function definedValues(
  values: Partial<Record<FieldName, AnswerValue>>,
): Record<FieldName, AnswerValue> {
  const out: Record<string, AnswerValue> = {};
  for (const [name, value] of Object.entries(values)) {
    if (value !== undefined) {
      out[name] = value;
    }
  }
  return out;
}

/** The field the node's refusal names (its message reads "field `x`
 * …"), so the dock shows the refusal there; `null` for the form. */
export function refusedField(
  question: Question,
  message: string,
): FieldName | null {
  const named = /field `([^`]+)`/.exec(message)?.[1];
  return question.fields.find((field) => field.name === named)?.name ?? null;
}

/** Whether a question can be answered here at all. */
export function answerable(question: Question): boolean {
  return question.fields.every((field) => field.kind !== "unknown");
}
