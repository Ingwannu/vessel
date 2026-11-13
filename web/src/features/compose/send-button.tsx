// The composer's one primary action (agent-orchestrator's): a 28px round
// button; an arrow to send or queue, GOAT's turned arrow to steer into
// the running turn, a square to stop it, a spinner while a send or its
// files are on their way. While ⌘ or Ctrl is held it shows, and a click
// does, the other way (all three references), so the icon always says
// what a click will do.

import { useEffect, useState } from "react";

import { deliveryLabel, deliveryNote } from "../../data/delivery";
import { currentPlatform } from "../../data/shortcuts/platform";
import { IconButton, type IconName, Tooltip } from "../../ui/kit";
import type { Delivery } from "../../wire/proto";

export type SendAction =
  | { kind: "send"; label: string; disabled: boolean; busy: boolean }
  | {
      kind: "deliver";
      delivery: Delivery;
      /** What ⌘/Ctrl sends instead; `null` when there is no other way. */
      alternate: Delivery | null;
      /** Why the person's choice is not the one used. */
      fallback: string | null;
      /** Whether a turn runs: between turns every delivery reads Send. */
      turn: boolean;
      disabled: boolean;
      busy: boolean;
      onSend: (other: boolean) => void;
    }
  | { kind: "stop"; onStop: () => void }
  /** A queued prompt being edited in the composer (GOAT's check). */
  | { kind: "update"; busy: boolean; disabled: boolean; onUpdate: () => void };

function deliveryIcon(delivery: Delivery): IconName {
  return delivery === "steer" ? "steer" : "arrow-up";
}

/** Whether ⌘ or Ctrl is held now (the window's keys, while the page has
 * focus). */
function useModifier(watch: boolean): boolean {
  const [held, setHeld] = useState(false);
  useEffect(() => {
    if (!watch) {
      return undefined;
    }
    const update = (event: KeyboardEvent) => {
      setHeld(event.metaKey || event.ctrlKey);
    };
    const release = () => {
      setHeld(false);
    };
    window.addEventListener("keydown", update);
    window.addEventListener("keyup", update);
    window.addEventListener("blur", release);
    return () => {
      window.removeEventListener("keydown", update);
      window.removeEventListener("keyup", update);
      window.removeEventListener("blur", release);
    };
  }, [watch]);
  return watch && held;
}

export function SendButton({ action }: { action: SendAction }) {
  const held = useModifier(
    action.kind === "deliver" && action.alternate !== null,
  );
  switch (action.kind) {
    case "send":
      return (
        <Tooltip label={action.label} shortcut="↵">
          <IconButton
            label={action.label}
            icon="arrow-up"
            variant="primary"
            shape="round"
            type="submit"
            disabled={action.disabled || action.busy}
            aria-busy={action.busy || undefined}
          />
        </Tooltip>
      );
    case "deliver": {
      const shown =
        held && action.alternate !== null ? action.alternate : action.delivery;
      const mod = currentPlatform() === "mac" ? "⌘" : "Ctrl+";
      const lines = [deliveryNote(shown)];
      if (action.alternate !== null && !held) {
        lines.push(
          `${mod}↵ or ${mod}click: ${deliveryLabel(action.alternate).toLowerCase()}`,
        );
      }
      if (action.fallback !== null && shown === action.delivery) {
        lines.push(action.fallback);
      }
      return (
        <Tooltip
          label={action.turn ? deliveryLabel(shown) : "Send"}
          shortcut="↵"
          {...(action.turn ? { lines } : {})}
        >
          <IconButton
            label={action.turn ? deliveryLabel(shown) : "Send"}
            icon={deliveryIcon(shown)}
            variant="primary"
            shape="round"
            disabled={action.disabled || action.busy}
            aria-busy={action.busy || undefined}
            onClick={(event) => {
              action.onSend(event.metaKey || event.ctrlKey);
            }}
          />
        </Tooltip>
      );
    }
    case "update":
      return (
        <Tooltip label="Update queued message" shortcut="↵">
          <IconButton
            label={
              action.busy ? "Updating queued message" : "Update queued message"
            }
            icon="check"
            variant="primary"
            shape="round"
            disabled={action.disabled || action.busy}
            aria-busy={action.busy || undefined}
            onClick={action.onUpdate}
          />
        </Tooltip>
      );
    case "stop":
      return (
        <Tooltip label="Stop turn">
          <IconButton
            label="Stop turn"
            icon="stop"
            variant="primary"
            shape="round"
            onClick={action.onStop}
          />
        </Tooltip>
      );
  }
}
