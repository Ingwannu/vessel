// List only when the workspace is usable. An in-flight read cannot
// resurrect a shell that just closed or outrun the create response.

import { useEffect, useRef, useState } from "react";

import type { Outcome } from "../../data/outcome";
import { useFleet } from "../../context";
import {
  openWorkspaceTerminal,
  workspaceTerminals,
} from "../../data/workspace/terminals";
import type { NodeKey } from "../../wire/bridge";
import type {
  SessionId,
  TerminalId,
  TerminalInfo,
  TerminalPlace,
} from "../../wire/proto";

const LIST_INTERVAL_MS = 2_000;

interface WorkspaceTerminals {
  rows: readonly TerminalInfo[];
  creating: boolean;
  create: () => Promise<Outcome<TerminalInfo> | null>;
  renamed: (terminal: TerminalInfo) => void;
  forget: (id: TerminalId) => void;
}

export function useTerminals(
  node: NodeKey,
  place: TerminalPlace,
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
      const result = await workspaceTerminals(
        fleet,
        node,
        place.kind === "project" ? place : null,
      );
      if (!live) {
        return;
      }
      if (result.ok && !pending.current && since === revision.current) {
        setRows(
          result.value.terminals.filter(
            (terminal) =>
              terminal.kind.kind === "shell" &&
              !closed.current.has(terminal.id) &&
              (place.kind === "project"
                ? terminal.place.kind === "project" &&
                  (terminal.place.dir ?? null) === (place.dir ?? null)
                : terminal.place.kind === "session" &&
                  sessionIds.has(terminal.place.session)),
          ),
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
      if (!enabled || pending.current) {
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
