// The composer's one primary action (agent-orchestrator's): a 28px round
// button, an arrow to send, a square to stop the turn that runs, a
// spinner while a send is on its way.

import { IconButton, Tooltip } from "../../ui/kit";

export type SendAction =
  | { kind: "send"; label: string; disabled: boolean; busy: boolean }
  | { kind: "stop"; onStop: () => void };

export function SendButton({ action }: { action: SendAction }) {
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
    case "stop":
      return (
        <Tooltip label="Stop the turn">
          <IconButton
            label="Stop the turn"
            icon="stop"
            variant="primary"
            shape="round"
            onClick={action.onStop}
          />
        </Tooltip>
      );
  }
}
