// Sessions (DESIGN.md §2.4, §6.12): every session on one node or all, as
// agent-orchestrator's board. A lane is where a session is in delivery
// and a card is a session; both the lane and the card's phrase come from
// the node (data/board.ts). Ended sessions are the archive, listed apart.

import { Link } from "@tanstack/react-router";
import { useState, type ReactElement } from "react";

import { openNewSession, useCrumbs, useFleet, useFleetState } from "../context";
import {
  board,
  type Card,
  isWorking,
  type Lane,
  laneLabel,
  LANES,
  needsYou,
  phraseLabel,
  sinceLabel,
  tone,
} from "../data/board";
import { searchArchive } from "../data/archive";
import type { FleetState } from "../data/fleet";
import {
  harnessMark,
  harnessName,
  isLive,
  sessionTitle,
  shortId,
} from "../data/format";
import { labelOf, nodeLabel } from "../data/nodes";
import { tree } from "../data/tree";
import { useWhere } from "../data/where";
import {
  Button,
  Dialog,
  EmptyState,
  Icon,
  IconButton,
  TextField,
  Tooltip,
} from "../ui/kit";
import { toast } from "../ui/toasts";

/** How many archived cards the archive shows before "Show more": a node
 * keeps every ended session's log, so the archive grows without bound. */
const ARCHIVE_PAGE = 30;

export function SessionsPage(): ReactElement {
  const where = useWhere();
  const scope = where.node ?? null;
  const fleet = useFleet();
  const state = useFleetState((value) => value);
  const [killing, setKilling] = useState<Card | null>(null);
  const [archiveOpen, setArchiveOpen] = useState(false);
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
  const dealt = board(rows);
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
      {/* The node scope, only when there is a choice to make. The page's
          title is in the frame's title bar. */}
      {book.length > 1 ? (
        <div className="board-scope">
          <div className="segmented sm" role="group" aria-label="Node">
            <Link
              className="segment"
              data-checked={scope === null ? "" : undefined}
              to="/sessions"
              aria-current={scope === null ? "true" : undefined}
            >
              All nodes
            </Link>
            {book.map((node) => (
              <Link
                key={node.key}
                className="segment"
                data-checked={scope === node.key ? "" : undefined}
                to="/sessions"
                search={{ node: node.key }}
                aria-current={scope === node.key ? "true" : undefined}
              >
                {nodeLabel(node, state.statuses[node.key])}
              </Link>
            ))}
          </div>
        </div>
      ) : null}
      {rows.length === 0 ? (
        <div className="page-body">
          <EmptyState
            title={
              scopeLabel === null
                ? "No sessions yet"
                : `No sessions on ${scopeLabel} yet`
            }
            action={
              <Button
                variant="primary"
                onClick={() => {
                  openNewSession(scope);
                }}
              >
                New session
              </Button>
            }
          >
            Start one and it appears here, in the lane of what it is doing.
          </EmptyState>
        </div>
      ) : (
        <>
          <div className="board" role="group" aria-label="Sessions by status">
            {LANES.map((lane) => (
              <LaneView
                key={lane}
                lane={lane}
                cards={dealt.lanes[lane]}
                state={state}
                scoped={scope !== null}
                onKill={setKilling}
              />
            ))}
          </div>
          {/* The archive is not a lane: a quiet count under the lanes that
              opens the ended sessions. */}
          <div className="archive-bar">
            <Button
              variant="ghost"
              size="xs"
              icon="archive"
              disabled={dealt.archive.length === 0}
              onClick={() => {
                setArchiveOpen(true);
              }}
            >
              Archive
              <span className="count">{dealt.archive.length}</span>
            </Button>
          </div>
        </>
      )}
      {archiveOpen ? (
        <Archive
          cards={dealt.archive}
          state={state}
          scoped={scope !== null}
          onClose={() => {
            setArchiveOpen(false);
          }}
        />
      ) : null}
      {killing === null ? null : (
        <Dialog
          title={`Kill ${shortId(killing.session.id)}?`}
          onClose={() => {
            setKilling(null);
          }}
          footer={
            <>
              <Button
                variant="ghost"
                onClick={() => {
                  setKilling(null);
                }}
              >
                Cancel
              </Button>
              <Button
                variant="danger-solid"
                onClick={() => {
                  kill(killing);
                }}
              >
                Kill
              </Button>
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

/** One lane: its heading (a dot and the name in the lane's colour, the
 * count) over its cards. */
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
  const label = laneLabel(lane);
  return (
    <section className="lane" aria-label={label} data-lane={lane}>
      <header className="lane-head">
        <span className="swatch" aria-hidden="true" />
        <h2>{label}</h2>
        <span className="count" aria-label={`${cards.length} sessions`}>
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
            />
          ))}
        </ul>
      </div>
    </section>
  );
}

/** The ended sessions, as the same cards, newest first, with a search. */
function Archive({
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
        placeholder="Search by directory, id or node"
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

/** Who started a session, when it was not this machine's owner token: a
 * paired device, or another session's agent (linked). */
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

/** One session's card (DESIGN.md §6.12): the agent's mark and the title,
 * where it works, who runs it, then one status line in the lane's colour.
 * The whole card opens the session. */
function SessionCard({
  card,
  state,
  scoped,
  onKill,
}: {
  card: Card;
  state: FleetState;
  scoped: boolean;
  /** `null` where a card offers no action (the archive). */
  onKill: ((card: Card) => void) | null;
}) {
  const { session, node, placed } = card;
  const spec = session.spec;
  const id = shortId(session.id);
  const title = sessionTitle(spec.cwd);
  const phrase = phraseLabel(placed.phrase);
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
        aria-label={`Open ${id}: ${title}, ${phrase}, on ${labelOf(state, node)}`}
      />
      <div className="card-head">
        <CardOrigin card={card} state={state} />
        <div className="card-title">
          <span
            className="avatar"
            role="img"
            aria-label={harnessName(spec.harness)}
          >
            {harnessMark(spec.harness)}
          </span>
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
        <div className="card-line">
          <span className="id">{id}</span>
          <span className="who">
            {session.profile ?? harnessName(spec.harness)}
            {model === null || model === undefined ? "" : ` · ${model}`}
          </span>
        </div>
      </div>
      <div className="card-foot">
        <span className={`card-status tone-${tone(placed)}`}>
          {isWorking(placed) ? (
            <span className="spinner" aria-hidden="true" />
          ) : null}
          <span className="phrase">{phrase}</span>
        </span>
        <span className="card-meta">
          {scoped ? null : `${labelOf(state, node)} · `}
          {sinceLabel(placed.at)}
        </span>
      </div>
      {session.status.state === "failed" ? (
        <div className="card-error">{session.status.message}</div>
      ) : null}
    </li>
  );
}
