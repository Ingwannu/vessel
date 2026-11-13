// A card's own actions where its machine keeps marks (protocol 10),
// agent-orchestrator's: an ended card in its lane can be archived; an
// archived card in the archive can be restored (started again where its
// agent can pick up its conversation, brought back to its lane where it
// cannot) or deleted for good. Each asks first where the reference does.

import type { Card } from "../../data/board";
import { canStartAgain } from "../../data/marks";
import { IconButton, Tooltip } from "../../ui/kit";
import { useMarkActions } from "../session/use-mark-actions";

export function EndedCardActions({
  card,
  label,
}: {
  card: Card;
  /** The machine's name. */
  label: string;
}) {
  const marks = useMarkActions(card.node, card.session, label);
  return (
    <span className="card-actions">
      <Tooltip label="Archive session">
        <IconButton
          label="Archive session"
          icon="archive"
          onClick={() => {
            marks.run("archive");
          }}
        />
      </Tooltip>
      {marks.dialog}
    </span>
  );
}

export function ArchivedCardActions({
  card,
  label,
}: {
  card: Card;
  /** The machine's name. */
  label: string;
}) {
  const marks = useMarkActions(card.node, card.session, label);
  const again = canStartAgain(card.session);
  const restore = again ? "Restore session" : "Unarchive";
  return (
    <span className="card-actions">
      <Tooltip
        label={
          again
            ? "Restore session"
            : "Unarchive: this agent cannot pick up where it left off"
        }
      >
        <IconButton
          label={restore}
          icon="restore"
          disabled={marks.busy !== null}
          aria-busy={marks.busy === "start-again" || undefined}
          onClick={() => {
            marks.run(again ? "start-again" : "unarchive");
          }}
        />
      </Tooltip>
      <Tooltip label="Delete permanently">
        <IconButton
          label="Delete permanently"
          icon="trash"
          disabled={marks.busy !== null}
          onClick={() => {
            marks.run("delete");
          }}
        />
      </Tooltip>
      {marks.dialog}
    </span>
  );
}
