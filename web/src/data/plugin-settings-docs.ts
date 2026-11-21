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
  followSave,
  readSettings,
  resetSettings,
  type SettingsAddress,
  writeSettings,
} from "./plugin-settings";
import { SURFACE_WORDS } from "./plugin-surfaces";
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

/** The last call issued per document, by `docKey` (node review R1): an
 * answer is applied only if no read, save or reset was issued after it, as
 * Paseo's query keeps only its latest fetch (use-settings.ts:25-30). */
const issued = new Map<string, number>();

/** A new call for `at`; every earlier read's answer is now stale. */
function issue(at: SettingsAddress): number {
  const key = docKey(at);
  const next = (issued.get(key) ?? 0) + 1;
  issued.set(key, next);
  return next;
}

/** Read `at` again; the last values stay until the answer, which is
 * dropped if a newer call was issued meanwhile. */
export async function loadDoc(
  fleet: Fleet,
  at: SettingsAddress,
): Promise<void> {
  const ticket = issue(at);
  const read = await readSettings(fleet, at);
  if (issued.get(docKey(at)) !== ticket) return;
  change(at, (entry) => ({
    ...entry,
    state: read.ok
      ? read.value
      : { status: "error", error: problemWords(read) },
  }));
}

/** What a write or reset over `from` answered, applied to the entry;
 * whether it saved. A save's answer is the newest state, so reads issued
 * before it are dropped, and the drafts that began from `from` follow it. */
function settle(
  at: SettingsAddress,
  key: string | null,
  from: SettingsRevision,
  answer: Outcome<PluginSettingsWrite>,
): boolean {
  issue(at);
  if (answer.ok && answer.value.status === "saved")
    followSave(at, from, answer.value.revision);
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
  issue(at);
  change(at, (entry) => ({ ...entry, saving: key, error: null }));
  return settle(
    at,
    key,
    revision,
    await writeSettings(fleet, at, revision, values),
  );
}

/** Replace `at` with its defaults over the revision last read. */
export async function resetDoc(
  fleet: Fleet,
  at: SettingsAddress,
): Promise<boolean> {
  const state = docEntry(settingsDocs.get(), at).state;
  if (state.status !== "ready" && state.status !== "invalid") return false;
  issue(at);
  change(at, (entry) => ({ ...entry, saving: "", error: null }));
  return settle(
    at,
    null,
    state.revision,
    await resetSettings(fleet, at, state.revision),
  );
}

/** A draft that began from another revision than the one on screen: the
 * page refuses it as the node would, without sending (node review R1). */
export function refuseStale(at: SettingsAddress, key: string): void {
  change(at, (entry) => ({
    ...entry,
    error: { key, words: SURFACE_WORDS.conflict, conflict: true },
  }));
}

/** Clear the refusal shown on `key`'s row (Escape, or the screen closing
 * when `key` is `null`: every refusal of the document). */
export function clearRefusal(at: SettingsAddress, key: string | null): void {
  const entry = docEntry(settingsDocs.get(), at);
  if (entry.error === null || (key !== null && entry.error.key !== key)) return;
  change(at, (current) => ({ ...current, error: null }));
}

/** Clear the last save's words and read again (Paseo's `reload()`). */
export async function reloadDoc(
  fleet: Fleet,
  at: SettingsAddress,
): Promise<void> {
  change(at, (entry) => ({ ...entry, error: null }));
  await loadDoc(fleet, at);
}
