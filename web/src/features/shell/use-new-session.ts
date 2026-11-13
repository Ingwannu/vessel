// Every "New" leads to `/new` (agent-orchestrator's new-task page),
// carrying the machine in view, and from a session its project: the
// title bar's button, the sidebar's row, the board's empty lanes, the
// palette and the `n` key.

import { useNavigate } from "@tanstack/react-router";
import { useCallback, useEffect } from "react";

import { closeNewSession, useChrome, useFleetState } from "../../context";
import { useWhere, type Where } from "../../data/where";
import { wantedPlace } from "../../data/workspace/wanted-place";
import { useShortcut } from "../shortcuts/handlers";

/** Open the new-session page on `machine` (`null`: the page picks). */
export function useNewSession(): (machine: string | null) => void {
  const navigate = useNavigate();
  return useCallback(
    (machine: string | null) => {
      void navigate({
        to: "/new",
        search: machine === null ? {} : { machine },
      });
    },
    [navigate],
  );
}

/** The machine the page shows: a session's, the one a Settings section is
 * about, or the one the board is scoped to. */
export function machineInView(where: Where): string | null {
  const [place, node] = where.segments;
  if (place === "sessions" && node !== undefined) {
    return node;
  }
  if (place === "settings" && where.machine !== undefined) {
    return where.machine;
  }
  return where.node ?? null;
}

/** Turns requests made through the chrome's store (`openNewSession`, the
 * palette's command) into the page, and answers the "new session"
 * shortcut (`N` by default). Mounted once, by the shell. */
export function useNewSessionEntry(): void {
  const open = useNewSession();
  const where = useWhere();
  const request = useChrome((chrome) => chrome.newSession);
  const inView = machineInView(where);
  // From a session, a new one starts in its project (agent-orchestrator's
  // ⌘N "the open session's project"; GOAT's Mod+Shift+N).
  const [place, at, named] = where.segments;
  const node = useFleetState((state) =>
    place === "sessions"
      ? (state.book?.find((entry) => entry.key === at)?.key ?? null)
      : null,
  );
  const project = useFleetState((state) =>
    node === null
      ? null
      : (state.sessions[node]?.find((session) => session.id === named)
          ?.workspace?.project ?? null),
  );
  const openFrom = useCallback(
    (machine: string | null) => {
      if (node !== null && machine === node && project !== null) {
        wantedPlace.set({ node, project });
      }
      open(machine);
    },
    [open, node, project],
  );
  useEffect(() => {
    if (request !== null) {
      closeNewSession();
      openFrom(request.node);
    }
  }, [request, openFrom]);
  useShortcut("new-session", () => {
    openFrom(inView);
    return true;
  });
}
