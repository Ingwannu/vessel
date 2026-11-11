// Telling the person about sessions they are not looking at (DESIGN.md
// §7.7): between two polls of a machine's sessions, what started to need
// them or finished (data/notices.ts) becomes, while the page is not in
// front, the browser's notification, a sound and a count in the tab's
// title, as Settings › Notifications says.
//
// "Not in front" is GOAT's: the page hidden, or its window not focused.
// The count is of notices since the page was last in front, and clears
// when it is. A system notification carries a tag per session and kind,
// so two tabs of this page show it once; clicking it opens the session.

import { useNavigate } from "@tanstack/react-router";
import { useEffect } from "react";

import { useFleet } from "../../context";
import { playsSound, showsSystem } from "../../data/alerts";
import {
  type Notice,
  noticeTitle,
  notices,
  titleWith,
} from "../../data/notices";
import { prefs } from "../../data/prefs";
import { sessionName } from "../../data/titles";
import { playNotice, unlockSound } from "./sound";
import { entriesOf } from "../../data/records";

function away(): boolean {
  return document.visibilityState !== "visible" || !document.hasFocus();
}

/** Watch every machine's sessions and tell what is worth telling.
 * Mounted once, by the shell. */
export function useNotices(): void {
  const fleet = useFleet();
  const navigate = useNavigate();
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
      if (found.length === 0 || !away()) {
        return;
      }
      found.forEach(tell);
      if (prefs.get().values.notifications.title) {
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
    window.addEventListener("focus", back);
    document.addEventListener("visibilitychange", back);
    window.addEventListener("pointerdown", unlockSound);
    window.addEventListener("keydown", unlockSound);
    return () => {
      stop();
      window.removeEventListener("focus", back);
      document.removeEventListener("visibilitychange", back);
      window.removeEventListener("pointerdown", unlockSound);
      window.removeEventListener("keydown", unlockSound);
      document.title = base;
    };
  }, [fleet, navigate]);
}
