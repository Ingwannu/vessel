// The board (DESIGN.md §2.4, §6.12): every session on one machine or all,
// as agent-orchestrator's board. A lane is where a session is in delivery
// and a card is a session; both the lane and the card's phrase come from
// the node (data/board.ts). Ended sessions are the archive, listed apart.

import "./board.css";

import { Link } from "@tanstack/react-router";
import { useState } from "react";

import { useCrumbs, useFleet, useFleetState } from "../../context";
import { board, type Card, LANES } from "../../data/board";
import { killSession } from "../../data/calls";
import { shortId } from "../../data/format";
import { labelOf, nodeLabel } from "../../data/nodes";
import { tree } from "../../data/tree";
import { Button, Dialog, EmptyState } from "../../ui/kit";
import { toast } from "../../ui/toasts";
import { useNewSession } from "../shell/use-new-session";
import { Archive } from "./archive";
import { Lane } from "./lane";
import type { NodeKey } from "../../wire/bridge";

export function Board({ scope }: { scope: NodeKey | null }) {
  const fleet = useFleet();
  const state = useFleetState((value) => value);
  const open = useNewSession();
  const [killing, setKilling] = useState<Card | null>(null);
  const [archiveOpen, setArchiveOpen] = useState(false);
  const book = state.book ?? [];
  const nodes = scope === null ? book.map((node) => node.key) : [scope];
  const scopeLabel = scope === null ? null : labelOf(state, scope);
  useCrumbs(
    scopeLabel === null
      ? [{ label: "Board" }]
      : [{ label: "Board", to: "/sessions" }, { label: scopeLabel }],
  );
  const rows = tree(state, nodes);
  const dealt = board(rows);
  const kill = (card: Card) => {
    setKilling(null);
    void killSession(fleet, card.node, card.session.id).then((outcome) => {
      if (outcome.ok) {
        toast(`Killed ${shortId(card.session.id)}`);
        void fleet.refreshSessions(card.node);
      } else {
        toast(outcome.problem.message, "error");
      }
    });
  };
  const startNew = () => {
    open(scope);
  };
  return (
    <div className="page">
      {/* The machine scope, only when there is a choice to make. The
          page's title is in the frame's title bar. */}
      {book.length > 1 ? (
        <div className="board-scope">
          <div className="segmented sm" role="group" aria-label="Machine">
            <Link
              className="segment"
              data-checked={scope === null ? "" : undefined}
              to="/sessions"
              aria-current={scope === null ? "true" : undefined}
            >
              All machines
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
              <Button variant="primary" onClick={startNew}>
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
              <Lane
                key={lane}
                lane={lane}
                cards={dealt.lanes[lane]}
                state={state}
                scoped={scope !== null}
                onKill={setKilling}
                onNew={startNew}
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
            The agent on {labelOf(state, killing.node)} and everything it
            started stop now. The session&apos;s log stays.
          </p>
        </Dialog>
      )}
    </div>
  );
}
