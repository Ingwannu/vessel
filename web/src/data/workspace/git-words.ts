// What git's refusals mean, in the page's words: a sentence for what
// happened and one for what to do (the brief's "every git write shows its
// typed failure in plain words; none is a toast that says 'failed'").
//
// [Decision Log]
// - Purpose: a commit, a push, a pull, a switch or a stash can be refused
//   for a dozen reasons the node already names (`GitRefusal`); the
//   person needs to know which, and what to do next.
// - Constraint: git's own stderr can echo a credential, so the node sends
//   a fixed reason instead (GOAT's choice too); only a hook's output and a
//   remote's message travel, with credentials removed by the node.
// - Alternatives: GOAT's toasts with a fixed title per reason (they leave
//   with the moment, and the paths a refusal names do not fit); showing
//   the node's message alone (it is written for the CLI).
// - Choice: a pure function from the refusal to a title, what to do, and
//   what it names (paths, a hook's output), drawn where the action was
//   asked (the panel's notice), until the next action or a dismissal.
// - Cost: every reason added to the wire needs a sentence here; the
//   exhaustive switch makes the compiler ask for it.

import type {
  ConfigKey,
  GitOperation,
  GitRefusal,
  RelPath,
  RepositoryConfigReason,
} from "../../wire/proto";

/** A refusal said plainly. */
export interface RefusalWords {
  /** What happened, one sentence. */
  title: string;
  /** What to do about it, one sentence. */
  next: string;
  /** The paths it concerns, at most 100 (the node's bound). */
  paths: readonly RelPath[];
  /** Text to show as it came (a hook's output, the remote's message). */
  output: string | null;
}

/** What `operation` is called on the page. */
export function operationWord(operation: GitOperation): string {
  switch (operation) {
    case "merge":
      return "a merge";
    case "rebase":
      return "a rebase";
    case "cherry_pick":
      return "a cherry-pick";
    case "revert":
      return "a revert";
    case "bisect":
      return "a bisect";
  }
}

function words(
  title: string,
  next: string,
  extra: { paths?: readonly RelPath[]; output?: string } = {},
): RefusalWords {
  return {
    title,
    next,
    paths: extra.paths ?? [],
    output: extra.output ?? null,
  };
}

/** Why the repository's own configuration stopped a run (ADR 0030), and
 * what moves it out of the way: the setting to the person's global
 * configuration, where Vessel's git runs with it. */
function configWords(
  key: ConfigKey | null,
  reason: RepositoryConfigReason,
): RefusalWords {
  const named = key === null ? "Its git configuration" : `Its setting ${key}`;
  const next =
    "Vessel runs git without programs a repository's own configuration names. Move the setting to your global git configuration (under includeIf to keep it to this repository), or run this in a terminal.";
  switch (reason) {
    case "program":
      return words(`${named} names a program this would run.`, next);
    case "layout":
      return words(
        `${named} moves the working tree out of the folder.`,
        "Remove it from the repository's configuration to use the folder from Vessel.",
      );
    case "credential":
      return words(`${named} could send your credential elsewhere.`, next);
    case "unnamable":
      return words(
        `${named} names a driver git cannot turn off for one run.`,
        "Rename the driver in the repository's configuration, or run this in a terminal.",
      );
    case "too_many":
      return words(
        "The repository's configuration names more drivers than Vessel turns off in one run.",
        "Remove the ones it does not need, or run this in a terminal.",
      );
    case "too_large":
      return words(
        "The repository's configuration is larger than Vessel reads before running git.",
        "Trim it, or run this in a terminal.",
      );
    case "unknown":
      return words(
        `${named} stopped it, for a reason this page does not know.`,
        "Update this page's Vessel to read it.",
      );
  }
}

/** `refusal` in plain words. */
export function refusalWords(refusal: GitRefusal): RefusalWords {
  switch (refusal.kind) {
    case "needs_credential":
      return words(
        "The remote asked for a credential.",
        "Sign in to it on that machine (a credential helper or an SSH key that needs no prompt), then try again.",
      );
    case "timed_out":
      return words(
        "Git took too long and was stopped.",
        "Check the machine's network or the remote, then try again.",
      );
    case "non_fast_forward":
      return words(
        "The remote has commits this branch does not.",
        "Pull first, then push again. Vessel never forces a push.",
      );
    case "diverged":
      return words(
        "This branch and its upstream have both moved on.",
        "Pulling only fast-forwards; merge or rebase in a terminal or ask the agent to.",
      );
    case "would_overwrite":
      return words(
        "Local changes would be overwritten.",
        "Commit or stash them first.",
        { paths: refusal.paths },
      );
    case "conflicts":
      return words(
        "This left conflicts to resolve.",
        "Resolve them in these files, then stage and commit.",
        { paths: refusal.paths },
      );
    case "nothing_to_commit":
      return words(
        "There is nothing to commit.",
        "Stage changes first, or commit everything.",
      );
    case "no_identity":
      return words(
        "Git has no name and email for commits on that machine.",
        "Set user.name and user.email in git's configuration there.",
      );
    case "hook_failed":
      return words(
        "A hook of the repository refused it.",
        "Read what the hook said, fix it, and try again.",
        { output: refusal.output },
      );
    case "no_upstream":
      return words("This branch has no upstream.", "Push it to set one.");
    case "detached":
      return words(
        "No branch is checked out.",
        "Switch to a branch or create one first.",
      );
    case "checked_out_elsewhere":
      return words(
        "That branch is checked out in another worktree.",
        `Use it there (${refusal.path}), or choose another branch.`,
      );
    case "nothing_to_stash":
      return words(
        "There is nothing to stash.",
        "Stash keeps uncommitted changes; there are none.",
      );
    case "stash_moved":
      return words(
        "The stash changed since it was listed.",
        "Look at the list again and choose the entry once more.",
      );
    case "branch_exists":
      return words(
        "A branch with that name exists.",
        "Choose another name, or switch to it.",
      );
    case "no_such_ref":
      return words(
        "No branch, tag or commit has that name.",
        "Check the name, or fetch first.",
      );
    case "operation":
      return words(
        `The repository is in the middle of ${operationWord(refusal.operation)}.`,
        "Finish or abort it first.",
      );
    case "remote_failed":
      return words("The remote refused it.", "Read what it said below.", {
        output: refusal.message,
      });
    case "repository_config":
      return configWords(refusal.key ?? null, refusal.reason);
    case "unknown":
      return words(
        "Git refused it for a reason this page does not know.",
        "Update this page's Vessel to read it.",
      );
  }
}
