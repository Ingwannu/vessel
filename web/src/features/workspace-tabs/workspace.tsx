// All views share a workspace tab host. Agent bodies are rendered by the
// route owner so the agent and legacy terminal retain their normal entry.

import { type ReactNode } from "react";

import { sessionName } from "../../data/titles";
import { WorkspaceChat } from "../workspace/workspace-chat";
import type { ProjectsView } from "../../data/workspace/project-store";
import { tabKey, type WorkspaceTab } from "../../data/workspace/tab-layout";
import { Button, Dialog, Skeleton } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type {
  SessionInfo,
  SessionPlace,
  WorkspaceRoot,
} from "../../wire/proto";
import { NewSession } from "../compose/new-session";
import { useMedia } from "../shell/use-media";
import { Launcher, TabChrome } from "./chrome";
import type { TabItem } from "./chrome-parts";
import { CodePane } from "./file-pane";
import { RenameTab } from "./rename";
import { TerminalPane } from "./terminal-pane";
import { useWorkspace } from "./use-workspace";
import { tabWords } from "./words";

interface WorkspaceProps {
  node: NodeKey;
  anchor: SessionInfo;
  root: WorkspaceRoot | null;
  projects: ProjectsView;
  scope: string;
  agent: (session: SessionInfo, openChanges: () => void) => ReactNode;
}

export function Workspace({
  node,
  anchor,
  root,
  projects,
  scope,
  agent,
}: WorkspaceProps) {
  const words = tabWords(navigator.language);
  const compact = useMedia("(width < 600px)");
  const state = useWorkspace(node, anchor, root, projects, scope, words);
  const { active, actions, lifecycle, layout, markDirty, openFile, select } =
    state;

  const items = layout.tabs.map((tab): TabItem => {
    const key = tabKey(tab);
    const dirty = state.dirty.has(key);
    switch (tab.kind) {
      case "agent": {
        const known = state.session(tab.id);
        return {
          tab,
          label: known === undefined ? words.agent : sessionName(known),
          icon: known?.spec.mode === "terminal" ? "terminal" : "message",
          dirty,
        };
      }
      case "terminal":
        return {
          tab,
          label:
            state.terminals.find((item) => item.id === tab.id)?.title ??
            words.terminal,
          icon: "terminal",
          dirty,
        };
      case "file":
        return {
          tab,
          label: tab.path.split("/").at(-1) ?? tab.path,
          icon: "file",
          dirty,
        };
      case "files":
        return { tab, label: words.files, icon: "folder", dirty };
      case "changes":
        return { tab, label: words.changes, icon: "git-compare", dirty };
      case "diff":
        return { tab, label: words.diff, icon: "git-compare", dirty };
      case "draft":
        return { tab, label: words.newTab, icon: "compose", dirty };
    }
  });
  const launch = compact
    ? actions.filter((action) => action.label !== words.diff)
    : actions;
  const openChanges = () => select({ kind: "changes" });
  const place: SessionPlace | null =
    root === null
      ? null
      : root.kind === "worktree"
        ? { kind: "worktree", worktree: root.worktree }
        : { kind: "project", project: root.project };

  const openChat = () => select({ kind: "agent", id: anchor.id });
  const body = (tab: WorkspaceTab) => {
    const key = tabKey(tab);
    const selected = key === layout.active;
    switch (tab.kind) {
      case "agent": {
        const session = state.session(tab.id);
        return selected ? (
          session === undefined ? (
            <Skeleton />
          ) : (
            agent(session, openChanges)
          )
        ) : null;
      }
      case "terminal":
        return selected ? (
          <TerminalPane
            node={node}
            terminal={tab.id}
            connected={state.protocol !== null}
            words={words}
          />
        ) : null;
      case "draft":
        return selected || state.visited.has(key) ? (
          <NewSession
            wanted={node}
            workspace={{
              place,
              folder: anchor.spec.cwd ?? null,
              tray: scope,
              done: state.completeDraft,
              onDirtyChange: (dirty) => markDirty(key, dirty),
            }}
          />
        ) : null;
      case "files":
      case "changes":
      case "diff":
      case "file":
        // File and Explorer panes keep editor/tree state across a switch.
        // Diff has independent reads and is only mounted when visible.
        return selected ||
          ((tab.kind === "file" || tab.kind === "files") &&
            state.visited.has(key)) ? (
          <WorkspaceChat value={openChat}>
            <CodePane
              node={node}
              session={anchor}
              root={root}
              projects={projects}
              tab={tab}
              active={selected}
              goTo={
                tab.kind === "file" ? (state.lines.get(tab.path) ?? null) : null
              }
              open={openFile}
              back={() => select({ kind: "files" })}
              markDirty={markDirty}
            />
          </WorkspaceChat>
        ) : null;
    }
  };

  return (
    <section className="workspace-tabs">
      <TabChrome
        items={items}
        active={layout.active}
        words={words}
        launch={launch}
        actions={lifecycle.actions}
        select={select}
        close={lifecycle.askClose}
        closeMany={lifecycle.bulk}
        move={state.move}
      />
      {active === undefined ? (
        <Launcher actions={launch} words={words} />
      ) : null}
      {layout.tabs.map((tab) => {
        const key = tabKey(tab);
        const selected = key === layout.active;
        return (
          <div
            key={key}
            className="workspace-pane"
            role="tabpanel"
            aria-label={
              items.find((item) => tabKey(item.tab) === key)?.label ??
              words.workspace
            }
            hidden={!selected}
          >
            {body(tab)}
          </div>
        );
      })}
      {lifecycle.confirmation === null ? null : (
        <Dialog
          title={lifecycle.confirmation.title}
          onClose={lifecycle.cancel}
          footer={
            <>
              <Button variant="ghost" onClick={lifecycle.cancel}>
                {words.cancel}
              </Button>
              <Button variant="danger-solid" onClick={lifecycle.confirm}>
                {lifecycle.confirmation.label}
              </Button>
            </>
          }
        >
          <p className="workspace-close-copy">
            {lifecycle.confirmation.message}
          </p>
        </Dialog>
      )}
      {state.leave.status === "blocked" ? (
        <Dialog
          title={words.leaveTitle}
          onClose={state.leave.reset}
          footer={
            <>
              <Button variant="ghost" onClick={state.leave.reset}>
                {words.cancel}
              </Button>
              <Button variant="danger-solid" onClick={state.leave.proceed}>
                {words.leave}
              </Button>
            </>
          }
        >
          <p>{words.fileBody}</p>
        </Dialog>
      ) : null}
      {lifecycle.renaming === null ? null : (
        <RenameTab
          key={tabKey(lifecycle.renaming.tab)}
          label={lifecycle.renaming.label}
          protocol={state.protocol}
          words={words}
          cancel={lifecycle.cancelRename}
          rename={lifecycle.rename}
        />
      )}
    </section>
  );
}
