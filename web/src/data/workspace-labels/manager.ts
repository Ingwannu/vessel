// The label manager (Paseo "Manage labels…"): one machine's catalog,
// searched, a label opened to rename, recolor or delete.
//
// [Decision Log]
// - Purpose: edit a machine's labels in one place, as Paseo's manager
//   modal does, from the palette and the view menu.
// - Reference: Paseo's manager model picks a host, filters by a search,
//   opens a draft seeded from the label, enables Save only when the draft
//   differs and its name is not empty, sends name and color as one update
//   and keeps the draft as typed on failure; delete inspects the count of
//   workspaces first and asks before deleting; one operation at a time.
// - Choice: the same rules in a model outside React. Machines stand where
//   Paseo has hosts.

import type { NodeKey } from "../../wire/bridge";
import type { Label, LabelColor } from "../../wire/proto";
import type { Outcome } from "../outcome";
import { failureText, Observed } from "./picker";
import { labelKey, normalizeLabelName } from "./names";

/** A machine the manager can edit. */
export interface ManagedMachine {
  node: NodeKey;
  name: string;
  state: "online" | "offline" | "unsupported";
  labels: readonly Label[];
  /** The last failure reading its catalog, if any. */
  error?: string | null;
}

/** The label being edited, as typed so far. */
export interface Draft {
  /** The catalog name it edits, fixed while the draft is open. */
  name: string;
  typed: string;
  color: LabelColor;
}

export interface ManagerSnapshot {
  machines: readonly ManagedMachine[];
  node: NodeKey | null;
  machine: ManagedMachine | null;
  query: string;
  /** The machine's labels the search keeps. */
  labels: readonly Label[];
  draft: Draft | null;
  /** The catalog label behind the draft. */
  editing: Label | null;
  /** Whether Save would change something. */
  dirty: boolean;
  pending: boolean;
  error: string | null;
}

export interface ManagerCalls {
  update: (
    node: NodeKey,
    name: string,
    change: { new_name?: string; color?: LabelColor },
  ) => Promise<Outcome<unknown>>;
  /** How many workspaces carry the label. */
  inspect: (node: NodeKey, name: string) => Promise<Outcome<number>>;
  remove: (node: NodeKey, name: string) => Promise<Outcome<unknown>>;
}

export class ManagerModel extends Observed<ManagerSnapshot> {
  readonly #calls: ManagerCalls;
  #machines: readonly ManagedMachine[] = [];
  #node: NodeKey | null = null;
  #query = "";
  #draft: Draft | null = null;
  #error: string | null = null;
  #running: Promise<void> | null = null;
  /** Whether the person chose the machine (then it stays chosen). */
  #picked = false;

  constructor(calls: ManagerCalls) {
    super(snapshotOf([], null, "", null, null, false));
    this.#calls = calls;
  }

  /** The machines and their catalogs now. Until the person picks one, the
   * manager shows `preferred` (the machine they last chose) when it
   * answers, else the first that answers with labels, else the first that
   * answers (a book often starts with one that is off); a machine that
   * went away is replaced the same way, unless an operation is under way. */
  syncMachines(
    machines: readonly ManagedMachine[],
    preferred: NodeKey | null = null,
  ): void {
    this.#machines = machines;
    const known = machines.some((machine) => machine.node === this.#node);
    if (this.#running === null && (!known || !this.#picked)) {
      const online = machines.filter((machine) => machine.state === "online");
      const next =
        (
          online.find((machine) => machine.node === preferred) ??
          online.find((machine) => machine.labels.length > 0) ??
          online[0] ??
          machines[0]
        )?.node ?? null;
      if (next !== this.#node) {
        this.#node = next;
        this.#draft = null;
      }
    }
    this.#commit();
  }

  selectMachine(node: NodeKey): void {
    if (this.#running !== null || node === this.#node) {
      return;
    }
    this.#picked = true;
    this.#node = node;
    this.#draft = null;
    this.#error = null;
    this.#commit();
  }

  setQuery(query: string): void {
    this.#query = query;
    this.#commit();
  }

  edit(label: Label): void {
    if (this.#running !== null) {
      return;
    }
    this.#draft = { name: label.name, typed: label.name, color: label.color };
    this.#error = null;
    this.#commit();
  }

  cancelEdit(): void {
    if (this.#running !== null) {
      return;
    }
    this.#draft = null;
    this.#error = null;
    this.#commit();
  }

  setName(typed: string): void {
    if (this.#draft !== null) {
      this.#draft = { ...this.#draft, typed };
      this.#commit();
    }
  }

  setColor(color: LabelColor): void {
    if (this.#draft !== null) {
      this.#draft = { ...this.#draft, color };
      this.#commit();
    }
  }

  /** Send the draft as one update; it closes only once that landed. */
  save(): Promise<void> {
    const { draft, editing, dirty, node } = this.snapshot();
    if (draft === null || editing === null || !dirty || node === null) {
      return Promise.resolve();
    }
    const typed = normalizeLabelName(draft.typed);
    return this.#run(async () => {
      const outcome = await this.#calls.update(node, editing.name, {
        ...(typed === editing.name ? {} : { new_name: typed }),
        ...(draft.color === editing.color ? {} : { color: draft.color }),
      });
      this.#settle(outcome);
    });
  }

  /** Count the workspaces carrying the edited label, ask `confirm` with
   * that count, then delete it. */
  delete(confirm: (affected: number) => Promise<boolean>): Promise<void> {
    const { editing, node } = this.snapshot();
    if (editing === null || node === null) {
      return Promise.resolve();
    }
    return this.#run(async () => {
      const inspected = await this.#calls.inspect(node, editing.name);
      if (!inspected.ok) {
        this.#error = failureText(inspected);
        return;
      }
      if (!(await confirm(inspected.value))) {
        return;
      }
      this.#settle(await this.#calls.remove(node, editing.name));
    });
  }

  #settle(outcome: Outcome<unknown>): void {
    this.#error = failureText(outcome);
    if (outcome.ok) {
      this.#draft = null;
    }
  }

  #run(action: () => Promise<void>): Promise<void> {
    if (this.#running !== null) {
      return this.#running;
    }
    this.#error = null;
    this.#running = action().finally(() => {
      this.#running = null;
      this.#commit();
    });
    this.#commit();
    return this.#running;
  }

  #commit(): void {
    this.publish(
      snapshotOf(
        this.#machines,
        this.#node,
        this.#query,
        this.#draft,
        this.#error,
        this.#running !== null,
      ),
    );
  }
}

function snapshotOf(
  machines: readonly ManagedMachine[],
  node: NodeKey | null,
  query: string,
  draft: Draft | null,
  error: string | null,
  pending: boolean,
): ManagerSnapshot {
  const machine = machines.find((candidate) => candidate.node === node) ?? null;
  const search = query.trim().toLowerCase();
  const labels = (machine?.labels ?? []).filter((label) =>
    label.name.toLowerCase().includes(search),
  );
  const editing =
    draft === null
      ? null
      : (machine?.labels.find(
          (label) => labelKey(label.name) === labelKey(draft.name),
        ) ?? null);
  const typed = draft === null ? "" : normalizeLabelName(draft.typed);
  const dirty =
    draft !== null &&
    editing !== null &&
    typed.length > 0 &&
    (typed !== editing.name || draft.color !== editing.color);
  return {
    machines,
    node,
    machine,
    query,
    labels,
    draft,
    editing,
    dirty,
    pending,
    error,
  };
}
