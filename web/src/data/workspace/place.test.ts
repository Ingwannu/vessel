import { describe, expect, it } from "vitest";

import { fixtureId } from "../../testing/ids";
import type { ProjectInfo } from "../../wire/proto";
import { branchNameProblem } from "./branch-name";
import {
  NO_PLACE,
  placeProblem,
  type ProjectPlace,
  placeSpec,
  projectPlace,
  worktreeBranch,
} from "./place";

const PROJECT: ProjectInfo = {
  id: fixtureId("ProjectId", "p1"),
  name: "vessel",
  root: "/work/vessel",
  git: true,
  default_branch: "origin/main",
  state: { state: "ready" },
  created_at: "2026-11-12T00:00:00Z",
  revision: 1,
};

function inWorktree(change: Partial<ProjectPlace>): ProjectPlace {
  return { ...projectPlace(PROJECT.id), mode: "worktree", ...change };
}

describe("where a session runs", () => {
  it("runs in a typed folder, or the home folder when none", () => {
    expect(placeSpec(NO_PLACE)).toEqual({});
    expect(placeSpec({ kind: "path", folder: " /srv/app " })).toEqual({
      cwd: "/srv/app",
    });
  });

  it("names a project's folder by its place, never a path", () => {
    expect(placeSpec(projectPlace(PROJECT.id))).toEqual({
      place: { kind: "project", project: PROJECT.id },
    });
    expect(worktreeBranch(projectPlace(PROJECT.id))).toBeNull();
  });

  it("needs a worktree first for a new or an existing branch", () => {
    expect(placeSpec(inWorktree({}))).toBeNull();
    expect(worktreeBranch(inWorktree({}))).toEqual({ kind: "new" });
    expect(
      worktreeBranch(inWorktree({ base: "origin/dev", branch: "fix/x" })),
    ).toEqual({
      kind: "new",
      name: "fix/x",
      base: { kind: "ref", name: "origin/dev" },
    });
    expect(
      worktreeBranch(inWorktree({ mode: "branch", branch: "feature" })),
    ).toEqual({ kind: "existing", name: "feature" });
  });

  it("says why it cannot start yet", () => {
    expect(placeProblem(projectPlace(PROJECT.id), null)).toContain("not on");
    expect(
      placeProblem(projectPlace(PROJECT.id), {
        ...PROJECT,
        state: { state: "cloning", stage: "receiving" },
      }),
    ).toContain("still being cloned");
    expect(placeProblem(inWorktree({}), { ...PROJECT, git: false })).toContain(
      "not a git repository",
    );
    expect(placeProblem(inWorktree({ mode: "branch" }), PROJECT)).toBe(
      "Choose the branch to run on.",
    );
    expect(placeProblem(inWorktree({ branch: "bad name" }), PROJECT)).toContain(
      "spaces",
    );
    expect(placeProblem(inWorktree({}), PROJECT)).toBeNull();
  });
});

describe("a branch name as git checks it", () => {
  it.each([
    ["feature/login", null],
    ["-x", "dash"],
    ["a..b", ".."],
    ["a/.hidden", "start with a dot"],
    ["topic.lock", ".lock"],
    ["end.", "end with a dot"],
    ["a//b", "two in a row"],
    ["what?", "“?”"],
    ["HEAD", "not a branch name"],
  ])("%s", (name, problem) => {
    const said = branchNameProblem(name);
    if (problem === null) {
      expect(said).toBeNull();
    } else {
      expect(said).toContain(problem);
    }
  });
});
