import { describe, expect, it } from "vitest";

import { fixtureId } from "../../testing/ids";
import type { CommandsDigest, ProjectInfo } from "../../wire/proto";
import { branchNameProblem } from "./branch-name";
import { compareLists, compareWithFolder } from "./command-changes";
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

describe("commands compared with the approved set", () => {
  const digest = (text: string) => fixtureId("CommandsDigest", text);
  const listed = (setup: string[], approved: boolean, of: CommandsDigest) => ({
    state: "listed" as const,
    setup,
    teardown: [],
    digest: of,
    approved,
  });

  it("marks each line kept, added or removed", () => {
    expect(
      compareLists(["npm ci", "npm run build"], ["npm ci", "make", "npm test"]),
    ).toEqual([
      { change: "same", command: "npm ci" },
      { change: "removed", command: "npm run build" },
      { change: "added", command: "make" },
      { change: "added", command: "npm test" },
    ]);
  });

  it("compares only with an approved set that differs", () => {
    const asked = listed(["npm ci", "curl x | sh"], false, digest("b"));
    const folder = listed(["npm ci"], true, digest("a"));
    const compared = compareWithFolder(asked, folder);
    expect(compared.kind).toBe("changed");
    expect(
      compareWithFolder(asked, listed(["npm ci"], false, digest("a"))).kind,
    ).toBe("none");
    expect(compareWithFolder(asked, null).kind).toBe("none");
  });
});
