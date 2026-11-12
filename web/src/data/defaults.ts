// What a new session on a machine starts with when the person has not
// chosen yet (DESIGN.md §7.5): the agent, profile and choices of the last
// session started there, kept with the bridge (ADR 0028), so they hold
// across browsers and launches.
//
// [Decision Log]
// - Purpose: the owner asked for defaults per agent; milestone 10 started
//   every new session on the first agent with no choices.
// - References: agent-orchestrator and Paseo remember the last selection
//   (per project and host, per provider) and start the next one from it;
//   GOAT keeps sticky selections per provider. None has a separate
//   screen of defaults for an agent, and Vessel adds none: what was last
//   started is what starts next.
// - Choice: on a successful start, the machine's (by node id, so a
//   renamed machine keeps them) last agent, and that agent's profile and
//   choices, are kept. On the new-session page they fill in only what the
//   person has not touched, and a choice the agent no longer offers is
//   dropped where every pick is (data/picks.ts). Bounded as the bridge
//   bounds them: 8 choices, ids and values of 64 bytes.

import type {
  ChoiceValue,
  Preferences,
  PreferenceChange,
} from "../wire/bridge";
import type {
  Harness,
  HarnessKind,
  NodeId,
  OptionChoice,
  OptionId,
} from "../wire/proto";
import type { Picks } from "./picks";
import { entriesOf } from "./records";

/** The most choices kept for an agent. */
export const MAX_CHOICES = 8;

/** The longest id or value kept, in UTF-8 bytes. */
export const MAX_CHOICE_BYTES = 64;

function short(text: string): boolean {
  return new TextEncoder().encode(text).length <= MAX_CHOICE_BYTES;
}

/** The agent a new session on `node` starts with, when it still has it. */
export function rememberedAgent(
  defaults: Preferences["defaults"],
  node: NodeId | null,
  harnesses: readonly Harness[],
): HarnessKind | null {
  const agent = node === null ? null : (defaults[node]?.agent ?? null);
  return harnesses.some((harness) => harness.kind === agent) ? agent : null;
}

/** The profile and choices kept for `agent` on `node`, as picks. */
export function rememberedPicks(
  defaults: Preferences["defaults"],
  node: NodeId | null,
  agent: HarnessKind | null,
): { profile: string; picks: Picks } | null {
  if (node === null || agent === null) {
    return null;
  }
  const kept = defaults[node]?.agents[agent];
  if (kept === undefined) {
    return null;
  }
  const picks: Record<OptionId, OptionChoice> = {};
  for (const [id, value] of entriesOf(kept.choices)) {
    picks[id] = { id, ...value };
  }
  return { profile: kept.profile ?? "", picks };
}

function valueOf(choice: OptionChoice): ChoiceValue {
  switch (choice.kind) {
    case "select":
      return { kind: "select", value: choice.value };
    case "toggle":
      return { kind: "toggle", on: choice.on };
  }
}

/** The changes that keep a started session's agent, profile and choices
 * for `node`. Choices past the bounds are not kept. */
export function rememberStart(
  node: NodeId,
  agent: HarnessKind,
  profile: string,
  choices: readonly OptionChoice[],
): PreferenceChange[] {
  const kept: Record<string, ChoiceValue> = {};
  for (const choice of choices) {
    const value = valueOf(choice);
    const fits =
      short(choice.id) && (value.kind === "toggle" || short(value.value));
    if (fits && Object.keys(kept).length < MAX_CHOICES) {
      kept[choice.id] = value;
    }
  }
  return [
    { set: "default_agent", value: { node, agent } },
    {
      set: "agent_defaults",
      value: {
        node,
        agent,
        profile: profile === "" ? null : profile,
        choices: kept,
      },
    },
  ];
}
