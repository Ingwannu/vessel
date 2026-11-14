// The new-session box's row of controls: the "+" (files for the first
// message, a profile, a terminal where the machine offers one for the
// agent), the agent with its model and effort (the four, then the agents
// added on the machine, ADR 0032), and its permissions (DESIGN.md §6.15).

import type { ProfileChoice } from "../../data/agent-choices";
import { agentChoices } from "../../data/agent-choices";
import { type AgentModes, offersTerminal } from "../../data/agents";
import type { Draft } from "../../data/new-session";
import type { ControlsState } from "../../data/options";
import type { Tray } from "../../data/tray";
import type {
  AgentInfo,
  Harness,
  HarnessRef,
  OptionChoice,
} from "../../wire/proto";
import { AddMenu } from "./add-menu";
import { AgentMenu } from "./agent-menu";
import { PermissionsMenu } from "./permissions-menu";

export function NewSessionControls({
  profiles,
  draft,
  effective,
  change,
  restart,
  terminal,
  modes,
  harnesses,
  agents,
  agent,
  agentLabel,
  controls,
  choose,
  tray,
}: {
  profiles: readonly ProfileChoice[];
  /** What the person chose. */
  draft: Draft;
  /** With the machine's remembered start filled in. */
  effective: Draft;
  change: (next: Partial<Draft>) => void;
  restart: (next: Partial<Draft>) => void;
  terminal: boolean;
  /** How the machine runs the agent; `null` before one is known. */
  modes: AgentModes | null;
  harnesses: readonly Harness[];
  /** The machine's `agent.list`, for an added agent's label. */
  agents: readonly AgentInfo[];
  agent: HarnessRef | null;
  /** What the page calls the agent. */
  agentLabel: string;
  controls: ControlsState;
  choose: (choice: OptionChoice) => void;
  tray: Tray;
}) {
  return (
    <>
      {/* Nothing to offer, nothing drawn: an added agent with no profile
          has no terminal, and a terminal takes no files. */}
      {!terminal || profiles.length > 1 || offersTerminal(modes) ? (
        <AddMenu
          profiles={profiles}
          profile={effective.profile}
          onProfile={(profile) => {
            change({ profile, picks: {}, chosen: true });
          }}
          terminal={draft.terminal}
          modes={modes}
          onTerminal={(on) => {
            change({ terminal: on });
          }}
          args={draft.args}
          onArgs={(args) => {
            change({ args });
          }}
          {...(terminal
            ? {}
            : {
                onFiles: (files: File[]) => {
                  tray.add(files);
                },
              })}
        />
      ) : null}
      {agent === null ? null : (
        <AgentMenu
          agent={{
            value: agent,
            name: agentLabel,
            choices: agentChoices(harnesses, agents),
            onChange: (next) => {
              restart({ agent: next });
            },
          }}
          state={controls}
          onChoose={choose}
        />
      )}
      {controls.kind === "ready" && controls.controls.permissions !== null ? (
        <PermissionsMenu
          control={controls.controls.permissions}
          onChoose={choose}
        />
      ) : null}
    </>
  );
}
