import { describe, expect, it } from "vitest";

import { refusalWords } from "./git-words";
import {
  cloneFailureWords,
  counted,
  gitSupportWords,
  keptReasonWords,
  projectStateWord,
  setupFailureWords,
  snapshotCovers,
  worktreeRefusalWords,
} from "./worktree-words";

describe("git's refusals in plain words", () => {
  it("names what happened and what to do, never the reason's name", () => {
    const push = refusalWords({ kind: "non_fast_forward" });
    expect(push.title).toBe("The remote has commits this branch does not.");
    expect(push.next).toContain("Pull first");
    expect(push.title).not.toContain("non_fast_forward");
  });

  it("carries the paths and the output a refusal names", () => {
    const overwrite = refusalWords({
      kind: "would_overwrite",
      paths: ["src/a.ts", "README.md"],
    });
    expect(overwrite.paths).toEqual(["src/a.ts", "README.md"]);
    const hook = refusalWords({ kind: "hook_failed", output: "lint failed" });
    expect(hook.output).toBe("lint failed");
    expect(refusalWords({ kind: "nothing_to_commit" }).output).toBeNull();
  });

  it("says which operation holds the repository", () => {
    expect(
      refusalWords({ kind: "operation", operation: "cherry_pick" }).title,
    ).toBe("The repository is in the middle of a cherry-pick.");
  });

  it("reads a reason from a newer machine as unknown, not as a failure", () => {
    expect(refusalWords({ kind: "unknown" }).title).toContain("does not know");
  });

  it("names the repository's own setting that stopped it, never a value", () => {
    const ssh = refusalWords({
      kind: "repository_config",
      key: "core.sshcommand",
      reason: "program",
    });
    expect(ssh.title).toBe(
      "Its setting core.sshcommand names a program this would run.",
    );
    expect(ssh.next).toContain("global git configuration");
    const whole = refusalWords({
      kind: "repository_config",
      reason: "too_large",
    });
    expect(whole.title).toContain("larger than Vessel reads");
  });
});

describe("worktrees and projects in plain words", () => {
  it("counts with the noun", () => {
    expect(counted(1, "file", "files")).toBe("1 file");
    expect(counted(3, "file", "files")).toBe("3 files");
  });

  it("says each reason a worktree was kept", () => {
    expect(keptReasonWords({ kind: "uncommitted", files: 2 })).toBe(
      "2 files with uncommitted changes",
    );
    expect(keptReasonWords({ kind: "unique_commits", commits: 1 })).toBe(
      "1 commit on no other branch",
    );
    expect(keptReasonWords({ kind: "operation", operation: "rebase" })).toBe(
      "A rebase in progress",
    );
  });

  it("offers a snapshot only for what a snapshot keeps", () => {
    expect(snapshotCovers({ kind: "uncommitted", files: 1 })).toBe(true);
    expect(snapshotCovers({ kind: "in_use" })).toBe(false);
    expect(keptReasonWords({ kind: "in_use" })).toContain("a terminal");
    expect(snapshotCovers({ kind: "submodules" })).toBe(false);
  });

  it("names the setup command that failed", () => {
    expect(
      setupFailureWords({ kind: "exit", command: 1, code: 2 }, [
        "npm ci",
        "npm run build",
      ]),
    ).toBe("`npm run build` exited with 2.");
    expect(setupFailureWords({ kind: "exit", command: 4, code: null })).toBe(
      "Setup command 5 was stopped by a signal.",
    );
  });

  it("shows a clone's stage and percent, and nothing for a ready project", () => {
    expect(
      projectStateWord({ state: "cloning", stage: "receiving", percent: 42 }),
    ).toBe("Receiving objects · 42%");
    expect(projectStateWord({ state: "ready" })).toBeNull();
    expect(cloneFailureWords("needs_credential")).toContain("credential");
  });

  it("says when the machine's git cannot be used", () => {
    expect(
      gitSupportWords({
        state: "too_old",
        version: { major: 2, minor: 17, patch: 1 },
        minimum: { major: 2, minor: 25, patch: 0 },
      }),
    ).toContain("2.17 is too old");
    expect(
      gitSupportWords({
        state: "found",
        version: { major: 2, minor: 43, patch: 0 },
      }),
    ).toBeNull();
  });

  it("says why a worktree was not made", () => {
    expect(
      worktreeRefusalWords({ kind: "checked_out_elsewhere", path: "/w/x" }),
    ).toContain("/w/x");
  });
});
