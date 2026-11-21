// A workspace's part in terminal notices (ADR 0114 "Terminal notices",
// claims row 3, page part):
// - a notification's click asked for one of its terminals: show that tab
//   (`data/terminal-tab-request.ts`);
// - its active terminal tab is what presence reports as `terminal` focus
//   (`data/shown-terminal.ts`);
// - while the page is visible and that terminal asks for the person, clear
//   its attention at the exact revision its list shows, once per revision,
//   on a machine that admits claims (ADR 0089 `terminal.attention_clear`).
//   Paseo's node clears a focused terminal's attention on a visible
//   heartbeat (`session.ts:5064-5066`); Vessel's clear is the client's, so
//   a stale view never clears newer attention.

import { useEffect, useRef } from "react";

import { useFleet, useFleetState } from "../../context";
import { claimsOn } from "../../data/claims";
import { attempt } from "../../data/outcome";
import { setShownTerminal } from "../../data/shown-terminal";
import { toClear } from "../../data/terminal-notices";
import { takeTerminalTab } from "../../data/terminal-tab-request";
import type { WorkspaceTab } from "../../data/workspace/tab-layout";
import type { NodeKey } from "../../wire/bridge";
import type { TerminalInfo } from "../../wire/proto";

export function useTerminalAttention(
  node: NodeKey,
  active: WorkspaceTab | undefined,
  rows: readonly TerminalInfo[],
  select: (tab: WorkspaceTab) => void,
): void {
  const fleet = useFleet();
  const claims = useFleetState((state) => claimsOn(state.statuses[node]));
  const terminal = active?.kind === "terminal" ? active.id : null;

  useEffect(() => {
    const asked = takeTerminalTab(
      node,
      rows.map((row) => row.id),
    );
    if (asked !== null) {
      select({ kind: "terminal", id: asked });
    }
  });

  useEffect(() => {
    setShownTerminal(terminal === null ? null : { node, terminal });
    return () => {
      setShownTerminal(null);
    };
  }, [node, terminal]);

  const cleared = useRef<string | null>(null);
  const row = rows.find((each) => each.id === terminal);
  const clear = claims && row !== undefined ? toClear(row) : null;
  const clearTerminal = clear?.terminal ?? null;
  const clearRevision = clear?.revision ?? null;
  useEffect(() => {
    if (clearTerminal === null || clearRevision === null) {
      return;
    }
    const key = `${clearTerminal}:${String(clearRevision)}`;
    // Shown now, or once the page becomes visible again.
    const send = () => {
      if (document.visibilityState !== "visible" || cleared.current === key) {
        return;
      }
      cleared.current = key;
      void attempt(() =>
        fleet.bridge.call(node, "terminal.attention_clear", {
          terminal: clearTerminal,
          revision: clearRevision,
        }),
      );
    };
    send();
    document.addEventListener("visibilitychange", send);
    return () => {
      document.removeEventListener("visibilitychange", send);
    };
  }, [fleet, node, clearTerminal, clearRevision]);
}
