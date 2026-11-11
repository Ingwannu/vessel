// Who started a session, when it was not this machine's owner: a paired
// device, or another session's agent (linked to that session, and its
// machine named when it runs elsewhere). DESIGN.md §2.4.

import { Link } from "@tanstack/react-router";

import type { Card } from "../../data/board";
import type { FleetState } from "../../data/fleet";
import { shortId } from "../../data/format";
import { labelOf } from "../../data/nodes";

export function CardOrigin({ card, state }: { card: Card; state: FleetState }) {
  const by = card.session.created_by;
  switch (by.kind) {
    case "owner":
      return null;
    case "device":
      return <div className="card-tag">device {by.id.slice(0, 6)}</div>;
    case "agent": {
      // The parent's book key is known only when the parent is listed
      // (`Row.parentNode`); the creator's own `node` is a node id, which
      // the page's routes do not take.
      const parentNode = card.parentNode;
      return (
        <div className="card-tag">
          agent of{" "}
          {parentNode === null ? (
            <span className="mono">{shortId(by.session)}</span>
          ) : (
            <Link
              className="mono"
              to="/sessions/$node/$id"
              params={{ node: parentNode, id: by.session }}
            >
              {shortId(by.session)}
            </Link>
          )}
          {parentNode !== null && parentNode !== card.node
            ? ` on ${labelOf(state, parentNode)}`
            : null}
        </div>
      );
    }
  }
}
