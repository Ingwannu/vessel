// Every "New" leads to `/new` (agent-orchestrator's new-task page),
// carrying the machine in view: the title bar's button, the sidebar's
// row, the board's empty lanes, the palette and the `n` key.

import { useNavigate } from "@tanstack/react-router";
import { useCallback, useEffect } from "react";

import { closeNewSession, useChrome } from "../../context";
import { useWhere, type Where } from "../../data/where";
import { typing } from "../palette/shortcuts";

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

/** The machine the page shows: a session's, or the one the board is
 * scoped to. */
export function machineInView(where: Where): string | null {
  const [place, node] = where.segments;
  if (place === "sessions" && node !== undefined) {
    return node;
  }
  return where.node ?? null;
}

/** Turns requests made through the chrome's store (`openNewSession`, the
 * palette's command) into the page, and listens for `n`. Mounted once,
 * by the shell. */
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
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (
        event.key !== "n" ||
        event.metaKey ||
        event.ctrlKey ||
        event.altKey ||
        typing(event.target)
      ) {
        return;
      }
      event.preventDefault();
      open(inView);
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
    };
  }, [open, inView]);
}
