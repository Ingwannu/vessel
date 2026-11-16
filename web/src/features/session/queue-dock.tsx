// The prompts waiting in the node's queue, over the composer
// (agent-orchestrator's `QueuedMessageDock`): a header with the count and,
// when there is more than one, a chevron that folds the list to the next
// one; a 40px row per prompt with its first line, then Steer or Send now,
// Edit, Delete and a grip to reorder (dragged, or Up and Down on the grip,
// GOAT's keys). A held queue says why and offers Resume (GOAT's).
//
// The rows are what the node last said (`queue_changed`); a change is
// asked of the node and shows when the node says it happened.

import "./queue-dock.css";

import { type PointerEvent, useRef, useState } from "react";

import { holdLine, moveBefore, type QueueRow } from "../../data/queue-view";
import { Button, Icon, IconButton, Tooltip } from "../../ui/kit";
import type { QueueEntryId, QueueHold } from "../../wire/proto";

/** The row height (the reference's 40px): what a drag moves by. */
const ROW = 40;

export interface QueueActions {
  /** What Send now does, in the row's words ("Steer", "Send now"). */
  sendNowLabel: string;
  sendNowNote: string;
  onSendNow: (row: QueueRow) => void;
  onEdit: (row: QueueRow) => void;
  onRemove: (row: QueueRow) => void;
  onMove: (row: QueueRow, before: QueueEntryId | null) => void;
  onResume: () => void;
}

export function QueueDock({
  rows,
  held,
  editing,
  actions,
}: {
  rows: readonly QueueRow[];
  held: QueueHold | null;
  /** The entry being edited in the composer. */
  editing: QueueEntryId | null;
  actions: QueueActions;
}) {
  const [folded, setFolded] = useState(false);
  const [drag, setDrag] = useState<{ from: number; to: number } | null>(null);
  const start = useRef(0);
  if (rows.length === 0) {
    return null;
  }
  const shown = folded ? rows.slice(0, 1) : rows;
  const count = `${String(rows.length)} Queued ${rows.length === 1 ? "Message" : "Messages"}`;
  const grip = (row: QueueRow, index: number) => ({
    onPointerDown: (event: PointerEvent<HTMLButtonElement>) => {
      event.currentTarget.setPointerCapture(event.pointerId);
      start.current = event.clientY;
      setDrag({ from: index, to: index });
    },
    onPointerMove: (event: PointerEvent<HTMLButtonElement>) => {
      if (drag?.from === index) {
        const to = index + Math.round((event.clientY - start.current) / ROW);
        setDrag({
          from: index,
          to: Math.max(0, Math.min(rows.length - 1, to)),
        });
      }
    },
    onPointerUp: () => {
      if (drag?.from === index) {
        const before = moveBefore(rows, index, drag.to);
        if (before !== undefined) {
          actions.onMove(row, before);
        }
      }
      setDrag(null);
    },
    onPointerCancel: () => {
      setDrag(null);
    },
  });
  return (
    <section
      className="queue-dock"
      aria-label={`${String(rows.length)} queued messages`}
    >
      <header className="queue-head">
        {rows.length > 1 ? (
          <span className="queue-chevron" data-folded={folded ? "" : undefined}>
            <IconButton
              label={
                folded ? "Expand queued messages" : "Collapse queued messages"
              }
              icon="chevron-down"
              size="xs"
              aria-expanded={!folded}
              onClick={() => {
                setFolded(!folded);
              }}
            />
          </span>
        ) : null}
        <span className="queue-count">
          {count}
          {editing === null ? "" : " · editing"}
        </span>
        {held === null ? null : (
          <>
            <span className="queue-held" role="status">
              {holdLine(held)}
            </span>
            <Button size="xs" variant="ghost" onClick={actions.onResume}>
              Resume
            </Button>
          </>
        )}
      </header>
      <ol className="queue-rows">
        {shown.map((row, index) => (
          <li
            key={row.id}
            className="queue-row"
            data-editing={row.id === editing ? "" : undefined}
            data-dragging={drag?.from === index ? "" : undefined}
            data-drop={
              drag !== null && drag.from !== index && drag.to === index
                ? drag.to > drag.from
                  ? "after"
                  : "before"
                : undefined
            }
          >
            <span className="queue-mark" aria-hidden="true">
              <Icon name="circle" size={12} />
            </span>
            <span className="queue-text">
              {row.internalNotice !== undefined
                ? "Agent notification"
                : row.text === ""
                  ? `${String(row.attachments.length)} files`
                  : row.text}
            </span>
            {row.attachments.length > 0 && row.text !== "" ? (
              <span className="queue-files">
                <Icon name="paperclip" size={12} />
                {row.attachments.length}
              </span>
            ) : null}
            {row.by === null ? null : (
              <span className="queue-by">by {row.by}</span>
            )}
            <span className="queue-actions">
              <Tooltip label={actions.sendNowNote}>
                <Button
                  size="xs"
                  variant="ghost"
                  onClick={() => {
                    actions.onSendNow(row);
                  }}
                >
                  {actions.sendNowLabel}
                </Button>
              </Tooltip>
              <Tooltip label="Edit">
                <IconButton
                  label="Edit queued message"
                  icon="edit"
                  size="xs"
                  onClick={() => {
                    actions.onEdit(row);
                  }}
                />
              </Tooltip>
              <Tooltip label="Delete">
                <IconButton
                  label="Delete queued message"
                  icon="trash"
                  size="xs"
                  onClick={() => {
                    actions.onRemove(row);
                  }}
                />
              </Tooltip>
              {rows.length > 1 && !folded ? (
                <Tooltip label="Drag to reorder">
                  <IconButton
                    label="Drag to reorder queued message"
                    icon="grip"
                    size="xs"
                    {...grip(row, index)}
                    onKeyDown={(event) => {
                      const step =
                        event.key === "ArrowUp"
                          ? -1
                          : event.key === "ArrowDown"
                            ? 1
                            : 0;
                      const before =
                        step === 0
                          ? undefined
                          : moveBefore(rows, index, index + step);
                      if (before !== undefined) {
                        event.preventDefault();
                        actions.onMove(row, before);
                      }
                    }}
                  />
                </Tooltip>
              ) : null}
            </span>
          </li>
        ))}
      </ol>
    </section>
  );
}
