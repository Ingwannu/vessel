// The box's "+" (agent-orchestrator's add menu): what a new session starts
// with besides its prompt. Files to attach to the first message (its
// "Files — Attach files to this message"), a profile, when the machine
// has any for the agent (an account, or a model route, ADR 0015), and
// "Open as terminal" with the arguments the agent's command gets.
//
// A popover rather than a menu: it holds a text field, which a menu's
// typeahead and roving focus would fight.

import "./add-menu.css";

import type { ProfileChoice } from "../../data/agent-choices";
import {
  Button,
  Field,
  IconButton,
  Popover,
  Select,
  Switch,
  TextField,
  Tooltip,
  useFilePicker,
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
  /** Files picked for the first message; absent where files are not
   * taken (a terminal). */
  onFiles?: (files: File[]) => void;
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
  onFiles,
}: AddMenuProps) {
  const picker = useFilePicker((files) => {
    onFiles?.(files);
  });
  return (
    <Popover
      title="Start with"
      width="md"
      side="top"
      align="start"
      trigger={
        <Tooltip label="Files, profile and terminal">
          <IconButton label="Add" icon="plus" shape="round" />
        </Tooltip>
      }
    >
      <div className="add-body">
        {onFiles === undefined ? null : (
          <Button
            variant="ghost"
            size="sm"
            icon="paperclip"
            onClick={picker.open}
          >
            Attach files to this message
          </Button>
        )}
        {picker.input}
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
