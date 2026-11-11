// What the chosen agent offers, asked of its machine (`harness.options`,
// ADR 0022). The node starts the agent to answer the first time, so this
// takes seconds: the page shows "Loading models…" meanwhile and stays
// usable, and an answer for an earlier choice of machine, agent or
// profile is dropped when it arrives late.

import { useEffect, useState } from "react";

import { useFleet } from "../../context";
import { probeOptions } from "../../data/calls";
import type { OptionsState } from "../../data/new-session";
import type { HarnessKind } from "../../wire/proto";
import type { NodeKey } from "../../wire/bridge";

/** The options of `agent` under `profile` on `machine`; `wanted` is false
 * when nothing needs them (a terminal, no agent). */
export function useAgentOptions(
  machine: NodeKey | null,
  agent: HarnessKind | null,
  profile: string,
  wanted: boolean,
): OptionsState {
  const fleet = useFleet();
  const key = `${machine ?? ""}\u0000${agent ?? ""}\u0000${profile}`;
  const [answer, setAnswer] = useState<{
    key: string;
    state: OptionsState;
  } | null>(null);
  useEffect(() => {
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
          : { kind: "failed", message: outcome.problem.message },
      });
    });
    return () => {
      current = false;
    };
  }, [fleet, machine, agent, profile, wanted, key]);
  return answer?.key === key ? answer.state : { kind: "loading" };
}
