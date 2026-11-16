// The row menu's Labels picker: every label of the workspace's machine,
// the ones it carries ticked, a press toggling one.
//
// [Decision Log]
// - Purpose: assign and unassign labels on one workspace from its row.
// - Reference: Paseo's picker is multi-select (a press toggles and the
//   menu stays open) and allows one mutation in flight per label, so a
//   double press cannot send "assign" and "unassign" for the same label
//   at once; a failure says why under the list.
// - Choice: the same rules in a small model outside React, so they are
//   tested without drawing; the menu subscribes to its snapshot.

import type { Label } from "../../wire/proto";
import type { Outcome } from "../outcome";
import { labelKey } from "./names";

/** A model a component subscribes to (`useSyncExternalStore`). */
export class Observed<T> {
  readonly #listeners = new Set<() => void>();
  #snapshot: T;

  constructor(initial: T) {
    this.#snapshot = initial;
  }

  readonly snapshot = (): T => this.#snapshot;

  readonly subscribe = (listener: () => void): (() => void) => {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  };

  protected publish(snapshot: T): void {
    this.#snapshot = snapshot;
    for (const listener of this.#listeners) {
      listener();
    }
  }
}

/** One label as the picker lists it. */
export interface PickerRow {
  name: string;
  color: Label["color"];
  assigned: boolean;
}

export interface PickerSnapshot {
  rows: readonly PickerRow[];
  /** Why the last press failed, in the page's words. */
  error: string | null;
  /** Whether the machine answers; offline, every row is disabled. */
  online: boolean;
  /** The keys of the labels with a press in flight. */
  pending: readonly string[];
}

/** What the page says when a label change failed with no reason given. */
export const UPDATE_FAILED = "Unable to update label";

/** Why `outcome` failed, for the line under a label control. */
export function failureText(outcome: Outcome<unknown>): string | null {
  if (outcome.ok) {
    return null;
  }
  const message = outcome.problem.message.trim();
  return message === "" ? UPDATE_FAILED : message;
}

/** The catalog's labels, each told whether `assigned` names it. */
export function pickerRows(
  labels: readonly Label[],
  assigned: readonly string[],
): PickerRow[] {
  const keys = new Set(assigned.map(labelKey));
  return labels.map((label) => ({
    name: label.name,
    color: label.color,
    assigned: keys.has(labelKey(label.name)),
  }));
}

export class PickerModel extends Observed<PickerSnapshot> {
  readonly #mutate: (
    label: Label,
    assigned: boolean,
  ) => Promise<Outcome<unknown>>;
  #labels: readonly Label[] = [];
  #assigned: readonly string[] = [];
  #online = false;
  #error: string | null = null;
  readonly #pending = new Map<string, Promise<boolean>>();

  constructor(
    mutate: (label: Label, assigned: boolean) => Promise<Outcome<unknown>>,
  ) {
    super({ rows: [], error: null, online: false, pending: [] });
    this.#mutate = mutate;
  }

  /** What the machine says now: its catalog, the workspace's names and
   * whether it answers. */
  sync(input: {
    labels: readonly Label[];
    assigned: readonly string[];
    online: boolean;
  }): void {
    this.#labels = input.labels;
    this.#assigned = input.assigned;
    this.#online = input.online;
    this.#commit();
  }

  /** Assign `label` (`assigned`) or take it off; a second press on a
   * label with one in flight joins the first. `true` when it landed. */
  toggle(label: Label, assigned: boolean): Promise<boolean> {
    const key = labelKey(label.name);
    const running = this.#pending.get(key);
    if (running !== undefined) {
      return running;
    }
    this.#error = null;
    const press = this.#mutate(label, assigned).then((outcome) => {
      this.#error = failureText(outcome);
      this.#pending.delete(key);
      this.#commit();
      return outcome.ok;
    });
    this.#pending.set(key, press);
    this.#commit();
    return press;
  }

  #commit(): void {
    this.publish({
      rows: pickerRows(this.#labels, this.#assigned),
      error: this.#error,
      online: this.#online,
      pending: [...this.#pending.keys()],
    });
  }
}
