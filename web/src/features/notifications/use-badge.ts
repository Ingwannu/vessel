// The app badge and the favicon's state (ADR 0089, "The page's badge and
// favicon"; the rules are `data/badge.ts`), as Paseo's
// `use-favicon-status.ts` keeps both. Each
// change of the machines' sessions sets the count on the app icon where the
// browser offers `navigator.setAppBadge` (an installed page; elsewhere it
// is absent and nothing is set), and points the page's icon link at the
// plain, running or attention favicon.

import { useEffect } from "react";

import { useFleet } from "../../context";
import {
  badgeCount,
  type FaviconStatus,
  faviconHref,
  faviconStatus,
  workspaceBuckets,
} from "../../data/badge";

/** The badge on the app icon: the count, cleared at none. A browser that
 * refuses (no permission, not installed) is left as it is. */
function setBadge(count: number): void {
  if (!("setAppBadge" in navigator)) {
    return;
  }
  const done =
    count > 0 ? navigator.setAppBadge(count) : navigator.clearAppBadge();
  done.catch(() => undefined);
}

/** Point the icon link at `status`'s favicon. */
function setFavicon(status: FaviconStatus): void {
  const link = document.querySelector<HTMLLinkElement>('link[rel="icon"]');
  const href = faviconHref(status);
  if (link !== null && link.getAttribute("href") !== href) {
    link.setAttribute("href", href);
  }
}

/** Keep the badge and the favicon with the machines' sessions. Mounted
 * once, by the shell. */
export function useBadge(): void {
  const fleet = useFleet();
  useEffect(() => {
    let count = -1;
    let status: FaviconStatus | null = null;
    const update = () => {
      const { sessions } = fleet.store.get();
      const nextCount = badgeCount(workspaceBuckets(sessions));
      if (nextCount !== count) {
        count = nextCount;
        setBadge(nextCount);
      }
      const nextStatus = faviconStatus(sessions);
      if (nextStatus !== status) {
        status = nextStatus;
        setFavicon(nextStatus);
      }
    };
    update();
    const stop = fleet.store.subscribe(update);
    return () => {
      stop();
      setFavicon("none");
    };
  }, [fleet]);
}
