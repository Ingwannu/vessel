// The archive's search (DESIGN.md §6.12): by the start of a session's id,
// a piece of its folder, or its node's name, ignoring case.

import type { Card } from "./board";
import type { FleetState } from "./fleet";
import { labelOf } from "./nodes";

/** The cards of `cards` that match `query`; all of them for an empty one. */
export function searchArchive(
  cards: readonly Card[],
  state: FleetState,
  query: string,
): readonly Card[] {
  const text = query.trim().toLowerCase();
  if (text === "") {
    return cards;
  }
  return cards.filter(
    (card) =>
      card.session.id.startsWith(text) ||
      (card.session.spec.cwd ?? "~").toLowerCase().includes(text) ||
      labelOf(state, card.node).toLowerCase().includes(text),
  );
}
