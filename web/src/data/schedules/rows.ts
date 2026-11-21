// Display and action eligibility for the node's schedule state. A session
// missing from this tab's directory can be loading or archived, so the
// directory supplies names and marks only. The node owns terminal states.
import { sinceLabel } from "../board";
import { folderName } from "../format";
import type { NodeKey } from "../../wire/bridge";
import type {
  Cadence,
  HarnessRef,
  ScheduleInfo,
  ScheduleState,
  SessionInfo,
} from "../../wire/proto";
import { sessionName } from "../titles";

export type ScheduleBucket = "active" | "ended";
export type ScheduleAction = "pause" | "resume" | "run" | "delete";

export interface ScheduleRow {
  node: NodeKey;
  info: ScheduleInfo;
  pending?: ScheduleAction;
}

export function rowKey(row: ScheduleRow): string {
  return `${row.node}:${row.info.id}`;
}

export function scheduleTitle(info: ScheduleInfo): string {
  const { definition } = info;
  const title =
    definition.target.kind === "new_session"
      ? definition.target.spec.title
      : null;
  const first = [definition.name, title, ...definition.prompt.split("\n")]
    .map((value) => value?.trim() ?? "")
    .find((value) => value !== "");
  return (
    first ?? (isHeartbeat(info) ? "Untitled heartbeat" : "Untitled schedule")
  );
}

export function isHeartbeat(info: ScheduleInfo): boolean {
  return (
    info.defined_by.kind === "agent" &&
    info.definition.target.kind === "session" &&
    info.defined_by.session === info.definition.target.session
  );
}

export function bucket(state: ScheduleState): ScheduleBucket {
  return state.state === "active" || state.state === "paused"
    ? "active"
    : "ended";
}

export function stateLabel(state: ScheduleState): string {
  switch (state.state) {
    case "active":
      return "Active";
    case "paused":
      return "Paused";
    case "unknown":
      return "Unknown";
    case "ended":
      switch (state.reason) {
        case "max_runs":
          return "Finished";
        case "expired":
          return "Expired";
        case "creator_gone":
          return "Creator gone";
        case "target_gone":
          return "Target gone";
        case "unknown":
          return "Ended";
      }
  }
}

export function rowActions(info: ScheduleInfo): readonly ScheduleAction[] {
  if (isHeartbeat(info) || bucket(info.state) === "ended") return ["delete"];
  const actions: ScheduleAction[] = [
    info.state.state === "active" ? "pause" : "resume",
  ];
  if (
    info.definition.target.kind !== "unknown" &&
    info.last_run?.outcome.outcome !== "making"
  ) {
    actions.push("run");
  }
  return [...actions, "delete"];
}

export function scheduleTarget(
  info: ScheduleInfo,
  sessions: readonly SessionInfo[],
  projectName: string | null,
): { label: string; harness?: HarnessRef } {
  const { target } = info.definition;
  switch (target.kind) {
    case "unknown":
      return { label: "Unknown target" };
    case "new_session":
      return {
        label: projectName ?? folderName(target.spec.cwd) ?? "New agent",
        harness: target.spec.harness,
      };
    case "session": {
      const session = sessions.find((entry) => entry.id === target.session);
      return session === undefined
        ? { label: "Agent unavailable" }
        : { label: sessionName(session), harness: session.spec.harness };
    }
  }
}

export const CRON_PRESETS = [
  { value: "* * * * *", label: "Every minute" },
  { value: "0 * * * *", label: "Every hour" },
  { value: "0 9 * * *", label: "Daily 9:00" },
  { value: "0 9 * * 1-5", label: "Weekdays 9:00" },
  { value: "0 9 * * 1", label: "Mondays 9:00" },
] as const;

export function cadenceLabel(cadence: Cadence): string {
  switch (cadence.kind) {
    case "unknown":
      return "Unknown cadence";
    case "cron":
      return cronLabel(cadence.expr, cadence.zone);
    case "every": {
      const units = [
        [86_400, "day"],
        [3_600, "hour"],
        [60, "minute"],
      ] as const;
      const [size, unit] = units.find(
        ([size]) => cadence.seconds % size === 0,
      ) ?? [1, "second"];
      const count = cadence.seconds / size;
      return `Every ${count} ${unit}${count === 1 ? "" : "s"}`;
    }
  }
}

const DAYS = [
  "Sundays",
  "Mondays",
  "Tuesdays",
  "Wednesdays",
  "Thursdays",
  "Fridays",
  "Saturdays",
];

function cronDay(day: string): string | null {
  switch (day) {
    case "*":
      return "Daily";
    case "1-5":
      return "Weekdays";
    case "0,6":
    case "6,0":
      return "Weekends";
    default:
      return /^[0-6]$/u.test(day) ? (DAYS[Number(day)] ?? null) : null;
  }
}

function cronLabel(expression: string, zone: string): string {
  const [minute, hour, date, month, day] = expression.trim().split(/\s+/u);
  if (
    minute === undefined ||
    hour === undefined ||
    day === undefined ||
    date !== "*" ||
    month !== "*"
  )
    return expression;
  if (minute === "*" && hour === "*" && day === "*") return "Every minute";
  if (!/^\d+$/u.test(minute)) return expression;
  const mm = minute.padStart(2, "0");
  if (hour === "*") {
    if (day !== "*") return expression;
    return Number(minute) === 0 ? "Every hour" : `Every hour at :${mm}`;
  }
  const days = cronDay(day);
  return days !== null && /^\d+$/u.test(hour)
    ? `${days} at ${hour.padStart(2, "0")}:${mm} ${zone}`
    : expression;
}

function nextRun(instant: string, now: number): string | null {
  const minutes = (Date.parse(instant) - now) / 60_000;
  if (!Number.isFinite(minutes)) return null;
  if (minutes < 1) return "Next run soon";
  if (minutes < 60) return `Next run in ${Math.round(minutes)}m`;
  const hours = minutes / 60;
  if (hours < 24) return `Next run in ${Math.round(hours)}h`;
  return `Next run in ${Math.round(hours / 24)}d`;
}

export function scheduleMeta(
  info: ScheduleInfo,
  machine: string | null,
  now: number,
  pending: ScheduleAction | undefined,
): string[] {
  const meta = [
    cadenceLabel(info.definition.cadence),
    `Created ${sinceLabel(info.created_at, now)}`,
    info.last_run == null
      ? "Never run"
      : `Last run ${sinceLabel(info.last_run.at, now)}`,
  ];
  if (info.state.state === "active" && pending !== "pause") {
    const next = nextRun(info.state.next_due, now);
    if (next !== null) meta.push(next);
  }
  if (machine !== null && machine !== "") meta.unshift(machine);
  return meta;
}
