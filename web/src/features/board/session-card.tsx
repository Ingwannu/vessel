// One session's card (DESIGN.md §6.12): who started it when it was not
// you, the agent's mark and the session's title, where it works, which
// agent and model run it, then one status line in the lane's colour. The
// whole card opens the session.

import "./session-card.css";

import { Link } from "@tanstack/react-router";

import { agentMark, agentName } from "../../data/agents";
import type { Card } from "../../data/board";
import { sinceLabel } from "../../data/board";
import { useNow } from "../../data/clock";
import { isWorking, needsYou, phraseLabel, tone } from "../../data/phrase";
import type { FleetState } from "../../data/fleet";
import { isLive, shortId } from "../../data/format";
import { labelOf } from "../../data/nodes";
import { sessionName } from "../../data/titles";
import { Avatar, Icon, IconButton, Spinner, Tooltip } from "../../ui/kit";
import { BranchChip } from "../workspace/branch-chip";
import { CardOrigin } from "./card-origin";

export function SessionCard({
  card,
  state,
  scoped,
  onKill,
}: {
  card: Card;
  state: FleetState;
  /** Whether the board shows one machine (the card then omits it). */
  scoped: boolean;
  /** `null` where a card offers no action (the archive). */
  onKill: ((card: Card) => void) | null;
}) {
  const { session, node, placed } = card;
  const now = useNow();
  const spec = session.spec;
  const agents = state.agents[node] ?? [];
  const id = shortId(session.id);
  const title = sessionName(session);
  const phrase = phraseLabel(placed.phrase);
  const machine = labelOf(state, node);
  const model =
    session.model?.model ?? (spec.mode === "structured" ? spec.model : null);
  const attention = needsYou(placed);
  return (
    <li
      className={attention ? "card attention" : "card"}
      data-session={session.id}
    >
      {/* The card's own link sits under its content, so the content's
          links and buttons stay separate targets (not nested in an <a>). */}
      <Link
        className="card-open"
        to="/sessions/$node/$id"
        params={{ node, id: session.id }}
        aria-label={`Open ${id}: ${title}, ${phrase}, on ${machine}`}
      />
      <div className="card-head">
        <CardOrigin card={card} state={state} />
        <div className="card-title">
          <Avatar
            letter={agentMark(spec.harness, agents)}
            label={agentName(spec.harness, agents)}
          />
          <span className="title">{title}</span>
          {onKill !== null && isLive(session) ? (
            <span className="card-action">
              <Tooltip label="Kill">
                <IconButton
                  label={`Kill ${id}`}
                  icon="kill"
                  onClick={() => {
                    onKill(card);
                  }}
                />
              </Tooltip>
            </span>
          ) : null}
        </div>
        <div className="card-line">
          <Icon
            name={spec.mode === "terminal" ? "terminal" : "message"}
            size={14}
          />
          {/* Cut from the left: the end of a path is what tells two apart. */}
          <span className="path">
            <bdi>{spec.cwd ?? "~"}</bdi>
          </span>
        </div>
        {session.workspace?.branch === undefined ||
        session.workspace.branch === null ? null : (
          <div className="card-line">
            <BranchChip branch={session.workspace.branch} />
          </div>
        )}
        <div className="card-line">
          <span className="id">{id}</span>
          <span className="who">
            {session.profile ?? agentName(spec.harness, agents)}
            {model === null || model === undefined ? "" : ` · ${model}`}
          </span>
        </div>
      </div>
      <div className="card-foot">
        <span className={`card-status tone-${tone(placed)}`}>
          {isWorking(placed) ? <Spinner size={12} /> : null}
          <span className="phrase">{phrase}</span>
        </span>
        <span className="card-meta">
          {scoped ? null : `${machine} · `}
          {sinceLabel(placed.at, now)}
        </span>
      </div>
      {session.status.state === "failed" ? (
        <div className="card-error">{session.status.message}</div>
      ) : null}
    </li>
  );
}
