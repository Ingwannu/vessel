// The page's keyboard shortcuts that are not a control's own (DESIGN.md
// §7.1): `⌘K` / `/` for the palette (palette.tsx) and `g` then a letter
// to go to a place. They stand down while the operator types, so a `g` or
// a `/` in a field or the terminal is text.

import { useNavigate } from "@tanstack/react-router";
import { useEffect } from "react";

/** Whether `target` takes typed text. */
export function typing(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement ||
    (target instanceof HTMLElement &&
      (target.isContentEditable || target.closest(".xterm") !== null))
  );
}

/** Where `g` then `key` goes. */
const PLACES: Readonly<Record<string, string>> = {
  f: "/fleet",
  s: "/sessions",
  y: "/sync",
  d: "/devices",
};

/** `g f`, `g s`, `g y`, `g d`. */
export function useGoShortcuts(): void {
  const navigate = useNavigate();
  useEffect(() => {
    let leader = false;
    const onKey = (event: KeyboardEvent) => {
      if (typing(event.target)) {
        return;
      }
      if (leader) {
        leader = false;
        const to = PLACES[event.key];
        if (to !== undefined) {
          void navigate({ to });
        }
        return;
      }
      leader = event.key === "g";
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
    };
  }, [navigate]);
}
