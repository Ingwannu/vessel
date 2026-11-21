// The route owner supplies agent and legacy terminal bodies to the workspace.

import { type ReactNode, useCallback, useState } from "react";

import type { ProjectsView } from "../../data/workspace/project-store";
import { tabKey } from "../../data/workspace/tab-layout";
import { SCRIPT_PROTOCOL } from "../../data/workspace/scripts";
import { Button, Dialog } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type { SessionId, SessionInfo, WorkspaceRoot } from "../../wire/proto";
import { SessionBar } from "../session/session-bar";
import { useMedia } from "../shell/use-media";
import { Launcher, TabChrome } from "./chrome";
import { RenameTab } from "./rename";
import { ScriptsControl } from "./scripts-control";
import { tabItem } from "./tab-item";
import { useTerminalAttention } from "./use-terminal-attention";
import { useWorkspace } from "./use-workspace";
import { WorkspaceViews } from "./workspace-views";
import { tabWords } from "./words";

export interface AgentPaneActions {
  summaryOpen: boolean;
  closeSummary: () => void;
  onTitle: (id: SessionId, name: string | null) => void;
}

interface WorkspaceProps {
  node: NodeKey;
  label: string;
  anchor: SessionInfo;
  root: WorkspaceRoot | null;
  projects: ProjectsView;
  scope: string;
  agent: (session: SessionInfo, actions: AgentPaneActions) => ReactNode;
}

export function Workspace({
  node,
  label,
  anchor,
  root,
  projects,
  scope,
  agent,
}: WorkspaceProps) {
  const words = tabWords(navigator.language);
  const compact = useMedia("(width < 600px)");
  const narrowSummary = useMedia("(width < 1280px)");
  const [summaryOpen, setSummaryOpen] = useState(false);
  const [title, setTitle] = useState<{
    id: SessionId;
    name: string | null;
  } | null>(null);
  const onTitle = useCallback((id: SessionId, name: string | null) => {
    setTitle((current) =>
      current?.id === id && current.name === name ? current : { id, name },
    );
  }, []);
  const state = useWorkspace(node, anchor, root, projects, scope, words);
  const { active, actions, lifecycle, layout, location, select } = state;
  useTerminalAttention(node, active, state.terminals, select);
  const items = layout.tabs.map((tab) => tabItem(tab, state, words));
  const launch = compact
    ? actions.filter((action) => action.label !== words.diff)
    : actions;
  const heading = state.heading;
  const scriptProject =
    root?.kind === "project" ? root.project : anchor.workspace?.project;
  const scriptSession = root?.kind === "worktree" ? anchor.id : null;
  const openSummary = () => {
    select({ kind: "agent", id: heading.id });
    setSummaryOpen(true);
  };
  const closeSummary = () => {
    setSummaryOpen(false);
  };
  const openChanges = () => {
    location.openChanges("changes");
  };

  return (
    <section className="workspace-tabs">
      <SessionBar
        key={heading.id}
        node={node}
        label={label}
        session={heading}
        renamed={title?.id === heading.id ? title.name : null}
        file={state.shownFile}
        onPlace={openChanges}
        workspaceActions={
          state.protocol !== null &&
          state.protocol >= SCRIPT_PROTOCOL &&
          root !== null &&
          scriptProject !== undefined ? (
            <ScriptsControl
              key={`${scope}:${scriptSession ?? "project"}`}
              node={node}
              project={scriptProject}
              session={scriptSession}
              compact={compact}
              terminals={state.terminals}
              onTerminal={(id) => {
                select({ kind: "terminal", id });
              }}
            />
          ) : null
        }
        {...(narrowSummary && heading.spec.mode === "structured"
          ? { onSummary: openSummary }
          : {})}
      />
      <TabChrome
        items={items.filter(
          (item) => tabKey(item.tab) !== location.supporting?.key,
        )}
        active={layout.active}
        words={words}
        launch={launch}
        profiles={state.profiles}
        actions={lifecycle.actions}
        select={select}
        close={lifecycle.askClose}
        closeMany={lifecycle.bulk}
        move={state.move}
      />
      {active === undefined ? (
        <Launcher actions={launch} profiles={state.profiles} words={words} />
      ) : null}
      <WorkspaceViews
        node={node}
        anchor={anchor}
        root={root}
        projects={projects}
        scope={scope}
        state={state}
        words={words}
        items={items}
        renderAgent={agent}
        agentActions={{ summaryOpen, closeSummary, onTitle }}
      />
      {lifecycle.confirmation === null ? null : (
        <Dialog
          title={lifecycle.confirmation.title}
          sheet="content"
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
          sheet="content"
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
