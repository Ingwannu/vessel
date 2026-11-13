// The archived sessions (DESIGN.md §6.12; agent-orchestrator's archive):
// not a lane, a dialog the archive bar opens, holding the same cards
// newest first, with a search; each card restores (starts again, or
// comes back to its lane) or is deleted for good, where its machine keeps
// marks (protocol 10). From an older machine it holds its ended sessions,
// which have no such actions.

import "./archive.css";

import { useState } from "react";

import { searchArchive } from "../../data/archive";
import type { Card } from "../../data/board";
import type { FleetState } from "../../data/fleet";
import { isArchived } from "../../data/marks";
import { labelOf } from "../../data/nodes";
import { Button, Dialog, TextField } from "../../ui/kit";
import { ArchivedCardActions } from "./card-actions";
import { SessionCard } from "./session-card";

/** How many archived cards the archive shows before "Show more": a node
 * keeps every ended session's log, so the archive grows without bound. */
const ARCHIVE_PAGE = 30;

export function Archive({
  cards,
  state,
  scoped,
  onClose,
}: {
  cards: readonly Card[];
  state: FleetState;
  scoped: boolean;
  onClose: () => void;
}) {
  const [query, setQuery] = useState("");
  const [shown, setShown] = useState(ARCHIVE_PAGE);
  const found = searchArchive(cards, state, query);
  const rest = found.length - shown;
  return (
    <Dialog title="Archive" size="lg" onClose={onClose}>
      <TextField
        icon="search"
        role="searchbox"
        aria-label="Search archived sessions"
        placeholder="Search archived sessions"
        value={query}
        onChange={(next) => {
          setQuery(next);
          setShown(ARCHIVE_PAGE);
        }}
      />
      {found.length === 0 ? (
        <p className="hint">
          {cards.length === 0 ? "No archived sessions" : "No results."}
        </p>
      ) : (
        <ul className="cards archive-grid" aria-label="Archived sessions">
          {found.slice(0, shown).map((card) => (
            <SessionCard
              key={card.session.id}
              card={card}
              state={state}
              scoped={scoped}
              onKill={null}
              {...(isArchived(card.session)
                ? {
                    actions: (
                      <ArchivedCardActions
                        card={card}
                        label={labelOf(state, card.node)}
                      />
                    ),
                  }
                : {})}
            />
          ))}
        </ul>
      )}
      {rest > 0 ? (
        <div className="lane-more">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => {
              setShown(shown + ARCHIVE_PAGE);
            }}
          >
            Show {Math.min(rest, ARCHIVE_PAGE)} more
          </Button>
        </div>
      ) : null}
    </Dialog>
  );
}
