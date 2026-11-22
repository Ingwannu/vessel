// Form values map to either create or a revisioned definition update.
// Carries from CLI-created schedules (including elapsed cadence, expiry,
// catch-up and extra session options) must survive an unrelated edit.
import type {
  Cadence,
  CreateSchedule,
  OptionChoice,
  ScheduleDefinition,
  ScheduleInfo,
  SessionSpec,
  UpdateSchedule,
} from "../../wire/proto";
import { promptTextError } from "../prompt-text";

export type NewSessionSpec = Extract<SessionSpec, { mode: "structured" }>;

export type ScheduleEdit = ScheduleInfo & {
  definition: ScheduleDefinition & {
    target: {
      kind: "new_session";
      spec: NewSessionSpec;
      after: "archive" | "keep";
    };
  };
};

export function editable(info: ScheduleInfo): info is ScheduleEdit {
  const { definition } = info;
  const { target } = definition;
  return (
    info.defined_by.kind !== "agent" &&
    info.state.state !== "unknown" &&
    definition.catch_up !== "unknown" &&
    definition.cadence.kind !== "unknown" &&
    target.kind === "new_session" &&
    target.after !== "unknown" &&
    target.spec.mode === "structured"
  );
}

const UTF8 = new TextEncoder();

export function nameError(name: string): string | null {
  if (name.trim() === "") return null;
  if (UTF8.encode(name.trim()).length > 80)
    return "Name must be 80 bytes or fewer.";
  return /\p{Cc}/u.test(name.trim()) ? "Name must be one line." : null;
}

export function promptError(prompt: string): string | null {
  if (prompt.trim() === "") return "Enter a prompt.";
  return promptTextError(prompt);
}

export function maxRuns(text: string): number | null | "invalid" {
  if (text.trim() === "") return null;
  if (!/^\d+$/u.test(text.trim())) return "invalid";
  const count = Number(text);
  return count >= 1 && count <= 1_000_000 ? count : "invalid";
}

interface FormValue {
  name: string;
  prompt: string;
  cadence: Cadence;
  spec: NewSessionSpec;
  archive: boolean;
  cap: number | null;
}

function definition(
  value: FormValue,
  original?: ScheduleDefinition,
): ScheduleDefinition {
  return {
    ...original,
    name: value.name.trim() === "" ? null : value.name.trim(),
    prompt: value.prompt,
    cadence: value.cadence,
    target: {
      kind: "new_session",
      spec: value.spec,
      after: value.archive ? "archive" : "keep",
    },
    catch_up: original?.catch_up ?? "skip",
    max_runs: value.cap,
  };
}

export function createRequest(value: FormValue): CreateSchedule {
  return { definition: definition(value), paused: false };
}

export function editRequest(
  row: ScheduleEdit,
  value: FormValue,
): UpdateSchedule {
  return {
    schedule: row.id,
    revision: row.revision,
    definition: definition(value, row.definition),
  };
}

export function sessionOptions(
  original: readonly OptionChoice[],
  changes: readonly OptionChoice[],
): OptionChoice[] {
  const changed = new Map(changes.map((choice) => [choice.id, choice]));
  const kept = original.map((choice) => changed.get(choice.id) ?? choice);
  const existing = new Set(original.map((choice) => choice.id));
  return [...kept, ...changes.filter((choice) => !existing.has(choice.id))];
}
