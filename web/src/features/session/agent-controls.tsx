// A running session's agent controls in the composer's row (DESIGN.md
// §6.15): the agent with its model and effort, then its permissions,
// reading what the session's log last reported (`options_changed`) and
// changing it with `session.set_option`. A change shows when the node
// reports it; while one is on its way the controls wait; the agent's
// refusal is said in a toast.

import { useState } from "react";

import { useFleet } from "../../context";
import { setOption } from "../../data/calls";
import { reportedControls } from "../../data/options";
import { toast } from "../../ui/toasts";
import type { NodeKey } from "../../wire/bridge";
import type {
  HarnessKind,
  OptionChoice,
  SessionId,
  SessionOption,
} from "../../wire/proto";
import { AgentMenu } from "../compose/agent-menu";
import { PermissionsMenu } from "../compose/permissions-menu";

export function AgentControls({
  node,
  session,
  harness,
  agent,
  options,
  protocol,
  live,
}: {
  node: NodeKey;
  session: SessionId;
  harness: HarnessKind;
  /** The agent's name. */
  agent: string;
  options: readonly SessionOption[] | null;
  protocol: number | null;
  live: boolean;
}) {
  const fleet = useFleet();
  const [changing, setChanging] = useState(false);
  const controls = reportedControls(options, protocol, agent);
  const choose = (choice: OptionChoice) => {
    setChanging(true);
    void setOption(fleet, node, session, choice).then((outcome) => {
      setChanging(false);
      if (!outcome.ok) {
        toast(outcome.problem.message, "error");
      }
    });
  };
  return (
    <>
      <AgentMenu
        agent={{
          value: harness,
          name: agent,
          choices: null,
          onChange: () => undefined,
        }}
        state={controls}
        onChoose={choose}
        disabled={!live || changing}
      />
      {controls.kind === "ready" && controls.controls.permissions !== null ? (
        <PermissionsMenu
          control={controls.controls.permissions}
          onChoose={choose}
          disabled={!live || changing}
        />
      ) : null}
    </>
  );
}
