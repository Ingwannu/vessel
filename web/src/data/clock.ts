// The time a card's "6m ago" is written from (board.ts, `sinceLabel`):
// one clock for every card, ticking every 15 s while a card shows it.
//
// [Decision Log] (milestone 11, track P; docs/PERFORMANCE.md)
// - Purpose: the labels used to move only because every session list read
//   (every 2 s, fleet.ts) drew the whole board again, even when nothing
//   in it had changed: with 500 cards that was 15 long tasks every 10 s
//   at rest. Now an unchanged list draws nothing, and the labels need a
//   clock of their own.
// - Choice: a shared external store, started by its first reader and
//   stopped by its last, so nothing ticks while no card is drawn. The
//   labels count minutes, so 15 s is the most a label is late.
// - Alternatives: a timer per card (500 timers for one clock); keeping
//   the full redraw every 2 s (the cost above).

import { useSyncExternalStore } from "react";

const TICK_MS = 15_000;

let now = Date.now();
const readers = new Set<() => void>();
let timer: ReturnType<typeof setInterval> | null = null;

function tick(): void {
  now = Date.now();
  for (const reader of readers) {
    reader();
  }
}

function subscribe(reader: () => void): () => void {
  readers.add(reader);
  if (timer === null) {
    timer = setInterval(tick, TICK_MS);
    // The clock was stopped: read it again for the card subscribing now.
    tick();
  }
  return () => {
    readers.delete(reader);
    if (readers.size === 0 && timer !== null) {
      clearInterval(timer);
      timer = null;
    }
  };
}

/** The time, in milliseconds since the epoch, to at most 15 s. */
export function useNow(): number {
  return useSyncExternalStore(subscribe, () => now);
}
