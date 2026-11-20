import { useNavigate } from "@tanstack/react-router";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { useFleetState } from "../../context";
import {
  projectAt,
  type ProjectsView,
} from "../../data/workspace/project-store";
import { rootKey, sessionRoot } from "../../data/workspace/root";
import {
  moveTab,
  removeTab,
  showTab,
  tabKey,
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
import type { TabAction } from "./chrome";
import { childAgent, useTabLifecycle } from "./lifecycle";
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

export function useWorkspace(
  node: NodeKey,
  anchor: SessionInfo,
  root: WorkspaceRoot | null,
  projects: ProjectsView,
  scope: string,
  words: TabWords,
) {
  const navigate = useNavigate();
  const protocol = useFleetState((state) => {
    const status = state.statuses[node];
    return status?.state === "connected" ? status.protocol : null;
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
  const place = useMemo<TerminalPlace>(
    () =>
      root?.kind === "project"
        ? { kind: "project", project: root.project }
        : { kind: "session", session: anchor.id },
    [anchor.id, root],
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
  const { layout, update } = useLayout(scope, anchor.id, discovered);
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

  useEffect(() => {
    if (dirty.size === 0) {
      return undefined;
    }
    const warn = (event: BeforeUnloadEvent) => {
      event.preventDefault();
    };
    window.addEventListener("beforeunload", warn);
    return () => window.removeEventListener("beforeunload", warn);
  }, [dirty.size]);

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
  const actions: TabAction[] = [
    {
      label: words.agent,
      icon: "compose",
      run: () => select({ kind: "draft" }),
    },
    {
      label: words.terminal,
      icon: "terminal",
      disabled: !terminalEnabled || terminals.creating,
      run: () => void launchTerminal(),
    },
    {
      label: words.changes,
      icon: "git-compare",
      run: () => select({ kind: "changes" }),
    },
    {
      label: words.diff,
      icon: "git-compare",
      run: () => select({ kind: "diff" }),
    },
    {
      label: words.files,
      icon: "folder",
      run: () => select({ kind: "files" }),
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
    remove,
    markDirty,
    lifecycle,
    actions,
    terminals: terminals.rows,
    openFile,
    move: (source: string, target: string, side: "before" | "after") =>
      update((current) => moveTab(current, source, target, side)),
  };
}

function isLiveTerminal(terminal: TerminalInfo): boolean {
  return (
    terminal.status.state === "starting" || terminal.status.state === "running"
  );
}
