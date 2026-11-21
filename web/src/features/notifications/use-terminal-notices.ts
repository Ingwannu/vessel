// Terminal notices on the page (ADR 0114 "Terminal notices"; the rules are
// `data/terminal-notices.ts`). For each connected machine that admits
// claims, the page follows the whole terminal list on its revision. What a
// list brings is claimed by its exact revision when Settings' alerts would
// tell it; only an admitted claim is shown and sounded. A click opens a
// session holding the terminal and asks its workspace for the terminal's
// tab (`data/terminal-tab-request.ts`). While ADR 0089's protocol is
// unassigned no machine admits claims, so no list is followed.

import { useNavigate } from "@tanstack/react-router";
import { useEffect } from "react";

import { useFleet } from "../../context";
import { playsSound, showsSystem } from "../../data/alerts";
import { claimsOn } from "../../data/claims";
import type { Fleet } from "../../data/fleet";
import { attempt } from "../../data/outcome";
import { entriesOf } from "../../data/records";
import { requestTerminalTab } from "../../data/terminal-tab-request";
import {
  holds,
  readTerminals,
  type TerminalNotice,
  terminalNoticeTitle,
  terminalSource,
  type Watched,
} from "../../data/terminal-notices";
import { prefs } from "../../data/prefs";
import type { NodeKey } from "../../wire/bridge";
import { windowNotifies } from "./notifies";
import { playNotice } from "./sound";

/** How long a follower waits before asking again when the list's revision
 * did not move. */
const QUIET_MS = 1_000;

/** A session on `notice.node` whose workspace holds the terminal's place. */
function holder(fleet: Fleet, notice: TerminalNotice): string | null {
  const sessions = fleet.store.get().sessions[notice.node] ?? [];
  return sessions.find((session) => holds(session, notice.place))?.id ?? null;
}

/** Follow `node`'s terminals until `stopped` or a call fails, handing on
 * each list's notices. */
async function follow(
  fleet: Fleet,
  node: NodeKey,
  stopped: () => boolean,
  found: (notices: TerminalNotice[]) => void,
): Promise<void> {
  let watched: Watched = null;
  let changedFrom: number | null = null;
  while (!stopped()) {
    const listed = await attempt(() =>
      fleet.bridge.call(node, "terminal.list", {
        place: null,
        changed_from: changedFrom,
      }),
    );
    if (!listed.ok || stopped()) {
      return;
    }
    // A node that answers without waiting (its wait slots full, or an
    // older one) would make this ask again at round-trip speed: when the
    // revision did not move, wait a quiet second first (app review note,
    // as the catalog watcher's quiet wait).
    if (listed.value.revision === changedFrom) {
      await new Promise((resolve) => setTimeout(resolve, QUIET_MS));
    }
    changedFrom = listed.value.revision;
    const read = readTerminals(watched, node, listed.value.terminals);
    watched = read.watched;
    if (read.notices.length > 0) {
      found(read.notices);
    }
  }
}

/** Follow every claiming machine's terminals and tell what it admits.
 * Mounted once, by the shell. */
export function useTerminalNotices(): void {
  const fleet = useFleet();
  const navigate = useNavigate();
  useEffect(() => {
    const following = new Map<NodeKey, { stop: () => void }>();
    const tell = (notice: TerminalNotice) => {
      const { alerts } = prefs.get().values.notifications;
      if (
        showsSystem(alerts) &&
        "Notification" in window &&
        Notification.permission === "granted"
      ) {
        const shown = new Notification(terminalNoticeTitle(notice.kind), {
          body: notice.title === "" ? "Terminal" : notice.title,
          tag: `vessel:${notice.node}:terminal:${notice.terminal}:${notice.kind}`,
        });
        shown.onclick = () => {
          window.focus();
          const session = holder(fleet, notice);
          if (session !== null) {
            requestTerminalTab(notice.node, notice.terminal);
            void navigate({
              to: "/sessions/$node/$id",
              params: { node: notice.node, id: session },
            });
          }
          shown.close();
        };
      }
      if (playsSound(alerts)) {
        playNotice(notice.kind);
      }
    };
    const claimed = (notices: TerminalNotice[]) => {
      const { alerts } = prefs.get().values.notifications;
      if (!windowNotifies()) {
        // A window that does not show the system's notifications never
        // claims, so it never takes a notice from one that does; one that
        // sounds plays its own sound (ADR 0114's local path).
        if (playsSound(alerts)) {
          for (const notice of notices) {
            playNotice(notice.kind);
          }
        }
        return;
      }
      for (const notice of notices) {
        void attempt(() =>
          fleet.bridge.call(notice.node, "notification.claim", {
            source: terminalSource(notice),
          }),
        ).then((claim) => {
          if (claim.ok && claim.value === "admitted") {
            tell(notice);
          }
        });
      }
    };
    const sync = () => {
      const statuses = fleet.store.get().statuses;
      for (const [node, status] of entriesOf(statuses)) {
        if (claimsOn(status) && !following.has(node)) {
          let stopped = false;
          const follower = {
            stop: () => {
              stopped = true;
            },
          };
          following.set(node, follower);
          // A follower that ended on a failed call leaves its place, so the
          // next change of the machines starts a new one.
          void follow(fleet, node, () => stopped, claimed).finally(() => {
            if (following.get(node) === follower) {
              following.delete(node);
            }
          });
        }
      }
      for (const [node, follower] of following) {
        if (!claimsOn(statuses[node])) {
          follower.stop();
          following.delete(node);
        }
      }
    };
    sync();
    const unsubscribe = fleet.store.subscribe(sync);
    return () => {
      unsubscribe();
      for (const follower of following.values()) {
        follower.stop();
      }
      following.clear();
    };
  }, [fleet, navigate]);
}
