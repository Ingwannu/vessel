// Reporting this window's presence to every connected machine
// (`data/presence.ts`, Paseo's `use-client-activity.ts`). Mounted once, by
// the shell.
//
// Wiring, as Paseo's hook (`use-client-activity.ts`):
// - visibility from `visibilitychange` (`:69-75`);
// - input from window focus, pointer, key, wheel and touch (`:64-67,78-82`);
// - the shown session from the route (`:115-118`);
// - one heartbeat when a machine connects, then every 15 s (`:125-148`).
//
// A machine that refuses the call (older than protocol 22, or a device it
// does not take presence from) is not asked again until it reconnects.

import { useEffect, useRef, useSyncExternalStore } from "react";

import { useFleet } from "../../context";
import {
  createPresenceTracker,
  HEARTBEAT_INTERVAL_MS,
  PRESENCE_PROTOCOL,
  type PresenceTracker,
  refusesPresence,
  reportPresence,
  type ShownSession,
  shownOf,
} from "../../data/presence";
import { claimsOn } from "../../data/claims";
import { entriesOf } from "../../data/records";
import {
  shownTerminal,
  subscribeShownTerminal,
} from "../../data/shown-terminal";
import { shownSession, useWhere } from "../../data/where";
import type { NodeKey } from "../../wire/bridge";

export function usePresence(): void {
  const fleet = useFleet();
  const shown = shownOf(shownSession(useWhere()));
  // The shown session at each render, for the tracker made below.
  const current = useRef<ShownSession | null>(shown);
  useEffect(() => {
    current.current = shown;
  });
  const tracker = useRef<PresenceTracker | null>(null);

  useEffect(() => {
    const refused = new Set<NodeKey>();
    const serving = (): NodeKey[] =>
      entriesOf(fleet.store.get().statuses)
        .filter(
          ([node, status]) =>
            status.state === "connected" &&
            status.protocol >= PRESENCE_PROTOCOL &&
            !refused.has(node),
        )
        .map(([node]) => node);
    const made = createPresenceTracker({
      link: {
        nodes: serving,
        terminals: (node) => claimsOn(fleet.store.get().statuses[node]),
        send(node, report) {
          void reportPresence(fleet, node, report).then((sent) => {
            if (!sent.ok && refusesPresence(sent.problem)) {
              refused.add(node);
            }
          });
        },
      },
      now: () => Date.now(),
      initialVisible: document.visibilityState === "visible",
      initialShown: current.current,
    });
    tracker.current = made;

    let connected = new Set(serving());
    const stop = fleet.store.subscribe(() => {
      const statuses = fleet.store.get().statuses;
      for (const [node, status] of entriesOf(statuses)) {
        if (status.state !== "connected") {
          refused.delete(node);
        }
      }
      const now = new Set(serving());
      for (const node of now) {
        if (!connected.has(node)) {
          made.sendTo(node);
        }
      }
      for (const node of connected) {
        if (!now.has(node)) {
          made.forget(node);
        }
      }
      connected = now;
    });
    const timer = setInterval(() => {
      made.sendHeartbeat();
    }, HEARTBEAT_INTERVAL_MS);
    made.sendHeartbeat();

    const input = () => {
      made.recordUserActivity();
      made.maybeSendImmediateHeartbeat();
    };
    const visibility = () => {
      const visible = document.visibilityState === "visible";
      const { changed } = made.notifyAppVisibility(visible);
      if (changed && visible) {
        made.maybeSendImmediateHeartbeat();
      }
    };
    const passive = { passive: true } as const;
    document.addEventListener("visibilitychange", visibility);
    window.addEventListener("focus", input);
    window.addEventListener("pointerdown", input, passive);
    window.addEventListener("keydown", input);
    window.addEventListener("wheel", input, passive);
    window.addEventListener("touchstart", input, passive);
    return () => {
      stop();
      clearInterval(timer);
      document.removeEventListener("visibilitychange", visibility);
      window.removeEventListener("focus", input);
      window.removeEventListener("pointerdown", input);
      window.removeEventListener("keydown", input);
      window.removeEventListener("wheel", input);
      window.removeEventListener("touchstart", input);
      tracker.current = null;
    };
  }, [fleet]);

  const node = shown?.node;
  const id = shown?.id;
  const terminal = useSyncExternalStore(subscribeShownTerminal, shownTerminal);
  const terminalId =
    terminal !== null && terminal.node === node ? terminal.terminal : undefined;
  useEffect(() => {
    tracker.current?.setShown(
      node === undefined || id === undefined
        ? null
        : terminalId === undefined
          ? { node, id }
          : { node, id, terminal: terminalId },
    );
  }, [node, id, terminalId]);
}
