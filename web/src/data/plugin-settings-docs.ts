// The settings documents a page shows, as `useSettings` keeps them in
// Paseo (packages/app/src/plugins/settings/use-settings.ts:16-85): read
// state, the save in flight and the last save's error, per document.
//
// [Decision Log]
// - Purpose: several rows of one screen bind the same document; they must
//   share one read, one revision and one save at a time, and the screen
//   must re-read when the node says a document changed.
// - Choice: a store of entries keyed by machine, plugin and document. A
//   screen asks for the documents it binds whenever the machine's
//   `plugin.watch` generation moves (ADR 0115, "Settings": the watch is
//   the change notice). A re-read keeps the last values on screen until the
//   answer arrives, so a save elsewhere does not flash "Loading".
// - States: `loading` (nothing read yet: no default is drawn as if saved),
//   `ready`, `invalid` (the node's bounded words; Reset recovers) and
//   `error` (the read failed; Retry). A save that fails keeps its words and
//   the key of the row that sent it, so the row shows them; a conflict is
//   marked so the screen offers Reload, as Paseo's `saveError` and
//   `reload()` do.

import type {
  PluginSettingsWrite,
  RawJson,
  SettingsRevision,
} from "../wire/proto";
import type { Fleet } from "./fleet";
import type { Outcome } from "./outcome";
import {
  readSettings,
  resetSettings,
  type SettingsAddress,
  writeSettings,
} from "./plugin-settings";
import { problemWords } from "./plugins";
import { Store } from "./store";

/** What a document reads as. */
export type DocState =
  | { status: "loading" }
  | { status: "ready"; revision: SettingsRevision; values: RawJson }
  | { status: "invalid"; revision: SettingsRevision; error: string }
  | { status: "error"; error: string };

/** The last save's refusal: the row that sent it (`null` for a reset). */
export interface SaveError {
  key: string | null;
  words: string;
  conflict: boolean;
}

/** One document's entry. */
export interface DocEntry {
  state: DocState;
  /** The key being saved (`""` for a reset), or `null`. */
  saving: string | null;
  error: SaveError | null;
}

const LOADING: DocEntry = {
  state: { status: "loading" },
  saving: null,
  error: null,
};

/** Every shown document, by `docKey`. */
export const settingsDocs = new Store<ReadonlyMap<string, DocEntry>>(new Map());

/** The key of the document at `at`. */
export function docKey(at: SettingsAddress): string {
  return JSON.stringify([at.node, at.plugin, at.settings]);
}

/** The entry of `at`, `loading` before its first read. */
export function docEntry(
  all: ReadonlyMap<string, DocEntry>,
  at: SettingsAddress,
): DocEntry {
  return all.get(docKey(at)) ?? LOADING;
}

function change(
  at: SettingsAddress,
  edit: (entry: DocEntry) => DocEntry,
): void {
  settingsDocs.update((all) =>
    new Map(all).set(docKey(at), edit(docEntry(all, at))),
  );
}

/** Read `at` again; the last values stay until the answer. */
export async function loadDoc(
  fleet: Fleet,
  at: SettingsAddress,
): Promise<void> {
  const read = await readSettings(fleet, at);
  change(at, (entry) => ({
    ...entry,
    state: read.ok
      ? read.value
      : { status: "error", error: problemWords(read) },
  }));
}

/** What a write or reset answered, applied to the entry; whether it saved. */
function settle(
  at: SettingsAddress,
  key: string | null,
  answer: Outcome<PluginSettingsWrite>,
): boolean {
  change(at, (entry) => {
    const done = { ...entry, saving: null };
    if (!answer.ok)
      return {
        ...done,
        error: { key, words: problemWords(answer), conflict: false },
      };
    const value = answer.value;
    switch (value.status) {
      case "saved":
        return {
          state: {
            status: "ready",
            revision: value.revision,
            values: value.values,
          },
          saving: null,
          error: null,
        };
      case "conflict":
        return { ...done, error: { key, words: value.error, conflict: true } };
      case "invalid":
        return { ...done, error: { key, words: value.error, conflict: false } };
    }
  });
  return answer.ok && answer.value.status === "saved";
}

/** Save `values` over `revision`, sent by the row bound to `key`. */
export async function saveDoc(
  fleet: Fleet,
  at: SettingsAddress,
  key: string,
  revision: SettingsRevision,
  values: RawJson,
): Promise<boolean> {
  change(at, (entry) => ({ ...entry, saving: key, error: null }));
  return settle(at, key, await writeSettings(fleet, at, revision, values));
}

/** Replace `at` with its defaults over the revision last read. */
export async function resetDoc(
  fleet: Fleet,
  at: SettingsAddress,
): Promise<boolean> {
  const state = docEntry(settingsDocs.get(), at).state;
  if (state.status !== "ready" && state.status !== "invalid") return false;
  change(at, (entry) => ({ ...entry, saving: "", error: null }));
  return settle(at, null, await resetSettings(fleet, at, state.revision));
}

/** Clear the last save's words and read again (Paseo's `reload()`). */
export async function reloadDoc(
  fleet: Fleet,
  at: SettingsAddress,
): Promise<void> {
  change(at, (entry) => ({ ...entry, error: null }));
  await loadDoc(fleet, at);
}
