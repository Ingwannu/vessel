// What a project's, a clone's, a worktree's and setup's states mean, in
// the page's words (one sentence each, the action they leave in the
// reader's hands), and why a worktree was kept (agent-orchestrator's and
// GOAT's "preserved" reasons, as a list a person chooses from).
//
// [Decision Log]
// - Purpose: the node names every state and reason as a closed set
//   (`ProjectState`, `CloneStage`, `SetupState`, `KeptReason`,
//   `WorktreeRefusal`); a screen must never show the set's raw names
//   ("words on the page": no `worktree_refs` on a button).
// - Choice: pure functions here, one per set, exhaustive; screens draw
//   their results. Counts are written with their noun ("3 files").
// - Cost: a new variant on the wire breaks the build until it has words.

import type {
  CloneFailure,
  CloneStage,
  GitSupport,
  KeptReason,
  ProjectState,
  SetupFailure,
  SetupState,
  WorktreeRefusal,
} from "../../wire/proto";
import { operationWord } from "./git-words";

/** `count` with its noun, singular when one. */
export function counted(count: number, one: string, many: string): string {
  return `${String(count)} ${count === 1 ? one : many}`;
}

/** `text` with its first letter in capitals. */
function sentence(text: string): string {
  return `${text.slice(0, 1).toUpperCase()}${text.slice(1)}`;
}

/** What a clone is doing. */
export function cloneStageWord(stage: CloneStage): string {
  switch (stage) {
    case "connecting":
      return "Connecting";
    case "counting":
      return "Counting objects";
    case "compressing":
      return "Compressing objects";
    case "receiving":
      return "Receiving objects";
    case "resolving":
      return "Resolving deltas";
    case "checking_out":
      return "Checking out files";
    case "unknown":
      return "Cloning";
  }
}

/** Why a clone failed, and what to do. */
export function cloneFailureWords(reason: CloneFailure): string {
  switch (reason) {
    case "needs_credential":
      return "The remote asked for a credential. Sign in to it on that machine (a credential helper or an SSH key), then clone again.";
    case "not_found":
      return "No repository was found at that address. Check it and clone again.";
    case "network":
      return "The machine could not reach the remote. Check its network, then clone again.";
    case "timed_out":
      return "The clone took longer than an hour and was stopped.";
    case "other":
    case "unknown":
      return "Git could not clone it. What it said is below.";
  }
}

/** A project's state in one short phrase, `null` when it is ready (a
 * ready project says nothing about itself). */
export function projectStateWord(state: ProjectState): string | null {
  switch (state.state) {
    case "ready":
      return null;
    case "cloning":
      return state.percent === null || state.percent === undefined
        ? cloneStageWord(state.stage)
        : `${cloneStageWord(state.stage)} · ${String(state.percent)}%`;
    case "clone_failed":
      return "Clone failed";
    case "missing":
      return "Folder missing";
    case "unknown":
      return "Unknown state";
  }
}

/** The machine's git, when it cannot be used, said plainly. */
export function gitSupportWords(git: GitSupport): string | null {
  switch (git.state) {
    case "found":
      return null;
    case "too_old":
      return `Git ${String(git.version.major)}.${String(git.version.minor)} is too old on this machine; Vessel needs ${String(git.minimum.major)}.${String(git.minimum.minor)} or newer for projects.`;
    case "missing":
      return "Git is not installed on this machine: folders can be added, but not cloned, and they get no worktrees or changes.";
    case "unknown":
      return null;
  }
}

/** Why setup failed. `commands` are the setup commands, to name the one
 * that failed when the node says which. */
export function setupFailureWords(
  failure: SetupFailure,
  commands: readonly string[] = [],
): string {
  switch (failure.kind) {
    case "exit": {
      const command = commands[failure.command];
      const code =
        failure.code === null || failure.code === undefined
          ? "was stopped by a signal"
          : `exited with ${String(failure.code)}`;
      return command === undefined
        ? `Setup command ${String(failure.command + 1)} ${code}.`
        : `\`${command}\` ${code}.`;
    }
    case "timed_out":
      return "Setup ran for 30 minutes and was stopped.";
    case "spawn":
      return "The machine could not start the setup command (its shell).";
    case "unknown":
      return "Setup failed.";
  }
}

/** A worktree's setup in one short phrase. */
export function setupWord(setup: SetupState): string {
  switch (setup.state) {
    case "none":
      return "No setup";
    case "skipped":
      return "Setup skipped";
    case "running":
      return "Setting up";
    case "succeeded":
      return "Set up";
    case "failed":
      return "Setup failed";
    case "unknown":
      return "Unknown";
  }
}

/** What would be lost, one line per reason. */
export function keptReasonWords(reason: KeptReason): string {
  switch (reason.kind) {
    case "uncommitted":
      return `${counted(reason.files, "file", "files")} with uncommitted changes`;
    case "unique_commits":
      return `${counted(reason.commits, "commit", "commits")} on no other branch`;
    case "operation":
      return sentence(`${operationWord(reason.operation)} in progress`);
    case "submodules":
      return "Submodules, whose changes are not inspected";
    case "flagged_files":
      return "Files marked so that git does not show their changes";
    case "worktree_refs":
      return "References of this worktree's own";
    case "in_use":
      // From protocol 11 a terminal or a service beside a session in the
      // worktree keeps it too (the node join of wave 2's first half).
      return "A live session, a terminal or service, or setup is using it";
    case "inspection_failed":
      return "Git could not inspect it";
    case "unknown":
      return "Something this page does not know";
  }
}

/** Whether a snapshot can keep what `reason` names: a session in it or
 * a failed inspection is not something a snapshot saves. */
export function snapshotCovers(reason: KeptReason): boolean {
  switch (reason.kind) {
    case "uncommitted":
    case "unique_commits":
    case "flagged_files":
    case "worktree_refs":
      return true;
    case "operation":
    case "submodules":
    case "in_use":
    case "inspection_failed":
    case "unknown":
      return false;
  }
}

/** Why a worktree was not made, and what to do. */
export function worktreeRefusalWords(reason: WorktreeRefusal): string {
  switch (reason.kind) {
    case "not_git":
      return "This project is not a git repository, so it has no worktrees. Run the session in its folder instead.";
    case "not_ready":
      return "The project is not ready yet (cloning, or its folder is missing).";
    case "no_commit":
      return "The repository has no commit yet; commit once in its folder first.";
    case "branch_exists":
      return "A branch with that name exists. Choose another name, or run on the existing branch.";
    case "checked_out_elsewhere":
      return `That branch is checked out in another worktree (${reason.path}).`;
    case "no_such_base":
      return "The base branch was not found. Choose another, or fetch first.";
    case "no_such_branch":
      return "That branch was not found.";
    case "no_such_snapshot":
      return "That snapshot is gone.";
    case "limit":
      return "This project has as many worktrees as a machine keeps. Remove one first.";
    case "unknown":
      return "The machine refused to make the worktree for a reason this page does not know.";
  }
}

/** Paseo's terminal warning (`workspace.tabs.confirmations.bulk.terminals`,
 * workspace-bulk-close.ts), said before a worktree's removal ends the
 * `count` live terminals in it (ADR 0121). */
export function closesTerminals(count: number): string {
  return `This will close ${String(count)} terminal(s). Any running process in a closed terminal will be stopped immediately.`;
}
