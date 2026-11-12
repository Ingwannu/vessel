// The new-session box's row of controls: the "+" (files for the first
// message, a profile, a terminal), the agent with its model and effort,
// and its permissions (DESIGN.md §6.15).

import type { ProfileChoice } from "../../data/agent-choices";
import { agentChoices } from "../../data/agent-choices";
import { agentName, runsTimeline } from "../../data/agents";
import type { Draft } from "../../data/new-session";
import type { ControlsState } from "../../data/options";
import type { Tray } from "../../data/tray";
import type { Harness, HarnessKind, OptionChoice } from "../../wire/proto";
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
  harness,
  harnesses,
  agent,
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
  harness: Harness | null;
  harnesses: readonly Harness[];
  agent: HarnessKind | null;
  controls: ControlsState;
  choose: (choice: OptionChoice) => void;
  tray: Tray;
}) {
  return (
    <>
      <AddMenu
        profiles={profiles}
        profile={effective.profile}
        onProfile={(profile) => {
          change({ profile, picks: {}, chosen: true });
        }}
        terminal={draft.terminal}
        canTimeline={harness === null || runsTimeline(harness)}
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
      {agent === null ? null : (
        <AgentMenu
          agent={{
            value: agent,
            name: agentName(agent),
            choices: agentChoices(harnesses),
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
