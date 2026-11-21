import { useBlocker, useNavigate } from "@tanstack/react-router";
import { useCallback, useEffect, useMemo, useState } from "react";

import { useFleetState } from "../../context";
import { type LinkedFile, linkedFileOf } from "../../data/forge-links";
import {
  projectAt,
  type ProjectsView,
} from "../../data/workspace/project-store";
import { rootKey, sessionRoot } from "../../data/workspace/root";
import {
  moveTab,
  removeTab,
  showTab,
  settleDraft,
  tabKey,
  type TabLayout,
  type WorkspaceTab,
} from "../../data/workspace/tab-layout";
import { pullPaneServed } from "../../data/workspace/pull/served";
import { TERMINAL_PROTOCOL } from "../../data/workspace/terminals";
import { toast } from "../../ui/toasts";
import type { NodeKey } from "../../wire/bridge";
import type {
  SessionId,
  SessionInfo,
  TerminalInfo,
  WorkspaceRoot,
} from "../../wire/proto";
import type { TabAction } from "./chrome-parts";
import { launchActions } from "./launch-actions";
import { childAgent, isLiveTerminal } from "./close-tabs";
import { useTabLifecycle } from "./lifecycle";
import { useLaunchShortcuts } from "./launch-shortcuts";
import { useLayout } from "./use-layout";
import type { ProfileGroup } from "./profile-rows";
import { useOpenLocation } from "./use-open-location";
import { useTerminalPlace, useTerminals } from "./use-terminals";
import { useTerminalProfiles } from "./use-terminal-profiles";
import type { TabWords } from "./words";

const NO_SESSIONS: readonly SessionInfo[] = [];

function inRoot(
  session: SessionInfo,
  root: WorkspaceRoot,
  projects: ProjectsView,
): boolean {
  const placed = sessionRoot(session);
  return placed === null
    ? root.kind === "project" &&
        projectAt(projects, session.spec.cwd)?.id === root.project
    : rootKey(placed) === rootKey(root);
}

interface WorkspaceState {
  layout: TabLayout;
  active: WorkspaceTab | undefined;
  protocol: number | null;
  visited: ReadonlySet<string>;
  dirty: ReadonlySet<string>;
  location: ReturnType<typeof useOpenLocation>;
  session: (id: SessionId) => SessionInfo | undefined;
  select: (tab: WorkspaceTab) => void;
  completeDraft: (id: SessionId) => void;
  leave: ReturnType<typeof useLeaveWorkspace>;
  markDirty: (key: string, dirty: boolean) => void;
  lifecycle: ReturnType<typeof useTabLifecycle>;
  actions: readonly TabAction[];
  /** The machine's terminal profiles for New tab (ADR 0117). */
  profiles: ProfileGroup;
  terminals: readonly TerminalInfo[];
  move: (source: string, target: string, side: "before" | "after") => void;
  /** The session the bar names: the active agent tab's, else the anchor. */
  heading: SessionInfo;
  /** The file a forge link names (Paseo: the active tab's file, at the
   * line it was opened at; TRACKER 467 step B). */
  shownFile: LinkedFile | null;
}

export function useWorkspace(
  node: NodeKey,
  anchor: SessionInfo,
  root: WorkspaceRoot | null,
  projects: ProjectsView,
  scope: string,
  words: TabWords,
): WorkspaceState {
  const navigate = useNavigate();
  const protocol = useFleetState((state) => {
    const status = state.statuses[node];
    return state.link.kind === "open" && status?.state === "connected"
      ? status.protocol
      : null;
  });
  const listed = useFleetState((state) => state.sessions[node] ?? NO_SESSIONS);
  const eligible = useMemo(
    () =>
      listed.filter((session) =>
        root === null
          ? session.id === anchor.id
          : inRoot(session, root, projects),
      ),
    [anchor.id, listed, projects, root],
  );
  const place = useTerminalPlace(protocol, root, anchor, eligible);
  const sessionIds = useMemo(
    () => new Set([anchor.id, ...eligible.map((session) => session.id)]),
    [anchor.id, eligible],
  );
  const terminalEnabled = protocol !== null && protocol >= TERMINAL_PROTOCOL;
  const terminals = useTerminals(node, place, sessionIds, terminalEnabled);
  const discovered: WorkspaceTab[] = [
    ...eligible
      .filter((agent) => !childAgent(agent) && agent.marks?.archived_at == null)
      .map((agent) => ({ kind: "agent" as const, id: agent.id })),
    ...terminals.rows
      .filter(isLiveTerminal)
      .map((terminal) => ({ kind: "terminal" as const, id: terminal.id })),
  ];
  const ended = terminals.rows
    // A short script can exit before its tab opens. Keep the selected
    // script's output available from the node's retained terminal.
    .filter(
      (terminal) =>
        terminal.kind.kind !== "script" &&
        terminal.kind.kind !== "service" &&
        !isLiveTerminal(terminal),
    )
    .map((terminal) => ({ kind: "terminal" as const, id: terminal.id }));
  const { layout, update } = useLayout(
    scope,
    node,
    anchor.id,
    discovered,
    ended,
  );
  const active = layout.tabs.find((tab) => tabKey(tab) === layout.active);
  const routed = active?.kind === "agent" ? active.id : null;
  const [dirty, setDirty] = useState<ReadonlySet<string>>(() => new Set());
  const [visited, setVisited] = useState<ReadonlySet<string>>(() => new Set());
  const select = (tab: WorkspaceTab) => {
    update((current) => showTab(current, tab));
  };
  const location = useOpenLocation(layout, update, select);
  const visible = [layout.active, location.supporting?.key].filter(
    (key): key is string => typeof key === "string" && !visited.has(key),
  );
  if (visible.length > 0) {
    setVisited(new Set([...visited, ...visible]));
  }

  useEffect(() => {
    if (routed !== null && routed !== anchor.id) {
      void navigate({
        to: "/sessions/$node/$id",
        params: { node, id: routed },
      });
    }
  }, [anchor.id, navigate, node, routed]);

  const leave = useLeaveWorkspace(node, sessionIds, dirty.size > 0);

  const session = (id: SessionId) =>
    listed.find((item) => item.id === id) ??
    (anchor.id === id ? anchor : undefined);
  const markDirty = useCallback((key: string, value: boolean) => {
    setDirty((current) => {
      if (current.has(key) === value) {
        return current;
      }
      const next = new Set(current);
      if (value) {
        next.add(key);
      } else {
        next.delete(key);
      }
      return next;
    });
  }, []);
  const remove = (tab: WorkspaceTab) => {
    const key = tabKey(tab);
    markDirty(key, false);
    location.forget(key);
    update((current) => removeTab(current, key));
    setVisited(
      (current) => new Set([...current].filter((item) => item !== key)),
    );
  };
  const lifecycle = useTabLifecycle({
    node,
    protocol,
    words,
    session,
    dirty: (tab) => dirty.has(tabKey(tab)),
    remove,
    forgetTerminal: terminals.forget,
    renamedTerminal: terminals.renamed,
  });
  // A shell, or a terminal profile's program (ADR 0117).
  const launchTerminal = async (profile?: string) => {
    const result = await terminals.create(profile);
    if (result?.ok === true) {
      select({ kind: "terminal", id: result.value.id });
    } else if (result !== null) {
      toast(result.problem.message, "error");
    }
  };
  const newAgent = () => {
    select({ kind: "draft" });
  };
  const newTerminal = () => void launchTerminal();
  const canCreateTerminal =
    terminalEnabled && !terminals.creating && place !== null;
  const profileRows = useTerminalProfiles(node, protocol, terminalEnabled);
  const profiles: ProfileGroup = {
    machine: node,
    rows: profileRows,
    disabled: !canCreateTerminal,
    launch: (profile) => void launchTerminal(profile.id),
  };
  const shortcuts = useLaunchShortcuts(
    newAgent,
    newTerminal,
    canCreateTerminal,
  );
  const actions = launchActions({
    words,
    select,
    openChanges: location.openChanges,
    openPull: location.openPull,
    newAgent,
    newTerminal,
    canCreateTerminal,
    needsAgent: place === null && terminalEnabled,
    shortcuts,
    pullOffered:
      anchor.pull_request?.summary != null && pullPaneServed(protocol),
  });
  return {
    layout,
    active,
    heading: active?.kind === "agent" ? (session(active.id) ?? anchor) : anchor,
    shownFile: linkedFileOf(active, location.lines),
    protocol,
    visited,
    dirty,
    location,
    session,
    select,
    completeDraft: (id) => {
      markDirty("draft", false);
      update((current) => settleDraft(current, id));
    },
    leave,
    markDirty,
    lifecycle,
    actions,
    profiles,
    terminals: terminals.rows,
    move: (source: string, target: string, side: "before" | "after") => {
      update((current) => moveTab(current, source, target, side));
    },
  };
}

function useLeaveWorkspace(
  node: NodeKey,
  sessions: ReadonlySet<SessionId>,
  dirty: boolean,
) {
  const paths = new Set([...sessions].map((id) => `/sessions/${node}/${id}`));
  return useBlocker({
    shouldBlockFn: ({ next }) => dirty && !paths.has(next.pathname),
    enableBeforeUnload: dirty,
    withResolver: true,
  });
}
