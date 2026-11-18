// Whether a media query matches now, followed as the window changes.

import { useCallback, useSyncExternalStore } from "react";

/** Whether `query` matches, drawn again when that changes. The
 * subscription is one per query (a new function each drawing would have
 * React unsubscribe and subscribe again every time). */
export function useMedia(query: string): boolean {
  const subscribe = useCallback(
    (change: () => void) => {
      const media = window.matchMedia(query);
      media.addEventListener("change", change);
      return () => {
        media.removeEventListener("change", change);
      };
    },
    [query],
  );
  return useSyncExternalStore(
    subscribe,
    () => window.matchMedia(query).matches,
  );
}
