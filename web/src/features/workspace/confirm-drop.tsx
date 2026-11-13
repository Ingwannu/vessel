// A drop nothing brings back (a stash, a worktree's snapshot), asked twice
// in its place: the first press turns "Drop" into "Keep" and "Drop for
// good", so a stray click loses nothing. In place rather than in a
// dialog, since the stash list is a dialog already and two would stack.

import "./confirm-drop.css";

import { useState } from "react";

import { Button } from "../../ui/kit";

export function ConfirmDrop({
  what,
  busy = false,
  disabled = false,
  onDrop,
}: {
  /** What is dropped, for the pair's name ("stash@{0}"). */
  what: string;
  busy?: boolean;
  disabled?: boolean;
  onDrop: () => void;
}) {
  const [asking, setAsking] = useState(false);
  if (!asking) {
    return (
      <Button
        size="xs"
        variant="danger"
        disabled={disabled}
        onClick={() => {
          setAsking(true);
        }}
      >
        Drop
      </Button>
    );
  }
  return (
    <span className="confirm-drop" role="group" aria-label={`Drop ${what}?`}>
      <Button
        size="xs"
        variant="ghost"
        onClick={() => {
          setAsking(false);
        }}
      >
        Keep
      </Button>
      <Button
        size="xs"
        variant="danger-solid"
        busy={busy}
        disabled={disabled}
        onClick={onDrop}
      >
        Drop for good
      </Button>
    </span>
  );
}
