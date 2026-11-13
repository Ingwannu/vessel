// What the chosen agent offers, asked of its machine (`harness.options`,
// ADR 0022). The node starts the agent to answer the first time, so this
// takes seconds: the page shows "Loading models…" meanwhile and stays
// usable, and an answer for an earlier choice of machine, agent or
// profile is dropped when it arrives late. A machine that will not start
// the agent now (`conflict`: an install running, a secret or a route's
// key not set there, ADR 0032) is said in the page's words.

import { useEffect, useRef, useState } from "react";

import { useAgents, useFleet } from "../../context";
import { refusedStart } from "../../data/agent-choices";
import { agentKey } from "../../data/agents";
import { probeOptions } from "../../data/calls";
import type { OptionsState } from "../../data/new-session";
import { nodeError } from "../../data/workspace/root";
import type { HarnessRef } from "../../wire/proto";
import type { NodeKey } from "../../wire/bridge";

/** The options of `agent` under `profile` on `machine`; `wanted` is false
 * when nothing needs them (a terminal, no agent). */
export function useAgentOptions(
  machine: NodeKey | null,
  agent: HarnessRef | null,
  profile: string,
  wanted: boolean,
): OptionsState {
  const fleet = useFleet();
  // Read when an answer arrives, not dependencies: a new object for the
  // same agent (`key` names it) or a label read later asks nothing again.
  // (This effect runs before the one that asks, so that one reads the
  // agent of this drawing.)
  const agents = useAgents(machine);
  const chosen = useRef({ agent, agents });
  useEffect(() => {
    chosen.current = { agent, agents };
  });
  const key = `${machine ?? ""}\u0000${agent === null ? "" : agentKey(agent)}\u0000${profile}`;
  const [answer, setAnswer] = useState<{
    key: string;
    state: OptionsState;
  } | null>(null);
  useEffect(() => {
    const agent = chosen.current.agent;
    if (!wanted || machine === null || agent === null) {
      return undefined;
    }
    let current = true;
    void probeOptions(
      fleet,
      machine,
      agent,
      profile === "" ? null : profile,
    ).then((outcome) => {
      if (!current) {
        return;
      }
      setAnswer({
        key,
        state: outcome.ok
          ? { kind: "ready", options: outcome.value.options }
          : nodeError(outcome.problem) === "conflict"
            ? {
                kind: "refused",
                message: refusedStart(agent, chosen.current.agents),
              }
            : { kind: "failed", message: outcome.problem.message },
      });
    });
    return () => {
      current = false;
    };
  }, [fleet, machine, profile, wanted, key]);
  return answer?.key === key ? answer.state : { kind: "loading" };
}
