// Settings rows bound to a plugin's document (ADR 0115, "Surfaces"):
// Paseo's `SettingsSwitch`, `SettingsSelect` and `SettingsInput`
// (packages/app/src/components/settings/index.tsx:98-198) with
// `useSettings`' states (packages/app/src/plugins/settings/use-settings.ts).
//
// [Decision Log]
// - Purpose: a settings screen edits one property per row and must say
//   when the document is loading, unreadable, refused or changed elsewhere.
// - Choice (coordinator ruling on P3b, Q2): a switch or select saves at
//   once at the revision last read. An input keeps a draft and the revision
//   it began from; Enter saves, Escape discards. A conflict keeps the
//   draft. Reload reads again and moves the document's drafts to the new
//   revision, so "Reload before saving again" (the node's words, Paseo's)
//   means what it says.
// - One status line per document, above the rows: loading, invalid with
//   Reset to defaults, a failed read with Retry, a conflict with Reload.
//   A refused value is shown on the row that sent it.

import { useFleet } from "../../../context";
import {
  boolAt,
  draftKey,
  dropDraft,
  keepDraft,
  rebaseDrafts,
  type SettingsAddress,
  settingsDrafts,
  textAt,
  withValue,
  WRONG_TYPE,
} from "../../../data/plugin-settings";
import {
  type DocEntry,
  docEntry,
  reloadDoc,
  resetDoc,
  saveDoc,
  settingsDocs,
} from "../../../data/plugin-settings-docs";
import { SURFACE_WORDS } from "../../../data/plugin-surfaces";
import { useStore } from "../../../data/store";
import type { ControlContent } from "../../../wire/proto";
import {
  Alert,
  Button,
  Field,
  Select,
  Switch,
  TextField,
} from "../../../ui/kit";
import type { SurfaceScope } from "./scope";

type Bound<K extends ControlContent["kind"]> = Extract<
  ControlContent,
  { kind: K }
>;

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

/** The refusal to show on the row bound to `key`, if any. */
function rowError(entry: DocEntry, key: string): string | null {
  const error = entry.error;
  return error !== null && !error.conflict && error.key === key
    ? error.words
    : null;
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
      <Alert variant="error" size="sm" description={state.error}>
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

/** A switch bound to a boolean property; a change saves at once. */
export function SettingsSwitchRow({
  scope,
  control,
}: {
  scope: SurfaceScope;
  control: Bound<"settings_switch">;
}) {
  const fleet = useFleet();
  const at = addressOf(scope, control.settings);
  const entry = useEntry(at);
  const state = entry.state;
  const bound =
    state.status === "ready" ? boolAt(state.values, control.key) : null;
  const usable = bound?.ok === true && scope.mayInvoke && entry.saving === null;
  return (
    <Field
      label={control.label}
      hint={control.hint ?? undefined}
      error={bound?.ok === false ? WRONG_TYPE : rowError(entry, control.key)}
    >
      <Switch
        label={control.label}
        checked={bound?.ok === true && bound.value}
        disabled={!usable}
        onChange={(next) => {
          if (state.status !== "ready") return;
          void saveDoc(
            fleet,
            at,
            control.key,
            state.revision,
            withValue(state.values, control.key, next),
          );
        }}
      />
    </Field>
  );
}

/** A select bound to a string property; a choice saves at once. */
export function SettingsSelectRow({
  scope,
  control,
}: {
  scope: SurfaceScope;
  control: Bound<"settings_select">;
}) {
  const fleet = useFleet();
  const at = addressOf(scope, control.settings);
  const entry = useEntry(at);
  const state = entry.state;
  const bound =
    state.status === "ready" ? textAt(state.values, control.key) : null;
  const options = control.options.map((option) => ({
    value: option.id,
    label: option.title,
  }));
  const value =
    bound?.ok === true && options.some((option) => option.value === bound.value)
      ? bound.value
      : null;
  return (
    <Field
      label={control.label}
      hint={control.hint ?? undefined}
      error={bound?.ok === false ? WRONG_TYPE : rowError(entry, control.key)}
    >
      <Select
        aria-label={control.label}
        value={value}
        options={options}
        disabled={
          bound?.ok !== true || !scope.mayInvoke || entry.saving !== null
        }
        onChange={(next: string) => {
          if (state.status !== "ready") return;
          void saveDoc(
            fleet,
            at,
            control.key,
            state.revision,
            withValue(state.values, control.key, next),
          );
        }}
      />
    </Field>
  );
}

/** An input bound to a string property: a draft until Enter. */
export function SettingsInputRow({
  scope,
  control,
}: {
  scope: SurfaceScope;
  control: Bound<"settings_input">;
}) {
  const fleet = useFleet();
  const at = addressOf(scope, control.settings);
  const entry = useEntry(at);
  const id = draftKey(at, control.key);
  const draft = useStore(settingsDrafts, (all) => all.get(id) ?? null);
  const state = entry.state;
  const bound =
    state.status === "ready" ? textAt(state.values, control.key) : null;
  const usable = bound?.ok === true && scope.mayInvoke && entry.saving === null;
  const shown = draft?.text ?? (bound?.ok === true ? bound.value : "");
  return (
    <Field
      label={control.label}
      hint={control.hint ?? undefined}
      error={bound?.ok === false ? WRONG_TYPE : rowError(entry, control.key)}
    >
      <TextField
        aria-label={control.label}
        value={shown}
        {...(control.placeholder === null
          ? {}
          : { placeholder: control.placeholder })}
        type={control.mode === "secret" ? "password" : "text"}
        disabled={!usable}
        onChange={(text) => {
          if (state.status !== "ready") return;
          keepDraft(id, { text, revision: draft?.revision ?? state.revision });
        }}
        onKeyDown={(event) => {
          if (event.key === "Escape" && draft !== null) {
            event.preventDefault();
            dropDraft(id);
            return;
          }
          if (
            event.key !== "Enter" ||
            draft === null ||
            state.status !== "ready"
          )
            return;
          event.preventDefault();
          void saveDoc(
            fleet,
            at,
            control.key,
            draft.revision,
            withValue(state.values, control.key, draft.text),
          ).then((saved) => {
            if (saved) dropDraft(id);
          });
        }}
      />
    </Field>
  );
}
