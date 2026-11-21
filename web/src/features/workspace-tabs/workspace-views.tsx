import type { ReactNode } from "react";

import type { ProjectsView } from "../../data/workspace/project-store";
import { pullReach } from "../../data/workspace/pull/served";
import { tabKey, type WorkspaceTab } from "../../data/workspace/tab-layout";
import { Button, IconButton, Skeleton } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type {
  SessionInfo,
  SessionPlace,
  WorkspaceRoot,
} from "../../wire/proto";
import { NewSession } from "../compose/new-session";
import { PullPane } from "../workspace/pull/pull-pane";
import { OpenChanges, WorkspaceChat } from "../workspace/workspace-chat";
import type { TabItem } from "./chrome-parts";
import { CodePane } from "./file-pane";
import { TerminalPane } from "./terminal-pane";
import type { useWorkspace } from "./use-workspace";
import type { AgentPaneActions } from "./workspace";
import type { TabWords } from "./words";

interface WorkspaceViewsProps {
  node: NodeKey;
  anchor: SessionInfo;
  root: WorkspaceRoot | null;
  projects: ProjectsView;
  scope: string;
  state: ReturnType<typeof useWorkspace>;
  words: TabWords;
  items: readonly TabItem[];
  renderAgent: (session: SessionInfo, actions: AgentPaneActions) => ReactNode;
  agentActions: AgentPaneActions;
}

export function WorkspaceViews({
  node,
  anchor,
  root,
  projects,
  scope,
  state,
  words,
  items,
  renderAgent,
  agentActions,
}: WorkspaceViewsProps) {
  const { layout, lifecycle, location, select, markDirty } = state;
  const place: SessionPlace | null =
    root === null
      ? null
      : root.kind === "worktree"
        ? { kind: "worktree", worktree: root.worktree }
        : { kind: "project", project: root.project };
  const openChat = () => {
    select({ kind: "agent", id: anchor.id });
  };

  const body = (tab: WorkspaceTab, visible: boolean, item: TabItem) => {
    const key = tabKey(tab);
    switch (tab.kind) {
      case "agent": {
        const session = state.session(tab.id);
        return visible ? (
          session === undefined ? (
            <Skeleton />
          ) : (
            <OpenChanges
              value={() => {
                location.openChanges("changes");
              }}
            >
              {renderAgent(session, agentActions)}
            </OpenChanges>
          )
        ) : null;
      }
      case "terminal":
        return visible ? (
          <TerminalPane
            node={node}
            terminal={tab.id}
            connected={state.protocol !== null}
            words={words}
          />
        ) : null;
      case "draft":
        return visible || state.visited.has(key) ? (
          <NewSession
            wanted={node}
            workspace={{
              place,
              folder: anchor.spec.cwd ?? null,
              tray: scope,
              done: state.completeDraft,
              onDirtyChange: (dirty) => {
                markDirty(key, dirty);
              },
            }}
          />
        ) : null;
      case "pull":
        return visible ? (
          <PullPane
            node={node}
            session={anchor}
            reach={pullReach(state.protocol)}
            openChat={openChat}
          />
        ) : null;
      case "files":
      case "changes":
      case "diff":
      case "file":
        // Keep the same file tree or editor when it moves across panes.
        return visible ||
          ((tab.kind === "file" || tab.kind === "files") &&
            state.visited.has(key)) ? (
          <WorkspaceChat value={openChat}>
            <CodePane
              node={node}
              session={anchor}
              root={root}
              projects={projects}
              tab={tab}
              active={visible}
              goTo={
                tab.kind === "file"
                  ? (location.lines.get(tab.path) ?? null)
                  : null
              }
              open={(path, line) => {
                const source =
                  tab.kind === "files"
                    ? "explorer_files"
                    : tab.kind === "diff" || tab.kind === "changes"
                      ? "diff_files"
                      : undefined;
                location.openFile(path, line, source);
              }}
              back={() => {
                if (location.supporting?.key === key) {
                  lifecycle.askClose(item);
                } else {
                  select({ kind: "files" });
                }
              }}
              markDirty={markDirty}
            />
          </WorkspaceChat>
        ) : null;
    }
  };

  return (
    <div className="workspace-views" data-support={location.supporting?.place}>
      {items.map((item) => {
        const key = tabKey(item.tab);
        const support =
          location.supporting?.key === key ? location.supporting.place : null;
        const visible = layout.active === key || support !== null;
        return (
          <div
            key={key}
            className="workspace-pane"
            role="tabpanel"
            aria-label={item.label}
            hidden={!visible}
            data-placement={support ?? "main"}
          >
            {support === null ? null : (
              <div className="workspace-support-header">
                {support === "explorer" ? (
                  <Button
                    icon="folder"
                    variant="ghost"
                    size="xs"
                    disabled={item.tab.kind === "files"}
                    onClick={location.openExplorer}
                  >
                    {words.files}
                  </Button>
                ) : null}
                <span>{item.label}</span>
                <IconButton
                  icon="close"
                  label={words.close}
                  size="xs"
                  variant="ghost"
                  onClick={() => {
                    lifecycle.askClose(item);
                  }}
                />
              </div>
            )}
            {body(item.tab, visible, item)}
          </div>
        );
      })}
    </div>
  );
}
