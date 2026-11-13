// The Changes tab's head (GOAT's branch toolbar and git control): the
// branch checked out, how far it is from its upstream and its base, and
// one split button whose left half does what makes sense now (commit,
// push, pull, fetch; `data/workspace/git-quick.ts`) and whose menu holds
// the rest of git's actions, each off with its reason when it cannot run.

import "./git.css";

import {
  type GitAction,
  actionBlocked,
  actionLabel,
  quickAction,
} from "../../data/workspace/git-quick";
import {
  Icon,
  type IconName,
  MenuItem,
  MenuSeparator,
  SplitButton,
} from "../../ui/kit";
import type { GitStatus } from "../../wire/proto";
import type { Git } from "./use-git";

/** What the menu opens beside the plain actions. */
export type GitDialog = "commit" | "switch" | "create" | "rename" | "stashes";

function actionIcon(action: GitAction): IconName {
  switch (action) {
    case "commit":
      return "git-commit";
    case "push":
      return "upload";
    case "pull":
      return "download";
    case "fetch":
      return "refresh";
  }
}

function Distance({ status }: { status: GitStatus }) {
  const upstream = status.upstream ?? null;
  const base = status.base ?? null;
  return (
    <>
      {upstream === null ? (
        status.branch === null || status.branch === undefined ? null : (
          <span className="git-distance">not pushed</span>
        )
      ) : upstream.ahead === 0 && upstream.behind === 0 ? null : (
        <span
          className="git-distance"
          aria-label={`${String(upstream.ahead)} ahead, ${String(upstream.behind)} behind ${upstream.name}`}
        >
          {upstream.ahead > 0 ? `↑${String(upstream.ahead)}` : ""}
          {upstream.behind > 0 ? ` ↓${String(upstream.behind)}` : ""}
        </span>
      )}
      {base === null || (base.ahead === 0 && base.behind === 0) ? null : (
        <span className="git-distance">
          {String(base.ahead)} ahead of {base.name}
        </span>
      )}
    </>
  );
}

export function GitBar({
  status,
  git,
  onDialog,
}: {
  status: GitStatus;
  git: Git;
  onDialog: (dialog: GitDialog) => void;
}) {
  const quick = quickAction(status);
  const run = (action: GitAction) => {
    switch (action) {
      case "commit":
        onDialog("commit");
        return;
      case "push":
        git.push(status.upstream === null || status.upstream === undefined);
        return;
      case "pull":
        git.pull();
        return;
      case "fetch":
        git.fetch();
        return;
    }
  };
  const blocked = actionBlocked(quick, status);
  const item = (action: GitAction, label: string = actionLabel(action)) => {
    const reason = actionBlocked(action, status);
    return (
      <MenuItem
        icon={actionIcon(action)}
        disabled={reason !== null || git.busy !== null}
        {...(reason === null ? {} : { description: reason })}
        onSelect={() => {
          run(action);
        }}
      >
        {label}
      </MenuItem>
    );
  };
  const detached = status.branch === null || status.branch === undefined;
  const noUpstream = status.upstream === null || status.upstream === undefined;
  return (
    <div className="git-bar">
      <span className="git-branch">
        <Icon name="git-branch" size={14} />
        <span className="git-branch-name">
          {detached
            ? `Detached at ${status.head?.slice(0, 7) ?? "nothing"}`
            : status.branch}
        </span>
        <Distance status={status} />
      </span>
      <SplitButton
        label={actionLabel(quick)}
        icon={actionIcon(quick)}
        onClick={() => {
          run(quick);
        }}
        disabled={blocked !== null || git.busy !== null}
        busy={git.busy === quick}
        {...(blocked === null ? {} : { reason: blocked })}
        menuLabel="Git action options"
        menu={
          <>
            {item("commit", "Commit…")}
            {item(
              "push",
              noUpstream && !detached ? "Push and set upstream" : "Push",
            )}
            {item("pull")}
            {item("fetch")}
            <MenuSeparator />
            <MenuItem
              icon="git-branch"
              onSelect={() => {
                onDialog("switch");
              }}
            >
              Switch branch…
            </MenuItem>
            <MenuItem
              icon="plus"
              onSelect={() => {
                onDialog("create");
              }}
            >
              New branch…
            </MenuItem>
            <MenuItem
              icon="edit"
              disabled={detached}
              onSelect={() => {
                onDialog("rename");
              }}
            >
              Rename branch…
            </MenuItem>
            <MenuSeparator />
            <MenuItem
              icon="archive"
              disabled={status.files.length === 0 || git.busy !== null}
              onSelect={() => {
                void git.stash({ kind: "save", untracked: false });
              }}
            >
              Stash changes
            </MenuItem>
            <MenuItem
              icon="archive"
              disabled={status.files.length === 0 || git.busy !== null}
              onSelect={() => {
                void git.stash({ kind: "save", untracked: true });
              }}
            >
              Stash, with untracked files
            </MenuItem>
            <MenuItem
              icon="list"
              onSelect={() => {
                onDialog("stashes");
              }}
            >
              Stashes…
            </MenuItem>
          </>
        }
      />
    </div>
  );
}
