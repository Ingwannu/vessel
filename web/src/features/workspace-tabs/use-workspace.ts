import { useBlocker, useNavigate } from "@tanstack/react-router";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { isLive } from "../../data/format";
import { useFleetState } from "../../context";
import { isArchived } from "../../data/marks";
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
import { TERMINAL_PROTOCOL } from "../../data/workspace/terminals";
import { toast } from "../../ui/toasts";
import type { NodeKey } from "../../wire/bridge";
import type {
  RelPath,
  SessionId,
  SessionInfo,
  TerminalInfo,
  TerminalPlace,
  WorkspaceRoot,
} from "../../wire/proto";
import type { TabAction } from "./chrome-parts";
import { childAgent, isLiveTerminal } from "./close-tabs";
import { useTabLifecycle } from "./lifecycle";
import { useLaunchShortcuts } from "./launch-shortcuts";
import { useLayout } from "./use-layout";
import { useTerminals } from "./use-terminals";
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
  lines: ReadonlyMap<RelPath, { line: number; seq: number }>;
  session: (id: SessionId) => SessionInfo | undefined;
  select: (tab: WorkspaceTab) => void;
  completeDraft: (id: SessionId) => void;
  leave: ReturnType<typeof useLeaveWorkspace>;
  markDirty: (key: string, dirty: boolean) => void;
  lifecycle: ReturnType<typeof useTabLifecycle>;
  actions: readonly TabAction[];
  terminals: readonly TerminalInfo[];
  openFile: (path: RelPath, line: number | null) => void;
  move: (source: string, target: string, side: "before" | "after") => void;
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
  // Ended agents can still host a shell for checking their work. Archive
  // alone ends those terminals and prevents the session from hosting more.
  const candidates = [anchor, ...eligible].filter(
    (agent) => !isArchived(agent),
  );
  const terminalAnchor = (candidates.find(isLive) ?? candidates[0])?.id;
  const place = useMemo<TerminalPlace | null>(
    () =>
      root?.kind === "project"
        ? { kind: "project", project: root.project }
        : terminalAnchor === undefined
          ? null
          : { kind: "session", session: terminalAnchor },
    [root, terminalAnchor],
  );
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
    .filter((terminal) => !isLiveTerminal(terminal))
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
  const [lines, setLines] = useState<
    ReadonlyMap<RelPath, { line: number; seq: number }>
  >(() => new Map());
  const nextLine = useRef(0);
  if (layout.active !== null && !visited.has(layout.active)) {
    setVisited(new Set([...visited, layout.active]));
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

  const select = (tab: WorkspaceTab) => {
    update((current) => showTab(current, tab));
  };
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
  const launchTerminal = async () => {
    const result = await terminals.create();
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
  const shortcuts = useLaunchShortcuts(
    newAgent,
    newTerminal,
    canCreateTerminal,
  );
  const actions: TabAction[] = [
    {
      label: words.agent,
      icon: "compose",
      run: newAgent,
      ...(shortcuts.agent === null ? {} : { shortcut: shortcuts.agent }),
    },
    {
      label: words.terminal,
      icon: "terminal",
      disabled: !canCreateTerminal,
      ...(place === null ? { description: words.terminalNeedsAgent } : {}),
      run: newTerminal,
      ...(shortcuts.terminal === null ? {} : { shortcut: shortcuts.terminal }),
    },
    {
      label: words.changes,
      icon: "git-compare",
      run: () => {
        select({ kind: "changes" });
      },
    },
    {
      label: words.diff,
      icon: "git-compare",
      run: () => {
        select({ kind: "diff" });
      },
    },
    {
      label: words.files,
      icon: "folder",
      run: () => {
        select({ kind: "files" });
      },
    },
  ];
  const openFile = (path: RelPath, line: number | null) => {
    if (line !== null) {
      nextLine.current += 1;
      const seek = { line, seq: nextLine.current };
      setLines((current) => new Map(current).set(path, seek));
    }
    select({ kind: "file", path });
  };
  return {
    layout,
    active,
    protocol,
    visited,
    dirty,
    lines,
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
    terminals: terminals.rows,
    openFile,
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
