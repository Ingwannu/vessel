// git's writes from the Changes tab, and what each answered: a success
// said once (a toast that leaves), a refusal said plainly and kept above
// the list until the next action or a dismissal (the brief's "none is a
// toast that says failed"). One write at a time: the node takes the
// index lock for most of them anyway.

import { useCallback, useState } from "react";

import { useFleet } from "../../context";
import { nextStep, type Outcome, type Problem } from "../../data/outcome";
import {
  commit,
  discard,
  fetch,
  pull,
  push,
  renameBranch,
  stage,
  stash,
  switchBranch,
  unstage,
} from "../../data/workspace/git-writes";
import {
  refusalWords,
  type RefusalWords,
} from "../../data/workspace/git-words";
import { nodeError } from "../../data/workspace/root";
import { toast } from "../../ui/toasts";
import type { NodeKey } from "../../wire/bridge";
import type {
  BranchName,
  CommitStage,
  GitRefusal,
  GitStatus,
  PathSet,
  StashAction,
  WorkspaceRoot,
  WorktreeBase,
} from "../../wire/proto";

/** What a write's answer means for the page. */
type Done =
  | { kind: "said"; text: string }
  | { kind: "status"; status: GitStatus }
  | { kind: "refused"; refusal: GitRefusal };

/** A failure of the call itself, said as a refusal is. */
function problemWords(problem: Problem): RefusalWords {
  if (nodeError(problem) === "unavailable") {
    return {
      title: "Another git is working in this repository.",
      next: "It holds the index lock; try again in a moment.",
      paths: [],
      output: null,
    };
  }
  return {
    title: problem.message,
    next: nextStep(problem),
    paths: [],
    output: null,
  };
}

export interface Git {
  /** The write running now, by name. */
  busy: string | null;
  notice: RefusalWords | null;
  dismiss: () => void;
  commit: (message: string, stageFirst: CommitStage) => Promise<boolean>;
  push: (setUpstream: boolean) => void;
  pull: () => void;
  fetch: () => void;
  switchTo: (
    branch: BranchName,
    create: WorktreeBase | null,
  ) => Promise<boolean>;
  rename: (to: BranchName) => Promise<boolean>;
  stash: (action: StashAction) => Promise<boolean>;
  stage: (paths: PathSet) => void;
  unstage: (paths: PathSet) => void;
  discard: (paths: PathSet, untracked: boolean) => Promise<boolean>;
}

export function useGit(
  node: NodeKey,
  root: WorkspaceRoot,
  took: (status: GitStatus) => void,
  refresh: () => void,
): Git {
  const fleet = useFleet();
  const [busy, setBusy] = useState<string | null>(null);
  const [notice, setNotice] = useState<RefusalWords | null>(null);

  const run = useCallback(
    async <T>(
      name: string,
      call: () => Promise<Outcome<T>>,
      read: (value: T) => Done,
    ): Promise<boolean> => {
      setBusy(name);
      setNotice(null);
      const outcome = await call();
      setBusy(null);
      if (!outcome.ok) {
        setNotice(problemWords(outcome.problem));
        refresh();
        return false;
      }
      const done = read(outcome.value);
      switch (done.kind) {
        case "said":
          toast(done.text);
          refresh();
          return true;
        case "status":
          took(done.status);
          return true;
        case "refused":
          setNotice(refusalWords(done.refusal));
          refresh();
          return false;
      }
    },
    [took, refresh],
  );

  return {
    busy,
    notice,
    dismiss: useCallback(() => {
      setNotice(null);
    }, []),
    commit: (message, stageFirst) =>
      run(
        "commit",
        () => commit(fleet, node, root, message, stageFirst),
        (value) =>
          value.outcome === "committed"
            ? {
                kind: "said",
                text: `Committed ${value.commit.id.slice(0, 7)} · ${value.commit.subject}`,
              }
            : { kind: "refused", refusal: value.refusal },
      ),
    push: (setUpstream) => {
      void run(
        "push",
        () => push(fleet, node, root, setUpstream),
        (value) =>
          value.outcome === "pushed"
            ? {
                kind: "said",
                text: `Pushed to ${value.remote}/${value.branch}${value.upstream_set ? " and set it as the upstream" : ""}`,
              }
            : { kind: "refused", refusal: value.refusal },
      );
    },
    pull: () => {
      void run(
        "pull",
        () => pull(fleet, node, root),
        (value) => {
          switch (value.outcome) {
            case "pulled":
              return {
                kind: "said",
                text: `Pulled ${value.from.slice(0, 7)} → ${value.to.slice(0, 7)}`,
              };
            case "up_to_date":
              return { kind: "said", text: "Already up to date" };
            case "refused":
              return { kind: "refused", refusal: value.refusal };
          }
        },
      );
    },
    fetch: () => {
      void run(
        "fetch",
        () => fetch(fleet, node, root),
        (value) =>
          value.outcome === "fetched"
            ? { kind: "said", text: "Fetched" }
            : { kind: "refused", refusal: value.refusal },
      );
    },
    switchTo: (branch, create) =>
      run(
        "switch",
        () => switchBranch(fleet, node, root, branch, create),
        (value) =>
          value.outcome === "switched"
            ? { kind: "said", text: `Switched to ${value.branch}` }
            : { kind: "refused", refusal: value.refusal },
      ),
    rename: (to) =>
      run(
        "rename",
        () => renameBranch(fleet, node, root, to),
        (value) =>
          value.outcome === "renamed"
            ? { kind: "said", text: `Renamed the branch to ${value.branch}` }
            : { kind: "refused", refusal: value.refusal },
      ),
    stash: (action) =>
      run(
        "stash",
        () => stash(fleet, node, root, action),
        (value) =>
          value.outcome === "done"
            ? { kind: "said", text: stashSaid(action) }
            : { kind: "refused", refusal: value.refusal },
      ),
    stage: (paths) => {
      void run(
        "stage",
        () => stage(fleet, node, root, paths),
        (status) => ({
          kind: "status",
          status,
        }),
      );
    },
    unstage: (paths) => {
      void run(
        "unstage",
        () => unstage(fleet, node, root, paths),
        (status) => ({ kind: "status", status }),
      );
    },
    discard: (paths, untracked) =>
      run(
        "discard",
        () => discard(fleet, node, root, paths, untracked),
        (status) => ({ kind: "status", status }),
      ),
  };
}

/** What a stash action did, in words. */
function stashSaid(action: StashAction): string {
  switch (action.kind) {
    case "save":
      return "Stashed the changes";
    case "apply":
      return "Applied the stash entry";
    case "pop":
      return "Applied and dropped the stash entry";
    case "drop":
      return "Dropped the stash entry";
  }
}
