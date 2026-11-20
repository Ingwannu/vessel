// The code panel's Changes tab (agent-orchestrator's review pane, GOAT's
// diff panel and git control): the branch and git's actions, the
// comparison and its counts with the view's toggles, a refusal said
// plainly, then every changed file's diff.

import "./changes.css";
import "./diff.css";

import { useState } from "react";

import { scopeEmpty, totals } from "../../data/workspace/changes";
import { nodeError } from "../../data/workspace/root";
import { EmptyState, IconButton, Skeleton, Tooltip } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type {
  DiffScope,
  FileChange,
  RelPath,
  SessionId,
  WorkspaceRoot,
} from "../../wire/proto";
import { ChangesList } from "./changes-list";
import { GitBar, type GitDialog } from "./git-bar";
import { GitDialogs } from "./git-dialogs";
import { GitNotice } from "./git-notice";
import { ScopeMenu } from "./scope-menu";
import { useChanges } from "./use-changes";
import { useGit } from "./use-git";

/** Whether a file's header offers staging or unstaging in `scope`. */
function indexAction(scope: DiffScope): "stage" | "unstage" | null {
  switch (scope.kind) {
    case "unstaged":
    case "uncommitted":
      return "stage";
    case "staged":
      return "unstage";
    case "base":
    case "commit":
      return null;
  }
}

/** Whether a file can be discarded from `scope` (only what is not
 * committed). */
function discardable(scope: DiffScope): boolean {
  return scope.kind === "unstaged" || scope.kind === "uncommitted";
}

function ChangesBody({
  node,
  session,
  root,
  scope,
  onScope,
  whitespace,
  onWhitespace,
  onOpen,
  defaultBranch,
  view,
}: {
  node: NodeKey;
  session: SessionId;
  root: WorkspaceRoot;
  scope: DiffScope;
  onScope: (scope: DiffScope) => void;
  whitespace: boolean;
  onWhitespace: (on: boolean) => void;
  onOpen: (path: RelPath) => void;
  defaultBranch: string | null;
  view: "changes" | "diff";
}) {
  const changes = useChanges(node, root, scope, whitespace);
  const git = useGit(node, root, changes.took, changes.refresh);
  const [dialog, setDialog] = useState<GitDialog | null>(null);
  const [discarding, setDiscarding] = useState<FileChange | null>(null);
  const [toggled, setToggled] = useState<ReadonlySet<RelPath>>(() => new Set());
  const summary = changes.summary;
  const files = summary.kind === "ready" ? summary.summary.files : [];
  const counts = totals(files);
  const closed =
    view === "changes"
      ? new Set(
          files
            .filter((file) => !toggled.has(file.path))
            .map((file) => file.path),
        )
      : toggled;
  const collapsed = closed.size > 0;
  const status = changes.status;
  const untracked = (path: RelPath) =>
    status?.files.some(
      (file) => file.path === path && file.worktree === "untracked",
    ) ?? false;
  // Discarding a file that is not in the last commit deletes it: an
  // untracked one (git clean), and an added one (restored from HEAD,
  // where it is not).
  const deletes = (path: RelPath) =>
    status?.files.some(
      (file) =>
        file.path === path &&
        (file.worktree === "untracked" || file.index === "added"),
    ) ?? false;
  return (
    <div className="changes">
      {status === null ? null : (
        <GitBar status={status} git={git} onDialog={setDialog} />
      )}
      <div className="changes-tools">
        <ScopeMenu
          node={node}
          root={root}
          scope={scope}
          onScope={onScope}
          added={counts.added}
          removed={counts.removed}
        />
        <span className="changes-tools-end">
          <Tooltip
            label={
              whitespace ? "Show whitespace changes" : "Hide whitespace changes"
            }
          >
            <IconButton
              label="Hide whitespace changes"
              icon="pilcrow"
              size="xs"
              variant="ghost"
              aria-pressed={whitespace}
              onClick={() => {
                onWhitespace(!whitespace);
              }}
            />
          </Tooltip>
          <Tooltip
            label={collapsed ? "Expand all files" : "Collapse all files"}
          >
            <IconButton
              label={collapsed ? "Expand all files" : "Collapse all files"}
              icon={collapsed ? "unfold" : "fold"}
              size="xs"
              variant="ghost"
              onClick={() => {
                const all = (view === "changes") === collapsed;
                setToggled(
                  all ? new Set(files.map((file) => file.path)) : new Set(),
                );
              }}
            />
          </Tooltip>
        </span>
      </div>
      {git.notice === null || dialog !== null || discarding !== null ? null : (
        <GitNotice notice={git.notice} onDismiss={git.dismiss} />
      )}
      {summary.kind === "loading" ? (
        <div className="changes-pad">
          <Skeleton />
        </div>
      ) : summary.kind === "failed" ? (
        <EmptyState
          title={
            nodeError(summary.problem) === "unsupported"
              ? "Not a git repository"
              : "Changes could not be read"
          }
          icon="git-compare"
        >
          {nodeError(summary.problem) === "unsupported"
            ? "This project's folder is not a git repository, so it has no changes to compare. Its files are in the Files tab."
            : summary.problem.message}
        </EmptyState>
      ) : files.length === 0 ? (
        <EmptyState title={scopeEmpty(scope)} icon="git-compare" />
      ) : (
        <ChangesList
          session={session}
          files={files}
          states={changes.states}
          want={changes.want}
          closed={closed}
          actions={{
            index: indexAction(scope),
            onIndex: (change) => {
              const paths = { kind: "paths" as const, paths: [change.path] };
              if (indexAction(scope) === "unstage") {
                git.unstage(paths);
              } else {
                git.stage(paths);
              }
            },
            onDiscard: discardable(scope)
              ? (change) => {
                  setDiscarding(change);
                }
              : null,
            onOpen: (change) => {
              onOpen(change.path);
            },
            onToggle: (change) => {
              setToggled((before) => {
                const next = new Set(before);
                if (!next.delete(change.path)) {
                  next.add(change.path);
                }
                return next;
              });
            },
          }}
        />
      )}
      {summary.kind === "ready" && summary.summary.more ? (
        <p className="changes-more">
          Only the first 5,000 files are listed; narrow the comparison to see
          the rest.
        </p>
      ) : null}
      <GitDialogs
        node={node}
        root={root}
        status={status}
        git={git}
        dialog={dialog}
        discarding={discarding}
        untracked={discarding !== null && untracked(discarding.path)}
        deletes={discarding !== null && deletes(discarding.path)}
        defaultBranch={defaultBranch}
        onClose={() => {
          setDialog(null);
          setDiscarding(null);
          git.dismiss();
        }}
      />
    </div>
  );
}

/** A comparison as text, to key the body by. */
function scopeKey(scope: DiffScope): string {
  return scope.kind === "commit" ? `commit:${scope.id}` : scope.kind;
}

export function ChangesTab({
  node,
  session,
  root,
  onOpen,
  defaultBranch,
  view,
}: {
  node: NodeKey;
  session: SessionId;
  root: WorkspaceRoot;
  onOpen: (path: RelPath) => void;
  /** The project's default branch, for the commit dialog's warning. */
  defaultBranch: string | null;
  view: "changes" | "diff";
}) {
  // A worktree's work is everything since its base (agent-orchestrator's
  // "Changes"); a project's own folder starts at what is not committed.
  const [scope, setScope] = useState<DiffScope>(
    root.kind === "worktree" && view === "changes"
      ? { kind: "base" }
      : { kind: "uncommitted" },
  );
  const [whitespace, setWhitespace] = useState(false);
  return (
    <ChangesBody
      key={`${scopeKey(scope)}:${String(whitespace)}`}
      node={node}
      session={session}
      root={root}
      scope={scope}
      onScope={setScope}
      whitespace={whitespace}
      onWhitespace={setWhitespace}
      onOpen={onOpen}
      defaultBranch={defaultBranch}
      view={view}
    />
  );
}
