// Telling the person about sessions they are not looking at (DESIGN.md
// §7.7, ADR 0114): between two polls of a machine's sessions, what
// started to need them or finished (data/notices.ts) becomes the
// browser's notification and a sound, as Settings › Notifications says,
// unless the page is in front showing that session (Paseo's rule,
// `told`). While the page is not in front, the tab's title counts them.
//
// "In front" is GOAT's: the page visible and its window focused. The
// count is of notices since the page was last in front, and clears when
// it is. A system notification carries a tag per session and kind, so
// two tabs of this page show it once; clicking it opens the session.

import { useNavigate } from "@tanstack/react-router";
import { useEffect, useRef } from "react";

import { useFleet } from "../../context";
import { playsSound, showsSystem } from "../../data/alerts";
import { askBeforeTelling, claimCalls, claimsOn } from "../../data/claims";
import {
  type Notice,
  type Seen,
  noticeTitle,
  notices,
  titleWith,
  told,
} from "../../data/notices";
import { prefs } from "../../data/prefs";
import { sessionName } from "../../data/titles";
import { windowNotifies } from "./notifies";
import { playNotice, unlockSound } from "./sound";
import { entriesOf } from "../../data/records";
import { shownSession, useWhere } from "../../data/where";
import type { NodeKey } from "../../wire/bridge";

function away(): boolean {
  return document.visibilityState !== "visible" || !document.hasFocus();
}

/** Watch every machine's sessions and tell what is worth telling.
 * Mounted once, by the shell. */
export function useNotices(): void {
  const fleet = useFleet();
  const navigate = useNavigate();
  const where = useWhere();
  // The session shown, read by the poll's callback at its time; kept
  // after each render (a ref is not written while rendering).
  const shown = useRef<Seen["shown"]>(null);
  useEffect(() => {
    shown.current = shownSession(where);
  });
  useEffect(() => {
    const base = document.title;
    let before = fleet.store.get().sessions;
    let unseen = 0;
    const tell = (notice: Notice) => {
      const { alerts } = prefs.get().values.notifications;
      if (
        showsSystem(alerts) &&
        "Notification" in window &&
        Notification.permission === "granted"
      ) {
        const shown = new Notification(noticeTitle(notice.kind), {
          body: sessionName(notice.session),
          tag: `vessel:${notice.node}:${notice.session.id}:${notice.kind}`,
        });
        shown.onclick = () => {
          window.focus();
          void navigate({
            to: "/sessions/$node/$id",
            params: { node: notice.node, id: notice.session.id },
          });
          shown.close();
        };
      }
      if (playsSound(alerts)) {
        playNotice(notice.kind);
      }
    };
    const stop = fleet.store.subscribe(() => {
      const after = fleet.store.get().sessions;
      if (after === before) {
        return;
      }
      const found = entriesOf(after).flatMap(([node, list]) => {
        const previous = before[node];
        return previous === undefined ? [] : notices(node, previous, list);
      });
      before = after;
      if (found.length === 0) {
        return;
      }
      const front = !away();
      // A machine that admits claims (ADR 0089) is asked first, once per
      // machine; every other machine's notices are told at once (ADR 0114).
      const asking = new Map<NodeKey, Notice[]>();
      for (const notice of told(found, { front, shown: shown.current })) {
        // Only a window that shows the system's notifications claims; a
        // window that only sounds keeps ADR 0114's local path
        // (`notifies.ts`).
        if (
          windowNotifies() &&
          claimsOn(fleet.store.get().statuses[notice.node])
        ) {
          asking.set(notice.node, [...(asking.get(notice.node) ?? []), notice]);
        } else {
          tell(notice);
        }
      }
      for (const [node, notices] of asking) {
        void askBeforeTelling(claimCalls(fleet, node), notices).then(
          (admitted) => {
            admitted.forEach(tell);
          },
        );
      }
      if (!front && prefs.get().values.notifications.title) {
        unseen += found.length;
        document.title = titleWith(base, unseen);
      }
    });
    const back = () => {
      if (!away() && unseen > 0) {
        unseen = 0;
        document.title = base;
      }
    };
    // Sound is made possible from a click or a key, and only while a
    // sound is wanted: an audio context holds the machine's audio output
    // open, and creating one took its time inside the reader's first
    // click (docs/PERFORMANCE.md, "Tracks P and W merged").
    const unlock = () => {
      if (playsSound(prefs.get().values.notifications.alerts)) {
        unlockSound();
      }
    };
    window.addEventListener("focus", back);
    document.addEventListener("visibilitychange", back);
    window.addEventListener("pointerdown", unlock);
    window.addEventListener("keydown", unlock);
    return () => {
      stop();
      window.removeEventListener("focus", back);
      document.removeEventListener("visibilitychange", back);
      window.removeEventListener("pointerdown", unlock);
      window.removeEventListener("keydown", unlock);
      document.title = base;
    };
  }, [fleet, navigate]);
}
