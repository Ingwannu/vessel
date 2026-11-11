// What a machine shares with the others (Settings › Sync, ADR 0012): the
// calls, and what each row says. Moved out of the old Sync page, which
// made these calls and decisions inside its component.

import type { NodeKey } from "../wire/bridge";
import type { FleetMember, Manifest, SecretName } from "../wire/proto";
import type { StateTone } from "./format";
import type { Fleet } from "./fleet";
import { attempt, type Outcome } from "./outcome";

/** What a home publishes, and who has joined it. */
export interface Published {
  manifest: Manifest;
  members: readonly FleetMember[];
  secrets: readonly SecretName[];
}

/** What `node` publishes, if it is a home. */
export function loadPublished(
  fleet: Fleet,
  node: NodeKey,
): Promise<Outcome<Published>> {
  return attempt(async () => {
    const [manifest, members, secrets] = await Promise.all([
      fleet.bridge.call(node, "fleet.manifest", null),
      fleet.bridge.call(node, "fleet.members", null),
      fleet.bridge.call(node, "secret.list", null),
    ]);
    return { manifest, members, secrets };
  });
}

/** The secrets the published tool servers name (names only; the values
 * are resolved on each machine and never leave it). */
export function secretNames(manifest: Manifest): string[] {
  const names = new Set<string>();
  for (const transport of Object.values(manifest.mcp)) {
    const values =
      transport.type === "stdio"
        ? Object.values(transport.env ?? {})
        : Object.values(transport.headers ?? {});
    for (const value of values) {
      if (typeof value !== "string") {
        names.add(value.secret);
      }
    }
  }
  return [...names].sort();
}

/** Whether a member has what the home publishes: a word and a tone. */
export function memberState(
  member: FleetMember,
  generation: number,
): { word: string; tone: StateTone } {
  const applied = member.applied ?? null;
  if (applied === null) {
    return { word: "Not yet", tone: "neutral" };
  }
  const behind = generation - applied;
  return behind <= 0
    ? { word: "Up to date", tone: "ok" }
    : { word: `${String(behind)} behind`, tone: "warn" };
}

/** A member's row's line: what it applied, its kept edits, its last
 * report. */
export function memberLine(
  member: FleetMember,
  ago: (iso: string) => string,
): string[] {
  const applied = member.applied ?? null;
  const conflicts = (member.conflicts ?? []).length;
  const reported = member.reported_at ?? null;
  return [
    applied === null
      ? "Has not reported"
      : `Applied version ${String(applied)}`,
    conflicts === 0
      ? "no kept edits"
      : `${String(conflicts)} kept ${conflicts === 1 ? "edit" : "edits"}`,
    ...(reported === null ? [] : [`reported ${ago(reported)} ago`]),
  ];
}

/** The members that kept a local edit over what the home published. */
export function withConflicts(
  members: readonly FleetMember[],
): readonly FleetMember[] {
  return members.filter((member) => (member.conflicts ?? []).length > 0);
}
