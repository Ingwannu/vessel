// The tray's project and place (GOAT's workspace selector beside
// agent-orchestrator's project pill): the project, or a typed folder;
// then, in a git project, "Local" (its folder as it is), "New worktree"
// with the base it starts from, or "Existing branch" with the branch.
// Quiet pills, as everything in the tray is.

import { useEffect, useState } from "react";

import { useFleet } from "../../context";
import { readBranches } from "../../data/workspace/git";
import {
  NO_PLACE,
  type PlaceMode,
  projectPlace,
} from "../../data/workspace/place";
import { projectIn, useProjects } from "../../data/workspace/project-store";
import { projectStateWord } from "../../data/workspace/worktree-words";
import { type Choice, Select } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type { Branch, ProjectId } from "../../wire/proto";
import type { Starter } from "./use-starter";

/** The project pill's value for "no project". */
const NONE = "none";

const MODES: readonly Choice<PlaceMode>[] = [
  { value: "folder", label: "Local", icon: "folder" },
  { value: "worktree", label: "New worktree", icon: "git-branch" },
  { value: "branch", label: "Existing branch", icon: "git-fork" },
];

/** A project's branches, read when a worktree is being chosen. */
function useBranches(
  machine: NodeKey | null,
  project: ProjectId | null,
): readonly Branch[] {
  const fleet = useFleet();
  const [branches, setBranches] = useState<{
    project: ProjectId | null;
    list: readonly Branch[];
  }>({ project: null, list: [] });
  useEffect(() => {
    if (machine === null || project === null) {
      return undefined;
    }
    let current = true;
    void readBranches(fleet, machine, { kind: "project", project }).then(
      (outcome) => {
        if (current && outcome.ok) {
          setBranches({ project, list: outcome.value.branches });
        }
      },
    );
    return () => {
      current = false;
    };
  }, [fleet, machine, project]);
  return branches.project === project ? branches.list : [];
}

export function WherePlace({
  machine,
  starter,
}: {
  machine: NodeKey | null;
  starter: Starter;
}) {
  const fleet = useFleet();
  const projects = useProjects(fleet, machine);
  const place = starter.place;
  const chosen = place.kind === "project" ? place : null;
  const project = chosen === null ? null : projectIn(projects, chosen.project);
  const branches = useBranches(
    machine,
    chosen !== null && chosen.mode !== "folder" ? chosen.project : null,
  );
  if (projects.kind !== "ready" || projects.list.projects.length === 0) {
    return null;
  }
  const options: Choice<ProjectId | typeof NONE>[] = [
    { value: NONE, label: "No project", icon: "folder" },
    ...projects.list.projects.map((each) => ({
      value: each.id,
      label: each.name,
      // A project that cannot be used yet says why; a ready one says
      // nothing (the pill shows the detail beside the name).
      ...(projectStateWord(each.state) === null
        ? {}
        : { detail: projectStateWord(each.state) ?? "" }),
      icon: "folder" as const,
    })),
  ];
  const local = branches.filter(
    (branch) => branch.remote === null || branch.remote === undefined,
  );
  return (
    <>
      <span className="where-project">
        <Select
          aria-label="Project"
          look="ghost"
          size="sm"
          width="fit"
          value={chosen?.project ?? NONE}
          options={options}
          onChange={(value) => {
            starter.setPlace(value === NONE ? NO_PLACE : projectPlace(value));
          }}
          side="bottom"
        />
      </span>
      {chosen === null || project?.git !== true ? null : (
        <span className="where-mode">
          <Select
            aria-label="Where it runs"
            look="ghost"
            size="sm"
            width="fit"
            value={chosen.mode}
            options={MODES}
            onChange={(mode) => {
              starter.setPlace({ ...chosen, mode, branch: "", base: null });
            }}
            side="bottom"
          />
        </span>
      )}
      {chosen?.mode === "worktree" ? (
        <span className="where-branch">
          <Select
            aria-label="Base branch"
            look="ghost"
            size="sm"
            width="fit"
            value={chosen.base ?? ""}
            placeholder={`From ${project?.default_branch ?? "the default branch"}`}
            options={branches.map((branch) => ({
              value: branch.name,
              label: `From ${branch.name}`,
              icon: "git-branch",
              ...(branch.default ? { detail: "default" } : {}),
            }))}
            onChange={(base) => {
              starter.setPlace({ ...chosen, base });
            }}
            side="bottom"
          />
        </span>
      ) : chosen?.mode === "branch" ? (
        <span className="where-branch">
          <Select
            aria-label="Branch"
            look="ghost"
            size="sm"
            width="fit"
            value={chosen.branch === "" ? null : chosen.branch}
            placeholder="Choose a branch"
            options={local.map((branch) => ({
              value: branch.name,
              label: branch.name,
              icon: "git-branch",
              disabled: branch.current || (branch.worktree ?? null) !== null,
              ...(branch.current
                ? { detail: "checked out in the folder" }
                : (branch.worktree ?? null) !== null
                  ? { detail: "in a worktree" }
                  : {}),
            }))}
            onChange={(branch) => {
              starter.setPlace({ ...chosen, branch });
            }}
            side="bottom"
          />
        </span>
      ) : null}
    </>
  );
}
