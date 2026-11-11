// What the page's first screen is (track W2): the board once a machine
// is connected, Paseo's and GOAT's welcome while none is, and nothing
// while it cannot tell yet.
//
// [Decision Log]
// - Purpose: with no machine connected, the board's empty lanes say
//   nothing about what to do. Paseo's first screen offers to connect a
//   computer and leaves by itself once a host is online; GOAT's says
//   "Waiting for your machines".
// - Constraint: on opening, every machine is "connecting" for a moment.
//   Showing the welcome then would flash it on every visit.
// - Choice: wait while the book is unread or any machine is still being
//   dialled for the first time; the welcome once every machine has
//   settled without connecting (or the book is empty); the board as soon
//   as one connects, whenever that happens. Once shown, the welcome stays
//   while a machine is redialled (each retry passes through "connecting"
//   again), so it does not blink away on every attempt.

import type { NodeKey, NodeStatus } from "../wire/bridge";
import type { FleetState } from "./fleet";
import { HERE } from "./machines";

/** What the page's index shows. */
export type FirstScreen = "wait" | "welcome" | "board";

/** What the index shows for `state`, having shown `before`. */
export function firstScreen(
  state: FleetState,
  before: FirstScreen = "wait",
): FirstScreen {
  if (state.book === null) {
    return state.bookError === null ? before : "welcome";
  }
  const statuses = state.book.map((node) => state.statuses[node.key]);
  if (statuses.some((status) => status?.state === "connected")) {
    return "board";
  }
  const unsettled = (status: NodeStatus | undefined) =>
    status === undefined ||
    status.state === "idle" ||
    status.state === "connecting";
  return statuses.some(unsettled) && before !== "welcome" ? "wait" : "welcome";
}

/** Whether the welcome should say how to start this machine's own node:
 * it is in the book and did not connect. */
export function hereIsDown(
  book: FleetState["book"],
  statuses: Readonly<Record<NodeKey, NodeStatus>>,
): boolean {
  const here = (book ?? []).find((node) => node.key === HERE);
  return here !== undefined && statuses[HERE]?.state !== "connected";
}
