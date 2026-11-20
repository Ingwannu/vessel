// List only when the workspace is usable. An in-flight read cannot
// resurrect a shell that just closed or outrun the create response.
//
// From protocol 40 (ADR 0121) a worktree or folder workspace lists its
// place by itself, the node matching it, and on the same tick reads the
// shells beside its sessions: agents still open those (`create_terminal`
// asks for `Session`), and the place's list leaves them out. One tick
// replaces the strip's list, so a new shell shows within it and an ended
// one leaves (`readStrip`).

import { useEffect, useMemo, useRef, useState } from "react";

import type { Outcome } from "../../data/outcome";
import { useFleet } from "../../context";
import {
  readStrip,
  sessionHost,
  speaksWorkspacePlaces,
  workspaceTerminalPlace,
} from "../../data/workspace/terminal-place";
import {
  openWorkspaceTerminal,
  workspaceTerminals,
} from "../../data/workspace/terminals";
import type { NodeKey } from "../../wire/bridge";
import type {
  SessionId,
  SessionInfo,
  TerminalId,
  TerminalInfo,
  TerminalPlace,
  WorkspaceRoot,
} from "../../wire/proto";

const LIST_INTERVAL_MS = 2_000;

/** The place the workspace opens its shells in. ADR 0121 (protocol 40):
 * the workspace's own folder holds them, so no agent is needed to open
 * one and archiving one ends none. Below 40 an ended (not archived)
 * agent still hosts them, as in 817. */
export function useTerminalPlace(
  protocol: number | null,
  root: WorkspaceRoot | null,
  anchor: SessionInfo,
  eligible: readonly SessionInfo[],
): TerminalPlace | null {
  const folder = anchor.spec.cwd ?? null;
  // The host matters only for a session place; leaving it out otherwise
  // keeps the place (and its list) from changing as agents start or stop.
  const host =
    speaksWorkspacePlaces(protocol) && (root !== null || folder !== null)
      ? null
      : sessionHost([anchor, ...eligible]);
  return useMemo(
    () => workspaceTerminalPlace({ protocol, root, folder, host }),
    [protocol, root, folder, host],
  );
}

interface WorkspaceTerminals {
  rows: readonly TerminalInfo[];
  creating: boolean;
  create: () => Promise<Outcome<TerminalInfo> | null>;
  renamed: (terminal: TerminalInfo) => void;
  forget: (id: TerminalId) => void;
}

export function useTerminals(
  node: NodeKey,
  place: TerminalPlace | null,
  sessionIds: ReadonlySet<SessionId>,
  enabled: boolean,
): WorkspaceTerminals {
  const fleet = useFleet();
  const [rows, setRows] = useState<readonly TerminalInfo[]>([]);
  const [creating, setCreating] = useState(false);
  const pending = useRef(false);
  const revision = useRef(0);
  const closed = useRef(new Set<TerminalId>());

  useEffect(() => {
    if (!enabled) {
      return undefined;
    }
    let live = true;
    let timer: number | undefined;
    const read = async () => {
      const since = revision.current;
      // No place (below 40, every agent archived): an empty strip, as before.
      const result =
        place === null
          ? { ok: true as const, value: [] }
          : await readStrip(
              (asked) => workspaceTerminals(fleet, node, asked),
              place,
              sessionIds,
            );
      if (!live) {
        return;
      }
      if (result.ok && !pending.current && since === revision.current) {
        setRows(
          result.value.filter((terminal) => !closed.current.has(terminal.id)),
        );
      }
      timer = window.setTimeout(() => void read(), LIST_INTERVAL_MS);
    };
    void read();
    return () => {
      live = false;
      window.clearTimeout(timer);
    };
  }, [enabled, fleet, node, place, sessionIds]);

  return {
    rows,
    creating,
    create: async () => {
      if (!enabled || place === null || pending.current) {
        return null;
      }
      pending.current = true;
      setCreating(true);
      const result = await openWorkspaceTerminal(fleet, node, place);
      pending.current = false;
      revision.current += 1;
      setCreating(false);
      if (result.ok) {
        setRows((items) => [
          result.value,
          ...items.filter((item) => item.id !== result.value.id),
        ]);
      }
      return result;
    },
    renamed: (terminal: TerminalInfo) => {
      revision.current += 1;
      setRows((items) =>
        items.map((item) => (item.id === terminal.id ? terminal : item)),
      );
    },
    forget: (id: TerminalId) => {
      revision.current += 1;
      closed.current.add(id);
      setRows((items) => items.filter((item) => item.id !== id));
    },
  };
}
