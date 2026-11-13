// Choices made on the new-session page before the session exists
// ("picks"), kept apart from what the agent reported (data/options.ts).
//
// Only the picks that differ from the agent's own values are sent, in the
// order the agent lists its options: the node applies them in order, and
// a choice the agent does not offer fails the session (ADR 0022), so a
// pick made for another agent or profile is dropped rather than sent.

import type { OptionChoice, OptionId, SessionOption } from "../wire/proto";

/** Choices made before a session exists, by option id. */
export type Picks = Readonly<Record<OptionId, OptionChoice>>;

/** Whether the agent would take `choice` for `option`. */
function offered(option: SessionOption, choice: OptionChoice): boolean {
  switch (option.kind) {
    case "select":
      return (
        choice.kind === "select" &&
        option.values.some((value) => value.id === choice.value)
      );
    case "toggle":
      return choice.kind === "toggle";
    case "unknown":
      return false;
  }
}

/** Whether `choice` is what `option` already is. */
function already(option: SessionOption, choice: OptionChoice): boolean {
  switch (option.kind) {
    case "select":
      return choice.kind === "select" && option.current === choice.value;
    case "toggle":
      return choice.kind === "toggle" && option.on === choice.on;
    case "unknown":
      return false;
  }
}

/** `options` as they will be once `picks` are applied: what the page
 * shows before the session exists. A pick the agent does not offer (made
 * for another agent or profile) is left out. */
export function withPicks(
  options: readonly SessionOption[],
  picks: Picks,
): SessionOption[] {
  return options.map((option) => {
    const choice = picks[option.id];
    if (choice === undefined || !offered(option, choice)) {
      return option;
    }
    if (option.kind === "select" && choice.kind === "select") {
      return { ...option, current: choice.value };
    }
    if (option.kind === "toggle" && choice.kind === "toggle") {
      return { ...option, on: choice.on };
    }
    return option;
  });
}

/** The choices to start a session with: the picks the agent offers and
 * that change something, in the order the agent lists its options. */
export function choicesFor(
  options: readonly SessionOption[],
  picks: Picks,
): OptionChoice[] {
  return options.flatMap((option) => {
    const choice = picks[option.id];
    return choice !== undefined &&
      offered(option, choice) &&
      !already(option, choice)
      ? [choice]
      : [];
  });
}

/** The choices to keep for the machine's next start: every pick the
 * agent offers, whether or not it changes its value (a pick equal to the
 * agent's default today is still the person's choice, and is applied
 * when the default moves). With the options not known (the agent had
 * not said when Send went), the picks as they are: a start that could
 * not check them does not make the kept ones fewer. */
export function keptChoices(
  options: readonly SessionOption[] | null,
  picks: Picks,
): OptionChoice[] {
  if (options === null) {
    return Object.values(picks);
  }
  return options.flatMap((option) => {
    const choice = picks[option.id];
    return choice !== undefined && offered(option, choice) ? [choice] : [];
  });
}

/** `picks` with `choice` added (replacing one for the same option). */
export function pick(picks: Picks, choice: OptionChoice): Picks {
  return { ...picks, [choice.id]: choice };
}
