// A session's title in the frame's bar, renamed where it stands
// (`session.rename`, protocol 9): a click (or Enter on it) turns it into
// a field; Enter or leaving the field saves, Escape keeps the old title.
// The rules are the node's (data/titles.ts checks them first); the bar
// shows the node's answer.

import { useEffect, useRef, useState } from "react";

import { useFleet, useFleetState } from "../../context";
import { renameSession } from "../../data/calls";
import { checkTitle, titleMax } from "../../data/titles";
import { TextField, Tooltip } from "../../ui/kit";
import { toast } from "../../ui/toasts";
import type { NodeKey } from "../../wire/bridge";
import type { SessionId } from "../../wire/proto";

export function SessionTitle({
  node,
  session,
  name,
  canRename,
}: {
  node: NodeKey;
  session: SessionId;
  /** What it is called now. */
  name: string;
  /** False for a session that has ended (the node refuses). */
  canRename: boolean;
}) {
  const fleet = useFleet();
  // The longest title this node takes: 200 from LONG_TITLES_PROTOCOL,
  // 120 before (TRACKER 801).
  const protocol = useFleetState((state) => {
    const status = state.statuses[node];
    return status?.state === "connected" ? status.protocol : null;
  });
  const [draft, setDraft] = useState<string | null>(null);
  // Set once Enter or Escape ended the edit, so the blur that follows
  // (the field leaving the page) does not save a second time.
  const ended = useRef(false);
  // Focus goes back to the title once the field leaves, as a dialog
  // returns it to its opener.
  const title = useRef<HTMLButtonElement>(null);
  const editing = draft !== null;
  const wasEditing = useRef(false);
  useEffect(() => {
    if (wasEditing.current && !editing) {
      title.current?.focus();
    }
    wasEditing.current = editing;
  }, [editing]);
  const [saved, setSaved] = useState<{ from: string; to: string } | null>(null);
  // The node's answer shows at once; the session's own report replaces it.
  const shown = saved !== null && saved.from === name ? saved.to : name;
  const start = () => {
    ended.current = false;
    setDraft(shown);
  };
  const save = (text: string) => {
    if (ended.current) {
      return;
    }
    ended.current = true;
    setDraft(null);
    const checked = checkTitle(text, titleMax(protocol));
    if (!checked.ok) {
      toast(checked.reason, "error");
      return;
    }
    if (checked.title === shown) {
      return;
    }
    void renameSession(fleet, node, session, checked.title).then((outcome) => {
      if (!outcome.ok) {
        toast(outcome.problem.message, "error");
        return;
      }
      setSaved({ from: name, to: outcome.value.title ?? checked.title });
    });
  };
  if (draft !== null) {
    return (
      <>
        <h1 className="visually-hidden">{shown}</h1>
        <span className="session-title-field">
          <TextField
            size="sm"
            aria-label="Title"
            value={draft}
            onChange={setDraft}
            autoFocus
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                event.preventDefault();
                save(draft);
              } else if (event.key === "Escape") {
                event.preventDefault();
                ended.current = true;
                setDraft(null);
              }
            }}
            onBlur={() => {
              save(draft);
            }}
          />
        </span>
      </>
    );
  }
  return (
    <h1 className="session-title">
      {canRename ? (
        <Tooltip label="Rename">
          <button
            ref={title}
            type="button"
            className="session-title-button"
            onClick={start}
          >
            {shown}
          </button>
        </Tooltip>
      ) : (
        <span className="session-title-text">{shown}</span>
      )}
    </h1>
  );
}
