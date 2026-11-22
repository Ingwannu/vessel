import { useState } from "react";

import type { NodeStatus } from "../../wire/bridge";
import type { ScheduleEdit } from "../../data/schedules/form";
import {
  chooseIsolation,
  type IsolationChoice,
  isProjectRoot,
  supportsScheduleIsolation,
} from "../../data/schedules/isolation";
import type { useScheduleTarget } from "./use-schedule-target";

export function useIsolationChoice(
  connection: NodeStatus | undefined,
  edit: ScheduleEdit | undefined,
  target: ReturnType<typeof useScheduleTarget>,
) {
  const [picked, setPicked] = useState<IsolationChoice | null>(null);
  const stored = edit?.definition.target.isolation ?? undefined;
  const supported = supportsScheduleIsolation(connection);
  const legacy = edit !== undefined && stored === undefined;
  const choice =
    picked ??
    (stored?.kind === "worktree" ? "worktree" : legacy ? "legacy" : "local");
  const root = isProjectRoot(target.spec);
  const project =
    target.spec?.place?.kind === "project" ? target.spec.place.project : null;
  const source = target.projectOptions.find((entry) => entry.value === project);
  const eligible = root && source !== undefined;
  const git = eligible && source.git;
  const valid = !supported
    ? stored === undefined
    : choice === "legacy" || (eligible && (choice !== "worktree" || git));

  return {
    visible: supported && (root || stored !== undefined),
    valid,
    policy: supported ? chooseIsolation(choice, stored) : undefined,
    reset: () => {
      setPicked(null);
    },
    field: { choice, change: setPicked, legacy, git, eligible },
  };
}
