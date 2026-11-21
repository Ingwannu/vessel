// Following one machine's plugins while Settings › Plugins is open (ADR
// 0115), and the order of a change: the parts of use-plugins.ts that decide
// when to read, apart from React, so they are tested on their own. The
// plugin catalog's sync (catalog-sync.tsx) follows each machine the same
// way.
//
// [Decision Log]
// - Purpose: Paseo subscribes to `plugin_catalog_changed` again whenever its
//   host connects (plugins-page.tsx 314-329). The page's watch has to
//   outlive a dropped link, a busy refusal and a node error the same way
//   (node review R1, 2026-11-20).
// - Constraint: when the link drops, the bridge rejects the call in flight
//   and leaves the machine's status alone, and the reconnect snapshot still
//   says connected. Nothing the page watches changes except the link itself.
// - Alternatives:
//   - End the loop on a failed poll and rely on the status changing. This
//     was the first version, and the status never changes on a socket drop.
//   - Re-list on a timer. That costs more calls and shows changes late.
// - Choice:
//   - The hook restarts `followPlugins` when the link opens again, because
//     its effect depends on the link being open.
//   - Inside, a failed poll waits `RETRY_MS` and starts over: a fresh
//     baseline, then a fresh read.
//   - An answer that brought nothing new waits out pace.ts's `QUIET_MS`, so
//     a node that answers at once (stopping, or a forwarded call) is not
//     asked back to back (note 3).
//   - The baseline is taken before the list is read, so a change between
//     the two still shows (note 5).
// - A change ends in Paseo's order (its mutation's onSuccess awaits the
//   refresh): say what it did, read the list again, and only then let the
//   controls go (R3). The old list is never clickable after a change.
// - A read that fails starts over too (`reload` answers false): the
//   catalog's sync has no Retry button, and a list it never read would
//   otherwise stay until the next change (app review note 1).
// - Cost: one long poll per open page, and after a failure, one retry every
//   5 s until the page closes or the link drops.

import type { Outcome } from "./outcome";
import { halted, pause, quiet } from "./workspace/pace";

/** The wait after a failed poll before starting over. */
export const RETRY_MS = 5000;

/** A generation the node never has: its first answer is immediate. */
const ANY = Number.MAX_SAFE_INTEGER;

/** What following needs. */
export interface Follow {
  /** Read again; false when the read failed, and following starts over
   * after `RETRY_MS` as after a failed poll. */
  reload: () => Promise<boolean>;
  /** `plugin.watch` after `after`. */
  poll: (after: number) => Promise<Outcome<number>>;
  /** Waits; pace.ts's by default (tests pass their own). */
  quietly?: (asked: number, signal: AbortSignal) => Promise<void>;
  retry?: (signal: AbortSignal) => Promise<void>;
  now?: () => number;
}

/** Follow the machine's plugins until `signal` stops it. */
export async function followPlugins(
  follow: Follow,
  signal: AbortSignal,
): Promise<void> {
  const quietly = follow.quietly ?? quiet;
  const retry = follow.retry ?? ((stop: AbortSignal) => pause(RETRY_MS, stop));
  const now = follow.now ?? (() => performance.now());
  while (!halted(signal)) {
    const base = await follow.poll(ANY);
    if (halted(signal)) return;
    const read = await follow.reload();
    if (halted(signal)) return;
    if (base.ok && read) {
      let seen = base.value;
      for (;;) {
        const asked = now();
        const next = await follow.poll(seen);
        if (halted(signal)) return;
        if (!next.ok) break;
        if (next.value === seen) {
          await quietly(asked, signal);
        } else {
          seen = next.value;
          if (!(await follow.reload())) break;
        }
        if (halted(signal)) return;
      }
    }
    await retry(signal);
  }
}

/** The end of a change: say what it did, read again, then release. */
export async function settleChange(steps: {
  say: () => void;
  reload: () => Promise<void>;
  release: () => void;
}): Promise<void> {
  steps.say();
  try {
    await steps.reload();
  } finally {
    steps.release();
  }
}
