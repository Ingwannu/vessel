// Whether a media query matches now, followed as the window changes.

import { useSyncExternalStore } from "react";

/** Below this width the sidebar beside the page is not drawn
 * (sidebar.css): it opens as a sheet from the title bar instead. */
export const PHONE = "(max-width: 767px)";

/** Whether `query` matches, drawn again when that changes. */
export function useMedia(query: string): boolean {
  return useSyncExternalStore(
    (change) => {
      const media = window.matchMedia(query);
      media.addEventListener("change", change);
      return () => {
        media.removeEventListener("change", change);
      };
    },
    () => window.matchMedia(query).matches,
  );
}
