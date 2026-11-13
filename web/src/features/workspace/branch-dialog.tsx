// Switching, creating and renaming a branch (GOAT's branch picker and
// Paseo's "Switch branch…"): a filter over the branches the node lists,
// each with its badges (current, default, remote, in a worktree); or a
// name, checked with git's rules as it is typed. A refusal (changes that
// would be overwritten, a branch checked out elsewhere) is said in place.

import { useEffect, useMemo, useState } from "react";

import { useFleet } from "../../context";
import { branchNameProblem } from "../../data/workspace/branch-name";
import { readBranches } from "../../data/workspace/git";
import {
  Badge,
  Button,
  Dialog,
  Field,
  Icon,
  Spinner,
  TextField,
} from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type { Branch, GitStatus, WorkspaceRoot } from "../../wire/proto";
import { GitNotice } from "./git-notice";
import type { Git } from "./use-git";

/** How many branches the list draws (the node lists up to 1,000). */
const SHOWN = 200;

function BranchList({
  node,
  root,
  git,
  onClose,
}: {
  node: NodeKey;
  root: WorkspaceRoot;
  git: Git;
  onClose: () => void;
}) {
  const fleet = useFleet();
  const [branches, setBranches] = useState<readonly Branch[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [filter, setFilter] = useState("");
  useEffect(() => {
    let current = true;
    void readBranches(fleet, node, root).then((outcome) => {
      if (!current) {
        return;
      }
      if (outcome.ok) {
        setBranches(outcome.value.branches);
      } else {
        setError(outcome.problem.message);
      }
    });
    return () => {
      current = false;
    };
  }, [fleet, node, root]);
  const shown = useMemo(() => {
    const needle = filter.trim().toLowerCase();
    return (branches ?? [])
      .filter((branch) => branch.name.toLowerCase().includes(needle))
      .slice(0, SHOWN);
  }, [branches, filter]);
  if (error !== null) {
    return <p className="branch-empty">{error}</p>;
  }
  if (branches === null) {
    return (
      <p className="branch-empty">
        <Spinner size={12} /> Loading branches…
      </p>
    );
  }
  return (
    <>
      <TextField
        value={filter}
        onChange={setFilter}
        placeholder="Filter branches…"
        icon="search"
        size="sm"
        aria-label="Filter branches"
        autoFocus
      />
      {shown.length === 0 ? (
        <p className="branch-empty">No branches found.</p>
      ) : (
        <ul className="branch-list" aria-label="Branches">
          {shown.map((branch) => (
            <li key={branch.name}>
              <button
                type="button"
                className="branch-row"
                disabled={branch.current || git.busy !== null}
                aria-current={branch.current ? "true" : undefined}
                onClick={() => {
                  // A remote-tracking branch is checked out as a local one
                  // of the same name, starting from it.
                  const remote = branch.remote ?? null;
                  const local =
                    remote === null
                      ? branch.name
                      : branch.name.slice(remote.length + 1);
                  void git
                    .switchTo(
                      local,
                      remote === null
                        ? null
                        : { kind: "ref", name: branch.name },
                    )
                    .then((done) => {
                      if (done) {
                        onClose();
                      }
                    });
                }}
              >
                <Icon name="git-branch" size={14} />
                <span className="branch-name">{branch.name}</span>
                <span className="branch-badges">
                  {branch.current ? <Badge>current</Badge> : null}
                  {branch.default ? <Badge>default</Badge> : null}
                  {branch.remote === null ||
                  branch.remote === undefined ? null : (
                    <Badge>remote</Badge>
                  )}
                  {branch.worktree === null ||
                  branch.worktree === undefined ? null : (
                    <Badge>worktree</Badge>
                  )}
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </>
  );
}

function NameForm({
  label,
  initial,
  action,
  onSubmit,
  busy,
}: {
  label: string;
  initial: string;
  action: string;
  onSubmit: (name: string) => void;
  busy: boolean;
}) {
  const [name, setName] = useState(initial);
  const problem = name.trim() === "" ? null : branchNameProblem(name.trim());
  const ready = name.trim() !== "" && problem === null && !busy;
  return (
    <form
      className="branch-form"
      onSubmit={(event) => {
        event.preventDefault();
        if (ready) {
          onSubmit(name.trim());
        }
      }}
    >
      <Field label={label} error={problem}>
        <TextField value={name} onChange={setName} mono autoFocus />
      </Field>
      <Button type="submit" variant="primary" disabled={!ready} busy={busy}>
        {action}
      </Button>
    </form>
  );
}

export function BranchDialog({
  mode,
  node,
  root,
  status,
  git,
  onClose,
}: {
  mode: "switch" | "create" | "rename";
  node: NodeKey;
  root: WorkspaceRoot;
  status: GitStatus;
  git: Git;
  onClose: () => void;
}) {
  const close = (done: boolean) => {
    if (done) {
      onClose();
    }
  };
  return (
    <Dialog
      title={
        mode === "switch"
          ? "Switch branch"
          : mode === "create"
            ? "New branch"
            : "Rename branch"
      }
      onClose={onClose}
    >
      {mode === "switch" ? (
        <BranchList node={node} root={root} git={git} onClose={onClose} />
      ) : mode === "create" ? (
        <>
          <p className="branch-from">
            <Icon name="git-branch" size={14} /> From{" "}
            {status.branch ?? status.head?.slice(0, 7) ?? "HEAD"}
          </p>
          <NameForm
            label="Branch name"
            initial=""
            action="Create and switch"
            busy={git.busy === "switch"}
            onSubmit={(name) => {
              void git
                .switchTo(name, { kind: "ref", name: "HEAD" })
                .then(close);
            }}
          />
        </>
      ) : (
        <NameForm
          label="New name"
          initial={status.branch ?? ""}
          action="Rename"
          busy={git.busy === "rename"}
          onSubmit={(name) => {
            void git.rename(name).then(close);
          }}
        />
      )}
      {git.notice === null ? null : <GitNotice notice={git.notice} />}
    </Dialog>
  );
}
