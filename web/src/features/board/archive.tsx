// The ended sessions (DESIGN.md §6.12): not a lane, a dialog the archive
// bar opens, holding the same cards newest first, with a search.

import "./archive.css";

import { useState } from "react";

import { searchArchive } from "../../data/archive";
import type { Card } from "../../data/board";
import type { FleetState } from "../../data/fleet";
import { Button, Dialog, TextField } from "../../ui/kit";
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
        aria-label="Search the archive"
        placeholder="Search by title, folder, id or machine"
        value={query}
        onChange={(next) => {
          setQuery(next);
          setShown(ARCHIVE_PAGE);
        }}
      />
      {found.length === 0 ? (
        <p className="hint">No ended session matches.</p>
      ) : (
        <ul className="cards archive-grid" aria-label="Ended sessions">
          {found.slice(0, shown).map((card) => (
            <SessionCard
              key={card.session.id}
              card={card}
              state={state}
              scoped={scoped}
              onKill={null}
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
