// Starting a session in a new worktree, in the order the node wants
// (track B's "what the page track needs to know"): `worktree.create`; if
// the base's commands are not approved, the person approves them (or
// starts without setup) and it is asked again; then setup's output is
// followed with `worktree.output` until setup is no longer running; a
// failed setup is shown and the person chooses (run it again, start
// anyway, stop); only then `session.create { place: worktree }`, since a
// session in a worktree whose setup runs is refused.
//
// [Decision Log]
// - Purpose: "where a session runs" on the new-session page, with nothing
//   started by a side door: approval is a person's choice, never
//   automatic, and setup runs only through the node's own rule.
// - Constraint: the node answers `worktree.create` at once and runs setup
//   in the background (it continues when the page goes away); setup's
//   output is paged (64 KiB) with a wait of at most 30 s.
// - Choice: one async function that walks the steps and asks the page
//   (`Steps`) at each point a person decides; the page draws the step it
//   is told. A clone and an approval are never chained.
// - Cost: leaving the page while setup runs leaves the worktree made and
//   set up, with no session in it; Settings › Projects lists it.

import { decodeBytes } from "../../data/terminal";
import type { Fleet } from "../../data/fleet";
import type { Problem } from "../../data/outcome";
import {
  compareWithFolder,
  type Compared,
  type ListedCommands,
} from "../../data/workspace/command-changes";
import { type ProjectPlace, worktreeBranch } from "../../data/workspace/place";
import {
  approveCommands,
  projectCommands,
} from "../../data/workspace/projects";
import {
  appendText,
  EMPTY_LOG,
  OutputDecoder,
  type SetupLog,
} from "../../data/workspace/setup-log";
import { worktreeRefusalWords } from "../../data/workspace/worktree-words";
import {
  createWorktree,
  readSetupOutput,
  runSetup,
} from "../../data/workspace/worktrees";
import type { NodeKey } from "../../wire/bridge";
import type {
  SetupChoice,
  SetupState,
  WorktreeId,
  WorktreeInfo,
} from "../../wire/proto";

/** What the page shows while a worktree is made. */
export type Step =
  | { kind: "idle" }
  | { kind: "making" }
  | {
      kind: "approval";
      commands: ListedCommands;
      compared: Compared;
      busy: boolean;
    }
  | { kind: "refused"; message: string }
  | {
      kind: "setup";
      branch: string;
      setup: SetupState;
      log: SetupLog;
      commands: readonly string[];
    };

/** What the page answers when the person decides. */
export interface Steps {
  show: (step: Step) => void;
  /** The commands need an approval: approve them, start without setup,
   * or cancel. */
  approve: () => Promise<"approve" | "skip" | "cancel">;
  /** Setup failed: run it again, start anyway, or stop. */
  failed: () => Promise<"retry" | "start" | "stop">;
}

/** How a worktree's making ended. */
export type Made =
  | { kind: "ready"; worktree: WorktreeId }
  | { kind: "stopped" }
  | { kind: "failed"; problem: Problem };

/** The branch a worktree is on, in words. */
function branchOf(info: WorktreeInfo): string {
  return info.branch ?? info.created_branch ?? "a detached HEAD";
}

/** Follow `info`'s setup output to its end, showing each page. */
async function follow(
  fleet: Fleet,
  node: NodeKey,
  info: WorktreeInfo,
  commands: readonly string[],
  steps: Steps,
): Promise<SetupState | Problem> {
  const decoder = new OutputDecoder();
  let log = EMPTY_LOG;
  let from = 0;
  let setup = info.setup;
  steps.show({ kind: "setup", branch: branchOf(info), setup, log, commands });
  for (;;) {
    const page = await readSetupOutput(fleet, node, info.id, from, true);
    if (!page.ok) {
      return page.problem;
    }
    log = appendText(log, decoder.text(decodeBytes(page.value.bytes)));
    from = page.value.next;
    setup = page.value.setup;
    steps.show({ kind: "setup", branch: branchOf(info), setup, log, commands });
    if (setup.state !== "running" && page.value.bytes === "") {
      return setup;
    }
  }
}

/** Make a worktree for `place` on `node` and set it up, asking the
 * person where they decide. */
export async function makeWorktree(
  fleet: Fleet,
  node: NodeKey,
  place: ProjectPlace,
  steps: Steps,
): Promise<Made> {
  const branch = worktreeBranch(place);
  if (branch === null) {
    return { kind: "stopped" };
  }
  let setupChoice: SetupChoice = place.setup;
  let commands: readonly string[] = [];
  for (;;) {
    steps.show({ kind: "making" });
    const made = await createWorktree(
      fleet,
      node,
      place.project,
      branch,
      setupChoice,
    );
    if (!made.ok) {
      return { kind: "failed", problem: made.problem };
    }
    const outcome = made.value;
    if (outcome.outcome === "refused") {
      steps.show({
        kind: "refused",
        message: worktreeRefusalWords(outcome.reason),
      });
      return { kind: "stopped" };
    }
    if (outcome.outcome === "needs_approval") {
      const listed = outcome.commands;
      if (listed.state !== "listed") {
        // Only a listed set needs an approval; anything else is a refusal
        // the page cannot act on.
        steps.show({
          kind: "refused",
          message: "The project's commands could not be read.",
        });
        return { kind: "stopped" };
      }
      const folder = await projectCommands(fleet, node, place.project);
      const compared = compareWithFolder(
        listed,
        folder.ok ? folder.value : null,
      );
      steps.show({ kind: "approval", commands: listed, compared, busy: false });
      const choice = await steps.approve();
      if (choice === "cancel") {
        steps.show({ kind: "idle" });
        return { kind: "stopped" };
      }
      if (choice === "skip") {
        setupChoice = "skip";
        continue;
      }
      steps.show({ kind: "approval", commands: listed, compared, busy: true });
      const approved = await approveCommands(
        fleet,
        node,
        place.project,
        listed.digest,
        true,
      );
      if (!approved.ok) {
        return { kind: "failed", problem: approved.problem };
      }
      commands = listed.setup;
      continue;
    }
    let info = outcome.worktree;
    for (;;) {
      const ended = await follow(fleet, node, info, commands, steps);
      if (!("state" in ended)) {
        return { kind: "failed", problem: ended };
      }
      if (ended.state !== "failed") {
        return { kind: "ready", worktree: info.id };
      }
      const choice = await steps.failed();
      if (choice === "start") {
        return { kind: "ready", worktree: info.id };
      }
      if (choice === "stop") {
        steps.show({ kind: "idle" });
        return { kind: "stopped" };
      }
      const again = await runSetup(fleet, node, info.id);
      if (!again.ok) {
        return { kind: "failed", problem: again.problem };
      }
      info = again.value;
    }
  }
}
