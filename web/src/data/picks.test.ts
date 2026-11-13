import { describe, expect, it } from "vitest";

import { fixtureId } from "../testing/ids";
import type { OptionChoice, SessionOption } from "../wire/proto";
import { choicesFor, keptChoices, pick } from "./picks";

const OPTIONS: SessionOption[] = [
  {
    id: fixtureId("OptionId", "mode"),
    name: "Mode",
    category: "mode",
    kind: "select",
    current: fixtureId("OptionValueId", "default"),
    values: [
      { id: fixtureId("OptionValueId", "default"), name: "Default" },
      { id: fixtureId("OptionValueId", "plan"), name: "Plan Mode" },
    ],
  },
  {
    id: fixtureId("OptionId", "effort"),
    name: "Effort",
    category: "thought_level",
    kind: "select",
    current: fixtureId("OptionValueId", "high"),
    values: [
      { id: fixtureId("OptionValueId", "low"), name: "Low" },
      { id: fixtureId("OptionValueId", "high"), name: "High" },
    ],
  },
];

function select(option: string, value: string): OptionChoice {
  return {
    id: fixtureId("OptionId", option),
    kind: "select",
    value: fixtureId("OptionValueId", value),
  };
}

describe("the choices kept for the machine's next start", () => {
  const picks = pick(
    pick(pick({}, select("mode", "plan")), select("effort", "high")),
    select("gone", "x"),
  );

  it("are every pick the agent offers, a value it already has included", () => {
    // Sent: only what changes the agent's value.
    expect(choicesFor(OPTIONS, picks)).toEqual([select("mode", "plan")]);
    // Kept: the effort chosen too, though it is the agent's default today.
    expect(keptChoices(OPTIONS, picks)).toEqual([
      select("mode", "plan"),
      select("effort", "high"),
    ]);
  });

  it("are the picks as they are when the agent had not said what it offers", () => {
    expect(keptChoices(null, picks)).toEqual([
      select("mode", "plan"),
      select("effort", "high"),
      select("gone", "x"),
    ]);
  });
});
