// "Add to chat" beside a selection in the code panel (agent-orchestrator's
// `SelectionAskButton`): a small floating button at the selection's end;
// Escape puts it away. What it sends is `use-selection.ts`'s.

import "./selection-ask.css";

import { useEffect } from "react";

import { Button } from "../../ui/kit";
import type { Ask } from "./use-selection";

export function SelectionAsk({
  ask,
  onSend,
  onDismiss,
}: {
  ask: Ask;
  onSend: () => void;
  onDismiss: () => void;
}) {
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onDismiss();
      }
    };
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("keydown", onKey);
    };
  }, [onDismiss]);
  return (
    <div
      className="selection-ask"
      style={{
        left: Math.min(ask.x, window.innerWidth - 120),
        top: Math.max(ask.y, 8),
      }}
      // A press here must not move the selection it is about.
      onMouseDown={(event) => {
        event.preventDefault();
      }}
    >
      <Button size="xs" variant="primary" icon="message" onClick={onSend}>
        Add to chat
      </Button>
    </div>
  );
}
