// Every "New" leads to `/new` (agent-orchestrator's new-task page),
// carrying the machine in view: the title bar's button, the sidebar's
// row, the board's empty lanes, the palette and the `n` key.

import { useNavigate } from "@tanstack/react-router";
import { useCallback, useEffect } from "react";

import { closeNewSession, useChrome } from "../../context";
import { useWhere, type Where } from "../../data/where";
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
  useEffect(() => {
    if (request !== null) {
      closeNewSession();
      open(request.node);
    }
  }, [request, open]);
  useShortcut("new-session", () => {
    open(inView);
    return true;
  });
}
