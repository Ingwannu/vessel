// One lane of the board (DESIGN.md §6.12): its heading (a dot and the
// name in the lane's colour, the count) over its own scrolling column of
// cards. An empty Building lane, where new sessions land, offers to start
// one.

import type { Card, Lane as LaneName } from "../../data/board";
import { laneLabel } from "../../data/board";
import type { FleetState } from "../../data/fleet";
import { hasConversation } from "../../data/delivery";
import { isLive } from "../../data/format";
import { labelOf } from "../../data/nodes";
import { Button } from "../../ui/kit";
import { EndedCardActions } from "./card-actions";
import { SessionCard } from "./session-card";

/** Whether `card` is an ended session on a machine that keeps marks: it
 * stays in its lane until archived, and its card offers to archive it. */
function endedWithMarks(card: Card, state: FleetState): boolean {
  const status = state.statuses[card.node];
  return (
    !isLive(card.session) &&
    hasConversation(status?.state === "connected" ? status.protocol : null)
  );
}

export function Lane({
  lane,
  cards,
  state,
  scoped,
  onKill,
  onNew,
}: {
  lane: LaneName;
  cards: readonly Card[];
  state: FleetState;
  scoped: boolean;
  onKill: (card: Card) => void;
  onNew: () => void;
}) {
  const label = laneLabel(lane);
  return (
    <section className="lane" aria-label={label} data-lane={lane}>
      <header className="lane-head">
        <span className="swatch" aria-hidden="true" />
        <h2>{label}</h2>
        <span className="count" aria-label={`${String(cards.length)} sessions`}>
          {cards.length}
        </span>
      </header>
      <div className="lane-body">
        <ul className="cards">
          {cards.map((card) => (
            <SessionCard
              key={card.session.id}
              card={card}
              state={state}
              scoped={scoped}
              onKill={onKill}
              {...(endedWithMarks(card, state)
                ? {
                    actions: (
                      <EndedCardActions
                        card={card}
                        label={labelOf(state, card.node)}
                      />
                    ),
                  }
                : {})}
            />
          ))}
        </ul>
        {cards.length === 0 && lane === "building" ? (
          <div className="lane-new">
            <Button variant="ghost" size="sm" icon="plus" onClick={onNew}>
              New session
            </Button>
          </div>
        ) : null}
      </div>
    </section>
  );
}
