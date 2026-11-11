// Sessions (DESIGN.md §2.4, §6.12): every session on one node or all, as a
// board. A lane is a status (data/board.ts); a card is a session. A session
// an agent started still sits under the one that started it when both are
// in a lane, and always names it (ADR 0013).

import { Link } from "@tanstack/react-router";
import { useState, type ReactElement } from "react";

import { openNewSession, useCrumbs, useFleet, useFleetState } from "../context";
import {
  board,
  type Card,
  type Lane,
  LANE_PAGE,
  laneLabel,
  LANES,
  laneTone,
} from "../data/board";
import type { FleetState } from "../data/fleet";
import {
  ago,
  harnessName,
  isLive,
  sessionMoment,
  sessionTitle,
  shortId,
  statusTone,
  statusWord,
  when,
} from "../data/format";
import { labelOf, nodeLabel } from "../data/nodes";
import { tree } from "../data/tree";
import { useWhere } from "../data/where";
import { Dialog } from "../ui/dialog";
import { Empty, Icon } from "../ui/parts";
import { toast } from "../ui/toasts";

export function SessionsPage(): ReactElement {
  const where = useWhere();
  const scope = where.node ?? null;
  const fleet = useFleet();
  const state = useFleetState((value) => value);
  const [killing, setKilling] = useState<Card | null>(null);
  const book = state.book ?? [];
  const nodes = scope === null ? book.map((node) => node.key) : [scope];
  const scopeLabel = scope === null ? null : labelOf(state, scope);
  useCrumbs(
    scopeLabel === null
      ? [{ label: "Sessions" }]
      : [{ label: "Sessions", to: "/sessions" }, { label: scopeLabel }],
    scope ?? "",
  );
  const rows = tree(state, nodes);
  const lanes = board(rows);
  const kill = (card: Card) => {
    setKilling(null);
    fleet.bridge
      .call(card.node, "session.kill", { session: card.session.id })
      .then(
        () => {
          toast(`Killed ${shortId(card.session.id)}`);
          void fleet.refreshSessions(card.node);
        },
        (failure: unknown) => {
          toast(
            failure instanceof Error ? failure.message : String(failure),
            "error",
          );
        },
      );
  };
  return (
    <div className="page">
      <div className="page-head board-head">
        <h1>Sessions</h1>
        <div className="seg" role="group" aria-label="Node">
          <Link
            className={scope === null ? "active" : undefined}
            to="/sessions"
            aria-current={scope === null ? "true" : undefined}
          >
            All nodes
          </Link>
          {book.map((node) => (
            <Link
              key={node.key}
              className={scope === node.key ? "active" : undefined}
              to="/sessions"
              search={{ node: node.key }}
              aria-current={scope === node.key ? "true" : undefined}
            >
              {nodeLabel(node, state.statuses[node.key])}
            </Link>
          ))}
        </div>
      </div>
      {rows.length === 0 ? (
        <div className="page-body">
          <Empty
            title={
              scopeLabel === null
                ? "No sessions"
                : `No sessions on ${scopeLabel}`
            }
            action={
              <button
                className="btn primary"
                type="button"
                onClick={() => {
                  openNewSession(scope);
                }}
              >
                New session
              </button>
            }
          >
            Start one to see it here.
          </Empty>
        </div>
      ) : (
        <div className="board" role="group" aria-label="Sessions by status">
          {LANES.map((lane) => (
            <LaneView
              key={lane}
              lane={lane}
              cards={lanes[lane]}
              state={state}
              scoped={scope !== null}
              onKill={setKilling}
            />
          ))}
        </div>
      )}
      {killing === null ? null : (
        <Dialog
          title={`Kill ${shortId(killing.session.id)}?`}
          onClose={() => {
            setKilling(null);
          }}
          footer={
            <>
              <button
                className="btn ghost"
                type="button"
                onClick={() => {
                  setKilling(null);
                }}
              >
                Cancel
              </button>
              <button
                className="btn danger solid"
                type="button"
                onClick={() => {
                  kill(killing);
                }}
              >
                Kill
              </button>
            </>
          }
        >
          <p className="hint">
            The harness on {labelOf(state, killing.node)} and everything it
            started stop now. The session&apos;s log stays.
          </p>
        </Dialog>
      )}
    </div>
  );
}

/** One lane: its heading (state dot, name, count) and its cards. */
function LaneView({
  lane,
  cards,
  state,
  scoped,
  onKill,
}: {
  lane: Lane;
  cards: readonly Card[];
  state: FleetState;
  scoped: boolean;
  onKill: (card: Card) => void;
}) {
  // How many cards are shown: a page at a time, since ended lanes hold
  // every session the node still has a log for.
  const [shown, setShown] = useState(LANE_PAGE);
  const label = laneLabel(lane);
  const tone = laneTone(lane);
  const rest = cards.length - shown;
  return (
    <section className="lane" aria-label={label} data-lane={lane}>
      <header className="lane-head">
        <span
          className={tone === "running" ? "status-dot" : `status-dot ${tone}`}
          aria-hidden="true"
        />
        <h2>{label}</h2>
        <span className="count" aria-label={`${cards.length} sessions`}>
          {cards.length}
        </span>
      </header>
      <div className="lane-body">
        <ul className="cards">
          {cards.slice(0, shown).map((card) => (
            <SessionCard
              key={card.session.id}
              card={card}
              state={state}
              scoped={scoped}
              onKill={onKill}
            />
          ))}
        </ul>
        {rest > 0 ? (
          <button
            className="btn ghost sm lane-more"
            type="button"
            onClick={() => {
              setShown(shown + LANE_PAGE);
            }}
          >
            Show {Math.min(rest, LANE_PAGE)} more
          </button>
        ) : null}
      </div>
    </section>
  );
}

/** Who started a session, when it was not this machine's owner token:
 * a paired device, or another session's agent (linked). */
function CardOrigin({ card, state }: { card: Card; state: FleetState }) {
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
          <Icon name="move" size="sm" />
          <span>
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
          </span>
        </div>
      );
    }
  }
}

/** One session's card (DESIGN.md §6.12): what it works in, who runs it,
 * then one status line. The whole card opens the session. */
function SessionCard({
  card,
  state,
  scoped,
  onKill,
}: {
  card: Card;
  state: FleetState;
  scoped: boolean;
  onKill: (card: Card) => void;
}) {
  const { session, node } = card;
  const spec = session.spec;
  const id = shortId(session.id);
  const title = sessionTitle(spec.cwd);
  const status = statusWord(session.status);
  const tone = statusTone(session.status);
  const model =
    session.model?.model ?? (spec.mode === "structured" ? spec.model : null);
  const moment = sessionMoment(session);
  return (
    <li
      className={card.indent === 0 ? "card" : `card in${card.indent}`}
      data-session={session.id}
    >
      {/* The card's own link sits under its content, so the content's
          links and buttons stay separate targets (not nested in an <a>). */}
      <Link
        className="card-open"
        to="/sessions/$node/$id"
        params={{ node, id: session.id }}
        aria-label={`Open ${id}: ${title}, ${status}, on ${labelOf(state, node)}`}
        title={`${spec.cwd ?? "~"} · ${when(moment)}`}
      />
      <CardOrigin card={card} state={state} />
      <div className="card-title">
        <Icon
          name={spec.mode === "terminal" ? "terminal" : "message"}
          size="sm"
        />
        <span className="title">{title}</span>
        {isLive(session) ? (
          <button
            className="btn ghost sm icon-btn card-action"
            type="button"
            aria-label={`Kill ${id}`}
            title="Kill"
            onClick={() => {
              onKill(card);
            }}
          >
            <Icon name="kill" size="sm" />
          </button>
        ) : null}
      </div>
      <div className="card-where">
        <span className="id">{id}</span>
        {/* Cut from the left: the end of a path is what tells two apart. */}
        <span className="path">
          <bdi>{spec.cwd ?? "~"}</bdi>
        </span>
      </div>
      <div className="card-agent">
        <span>{session.profile ?? harnessName(spec.harness)}</span>
        <span className="sep" aria-hidden="true">
          ·
        </span>
        <span>{spec.mode}</span>
        {model === null || model === undefined ? null : (
          <>
            <span className="sep" aria-hidden="true">
              ·
            </span>
            <span className="mono model">{model}</span>
          </>
        )}
      </div>
      <div className="card-foot">
        <span className={tone === "" ? "card-status" : `card-status ${tone}`}>
          <span className="dot" aria-hidden="true" />
          {status}
        </span>
        <span className="card-meta">
          {scoped ? null : `${labelOf(state, node)} · `}
          {ago(moment)}
        </span>
      </div>
      {session.status.state === "failed" ? (
        <div className="card-error">{session.status.message}</div>
      ) : null}
    </li>
  );
}
