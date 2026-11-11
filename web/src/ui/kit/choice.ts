// The one shape every list of choices in the kit takes (Select, a menu's
// radio group, Segmented, Tabs): a closed set of string values, each with
// what a person reads.
//
// Why values are `V extends string` and controls look the chosen option
// up by value instead of casting what the library hands back: Base UI
// reports a choice as the value it was given, typed loosely in places; a
// lookup keeps the caller's union exact without an `as`.

import type { IconName } from "./icon";

export interface Choice<V extends string> {
  value: V;
  /** What the option is called, sentence case. */
  label: string;
  /** A muted second part: a model behind a route, a node's state. */
  detail?: string;
  icon?: IconName;
  disabled?: boolean;
  /** A heading over a run of consecutive options with the same group. */
  group?: string;
}

/** The option whose value is `value`, if any. */
export function choiceOf<V extends string>(
  options: readonly Choice<V>[],
  value: unknown,
): Choice<V> | undefined {
  return options.find((option) => option.value === value);
}

/** `options` cut into runs that share a `group` (in order; a group that
 * reappears later starts a new run, as the reference's menus do). */
export function runs<V extends string>(
  options: readonly Choice<V>[],
): { group: string | undefined; options: Choice<V>[] }[] {
  const out: { group: string | undefined; options: Choice<V>[] }[] = [];
  for (const option of options) {
    const last = out.at(-1);
    if (last !== undefined && last.group === option.group) {
      last.options.push(option);
    } else {
      out.push({ group: option.group, options: [option] });
    }
  }
  return out;
}
