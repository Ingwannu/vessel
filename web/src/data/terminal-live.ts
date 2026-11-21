// Which of a followed terminal's records are live, and so may be answered
// (TRACKER 826, ADR 0073): those from the `next_seq` a view read after its
// last gap. Everything before that, and everything while no such read has
// answered, is history.
//
// [Decision Log]
// - Purpose: a query a program wrote while this view could not see it is
//   never answered late (ADR 0073). If the answer went in late, it would
//   land in whatever the program reads next, as typed text in a prompt.
// - Constraint: a follow resumes from its cursor after either of two gaps,
//   and the page's view hears of them in different ways:
//   - The tab's link to `vessel ui` closes and opens again. The bridge
//     client rejoins every stream from `next` (`bridge/client.ts`
//     `#linked`) and tells the view only through `onLink`.
//   - The bridge's own stream to the node reconnects. The view gets the
//     stream states `reconnecting`, then `resumed` or `missing`.
//   The records written during either gap arrive afterwards with seq
//   above any earlier read.
// - Choice: a gap clears the boundary. When the link is open again, or the
//   stream has resumed, the view reads the terminal or session again;
//   records from that read's `next_seq` are live. A read that answers after
//   a later gap began is ignored. Native does the same: it takes
//   `live_from` from every follow, reconnects included
//   (`terminal_tabs/stream.rs`).
// - Rejected: telling records apart by their timestamp. That compares the
//   node's clock with the browser's.
// - Cost: one read per gap. Records arriving before the read answers are
//   history. So is a query asked in the moment after a reconnect, which is
//   the same miss ADR 0073 states for a load.

import type { LinkState } from "../bridge/socket";
import type { StreamState } from "../wire/bridge";
import type { Outcome } from "./outcome";

/** Whether bytes are a follow's history (never answered) or live. */
export type Written = "history" | "live";

/** One view's boundary, for the life of its follow. */
export interface LiveBoundary {
  /** Whether the record `seq` is live. */
  written: (seq: number) => Written;
  /** The follow's stream state, as the view's handler gets it. */
  stream: (state: StreamState) => void;
  /** Stop listening; nothing is live after it. */
  stop: () => void;
}

/** A boundary read now with `read`, and again after every gap. `onLink`
 * is the bridge client's (it returns its unsubscribe). */
export function liveBoundary(input: {
  read: () => Promise<Outcome<{ next_seq: number }>>;
  onLink: (listener: (link: LinkState) => void) => () => void;
}): LiveBoundary {
  let liveFrom: number | null = null;
  let epoch = 0;
  let stopped = false;
  const gap = () => {
    epoch += 1;
    liveFrom = null;
  };
  const refresh = () => {
    gap();
    const asked = epoch;
    void input.read().then((read) => {
      if (!stopped && asked === epoch && read.ok) {
        liveFrom = read.value.next_seq;
      }
    });
  };
  const unlisten = input.onLink((link) => {
    if (link.kind === "open") {
      refresh();
    } else {
      gap();
    }
  });
  refresh();
  return {
    written: (seq) =>
      liveFrom !== null && seq >= liveFrom ? "live" : "history",
    stream: (state) => {
      switch (state.state) {
        case "resumed":
        case "missing":
          refresh();
          return;
        case "reconnecting":
        case "ended":
        case "failed":
          gap();
          return;
      }
    },
    stop: () => {
      stopped = true;
      gap();
      unlisten();
    },
  };
}
