// One plugin settings document's state above its rows (ADR 0115,
// "Surfaces"): Paseo's `useSettings` states (plugins/settings/
// use-settings.ts:73-84) and its `reload()` after a conflict.
//
// [Decision Log]
// - Loading, a failed read with Retry, invalid stored data with Reset to
//   defaults (Paseo's `reset()` recovers it), and a conflict with Reload.
//   Reload moves the document's drafts to the revision it read, so the
//   node's words "Reload before saving again" hold. A refusal of one value
//   shows on its own row instead (settings-rows.tsx).

import { useFleet } from "../../../context";
import {
  rebaseDrafts,
  type SettingsAddress,
} from "../../../data/plugin-settings";
import {
  type DocEntry,
  docEntry,
  reloadDoc,
  resetDoc,
  settingsDocs,
} from "../../../data/plugin-settings-docs";
import { SURFACE_WORDS } from "../../../data/plugin-surfaces";
import { useStore } from "../../../data/store";
import { Alert, Button } from "../../../ui/kit";
import type { SurfaceScope } from "./scope";

function addressOf(scope: SurfaceScope, settings: string): SettingsAddress {
  return {
    node: scope.node,
    plugin: scope.installation.address.plugin,
    settings,
  };
}

function useEntry(at: SettingsAddress): DocEntry {
  return useStore(settingsDocs, (all) => docEntry(all, at));
}

/** One document's state, above its rows. */
export function DocStatus({
  scope,
  settings,
}: {
  scope: SurfaceScope;
  settings: string;
}) {
  const fleet = useFleet();
  const at = addressOf(scope, settings);
  const entry = useEntry(at);
  const state = entry.state;
  if (state.status === "loading")
    return <p className="plugin-doc-note">{SURFACE_WORDS.loading}</p>;
  if (state.status === "error")
    return (
      <Alert variant="error" size="sm" description={state.error}>
        <Button
          size="sm"
          variant="outline"
          onClick={() => void reloadDoc(fleet, at)}
        >
          {SURFACE_WORDS.retry}
        </Button>
      </Alert>
    );
  if (state.status === "invalid")
    return (
      // A failed reset says why in place of the stored data's words
      // (node review note 2).
      <Alert
        variant="error"
        size="sm"
        description={entry.error?.words ?? state.error}
      >
        <Button
          size="sm"
          variant="outline"
          disabled={entry.saving !== null}
          onClick={() => void resetDoc(fleet, at)}
        >
          {SURFACE_WORDS.reset}
        </Button>
      </Alert>
    );
  const error = entry.error;
  if (error === null || (!error.conflict && error.key !== null)) return null;
  return (
    <Alert
      variant={error.conflict ? "warning" : "error"}
      size="sm"
      description={error.words}
    >
      {error.conflict ? (
        <Button
          size="sm"
          variant="outline"
          onClick={() =>
            void reloadDoc(fleet, at).then(() => {
              const now = docEntry(settingsDocs.get(), at).state;
              if (now.status === "ready") rebaseDrafts(at, now.revision);
            })
          }
        >
          {SURFACE_WORDS.reload}
        </Button>
      ) : null}
    </Alert>
  );
}
