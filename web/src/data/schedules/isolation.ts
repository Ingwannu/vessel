// Keep a legacy definition until its author chooses an explicit policy.
import type { NodeStatus } from "../../wire/bridge";
import {
  SCHEDULE_ISOLATION_PROTOCOL,
  type WorkspaceIsolation,
} from "../../wire/proto";
import type { NewSessionSpec } from "./form";

export function supportsScheduleIsolation(connection: NodeStatus | undefined) {
  const required: number | null = SCHEDULE_ISOLATION_PROTOCOL;
  return (
    required !== null &&
    connection?.state === "connected" &&
    connection.protocol >= required
  );
}

export type IsolationChoice = "legacy" | "local" | "worktree";

export function chooseIsolation(
  value: IsolationChoice,
  stored: WorkspaceIsolation | undefined,
): WorkspaceIsolation | undefined {
  switch (value) {
    case "legacy":
      return undefined;
    case "local":
      return { kind: "local" };
    case "worktree":
      return stored?.kind === "worktree"
        ? stored
        : { kind: "worktree", branch: { kind: "new" }, setup: "skip" };
  }
}

export function isProjectRoot(spec: NewSessionSpec | null): boolean {
  return (
    spec?.place?.kind === "project" &&
    spec.place.dir == null &&
    spec.cwd == null
  );
}
