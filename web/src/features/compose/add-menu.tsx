// The box's "+" (agent-orchestrator's add menu): what a new session starts
// with besides its prompt. A profile, when the machine has any for the
// agent (an account, or a model route, ADR 0015), and "Open as terminal"
// with the arguments the agent's command gets.
//
// A popover rather than a menu: it holds a text field, which a menu's
// typeahead and roving focus would fight.

import "./add-menu.css";

import type { ProfileChoice } from "../../data/agents";
import {
  Field,
  IconButton,
  Popover,
  Select,
  Switch,
  TextField,
  Tooltip,
} from "../../ui/kit";

export interface AddMenuProps {
  profiles: readonly ProfileChoice[];
  profile: string;
  onProfile: (profile: string) => void;
  terminal: boolean;
  /** `false` when the agent runs only in a terminal on this machine. */
  canTimeline: boolean;
  onTerminal: (terminal: boolean) => void;
  args: string;
  onArgs: (args: string) => void;
}

export function AddMenu({
  profiles,
  profile,
  onProfile,
  terminal,
  canTimeline,
  onTerminal,
  args,
  onArgs,
}: AddMenuProps) {
  return (
    <Popover
      title="Start with"
      width="md"
      side="top"
      align="start"
      trigger={
        <Tooltip label="Profile and terminal">
          <IconButton label="Add" icon="plus" shape="round" />
        </Tooltip>
      }
    >
      <div className="add-body">
        {profiles.length > 1 ? (
          <Field label="Profile">
            <Select value={profile} options={profiles} onChange={onProfile} />
          </Field>
        ) : null}
        <Switch
          label="Open as terminal"
          showLabel
          checked={terminal || !canTimeline}
          disabled={!canTimeline}
          onChange={onTerminal}
        />
        {terminal || !canTimeline ? (
          <Field
            label="Arguments"
            hint="Passed to the agent's command as typed."
          >
            <TextField
              mono
              size="sm"
              value={args}
              onChange={onArgs}
              placeholder="--flag value"
            />
          </Field>
        ) : null}
      </div>
    </Popover>
  );
}
