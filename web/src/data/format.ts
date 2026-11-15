// How the UI writes protocol values: one vocabulary, the wire's
// (DESIGN.md §1.1). Statuses keep their protocol names.

import type {
  PermissionOptionKind,
  SessionInfo,
  SessionStatus,
  ToolCallStatus,
} from "../wire/proto";

/** The first four characters of an id: what the chrome shows. */
export function shortId(id: string): string {
  return id.slice(0, 4);
}

/** The word for a session's status. */
export function statusWord(status: SessionStatus): string {
  switch (status.state) {
    case "exited":
      return status.code === null ? "exited" : `exited ${status.code}`;
    case "starting":
    case "running":
    case "failed":
    case "interrupted":
      return status.state;
  }
}

/** The colour a state is drawn in (DESIGN.md §4.1): ok is the ready
 * lane's green, warn the review lane's amber, danger red, interrupt the
 * tertiary grey, neutral none. The kit's Badge takes the same union. */
export type StateTone = "neutral" | "ok" | "warn" | "danger" | "interrupt";

/** The tone of a session's status (DESIGN.md §6.4). */
export function statusTone(status: SessionStatus): StateTone {
  switch (status.state) {
    case "starting":
      return "warn";
    case "running":
      return "ok";
    case "failed":
      return "danger";
    case "interrupted":
      return "interrupt";
    case "exited":
      return "neutral";
  }
}

/** The tone of a tool call's status: working is warn, done is ok. */
export function toolTone(status: ToolCallStatus): StateTone {
  switch (status) {
    case "pending":
    case "in_progress":
      return "warn";
    case "completed":
      return "ok";
    case "failed":
      return "danger";
    case "canceled":
      return "neutral";
  }
}

/** How loud a permission answer's button is (DESIGN.md §6.7): the
 * harness's first option is the primary one, a lasting rejection is
 * danger, the rest are plain. */
export function answerEmphasis(
  kind: PermissionOptionKind,
  first: boolean,
): "primary" | "secondary" | "danger" {
  if (kind === "reject_always") {
    return "danger";
  }
  return first ? "primary" : "secondary";
}

/** `items` joined with commas, or `none` when there are none. */
export function listOr(items: readonly string[], none: string): string {
  return items.length === 0 ? none : items.join(", ");
}

/** Whether a session counts as live (the sidebar's count). */
export function isLive(session: SessionInfo): boolean {
  return (
    session.status.state === "starting" || session.status.state === "running"
  );
}

/** The last segment of a session's working directory, the project the
 * agent works in; `null` when it has none (the node user's home, `cwd`
 * unset, or the root). What an untitled session is called is
 * `titles.ts`'s `sessionName`. */
export function folderName(cwd: string | null | undefined): string | null {
  if (cwd === null || cwd === undefined) {
    return null;
  }
  // Both separators: the node may be a Windows machine.
  const segments = cwd.split(/[\\/]+/).filter((segment) => segment !== "");
  return segments.at(-1) ?? null;
}

/** "3m", "2h", "5d" since `iso`, at `now`. */
export function ago(iso: string, now: number = Date.now()): string {
  const seconds = Math.max(0, Math.round((now - Date.parse(iso)) / 1000));
  if (seconds < 60) {
    return `${seconds}s`;
  }
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) {
    return `${minutes}m`;
  }
  const hours = Math.round(minutes / 60);
  if (hours < 48) {
    return `${hours}h`;
  }
  return `${Math.round(hours / 24)}d`;
}

/** The formats `when` writes with, made once: `toLocaleTimeString` and
 * `toLocaleDateString` make a formatter on every call, and a session's
 * page asks for its start time on every batch of its log as it replays
 * (a CPU profile of opening 20,000 entries: about 40 ms in those calls
 * alone, under load, milestone 11's page join). */
const TIME = new Intl.DateTimeFormat("en-GB", {
  hour: "2-digit",
  minute: "2-digit",
});
const DATE = new Intl.DateTimeFormat("en-CA");

/** A local time of day, or a date when not today. */
export function when(iso: string): string {
  const date = new Date(iso);
  const today = new Date();
  const time = TIME.format(date);
  if (date.toDateString() === today.toDateString()) {
    return `today ${time}`;
  }
  return `${DATE.format(date)} ${time}`;
}
