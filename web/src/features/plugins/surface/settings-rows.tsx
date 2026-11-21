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
// - Each row is the page's settings row (the kit Row, as General draws
//   its settings): label and hint on the left, the control on the right,
//   as Paseo's `SettingsRow`. A refused value shows on the row that sent
//   it; the document's state is doc-status.tsx's line above the rows.

import type { ReactNode } from "react";

import { useFleet } from "../../../context";
import {
  boolAt,
  draftKey,
  dropDraft,
  keepDraft,
  type SettingsAddress,
  settingsDrafts,
  textAt,
  withValue,
  WRONG_TYPE,
} from "../../../data/plugin-settings";
import {
  type DocEntry,
  docEntry,
  clearRefusal,
  refuseStale,
  saveDoc,
  settingsDocs,
} from "../../../data/plugin-settings-docs";
import { useStore } from "../../../data/store";
import type { ControlContent } from "../../../wire/proto";
import { Row, Select, Switch, TextField } from "../../../ui/kit";
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

/** Paseo's `SettingsRow` (components/settings/index.tsx:59-96) as the
 * page draws a setting: the kit Row, its label and hint (or the refusal,
 * in the danger colour) on the left and the control on the right. */
function SettingRow({
  label,
  hint,
  error,
  control,
}: {
  label: string;
  hint: string | null;
  error: string | null;
  control: ReactNode;
}) {
  const description =
    error === null ? (
      hint
    ) : (
      <span className="plugin-row-error" role="alert">
        {error}
      </span>
    );
  return (
    <Row
      title={label}
      {...(description === null ? {} : { detail: { description } })}
      control={control}
    />
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
    <SettingRow
      label={control.label}
      hint={control.hint}
      error={bound?.ok === false ? WRONG_TYPE : rowError(entry, control.key)}
      control={
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
      }
    />
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
  // A stored value no option names is shown as itself, as Paseo's select
  // shows the value it was given (node review note 4).
  const value = bound?.ok === true ? bound.value : null;
  if (value !== null && !options.some((option) => option.value === value))
    options.push({ value, label: value });
  return (
    <SettingRow
      label={control.label}
      hint={control.hint}
      error={bound?.ok === false ? WRONG_TYPE : rowError(entry, control.key)}
      control={
        <Select
          aria-label={control.label}
          size="sm"
          width="fit"
          align="end"
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
      }
    />
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
    <SettingRow
      label={control.label}
      hint={control.hint}
      error={bound?.ok === false ? WRONG_TYPE : rowError(entry, control.key)}
      control={
        <span className="plugin-row-input">
          <TextField
            aria-label={control.label}
            size="sm"
            value={shown}
            {...(control.placeholder === null
              ? {}
              : { placeholder: control.placeholder })}
            type={control.mode === "secret" ? "password" : "text"}
            disabled={!usable}
            onChange={(text) => {
              if (state.status !== "ready") return;
              keepDraft(id, {
                text,
                revision: draft?.revision ?? state.revision,
              });
            }}
            onKeyDown={(event) => {
              if (event.key === "Escape" && draft !== null) {
                event.preventDefault();
                dropDraft(id);
                clearRefusal(at, control.key);
                return;
              }
              if (
                event.key !== "Enter" ||
                draft === null ||
                state.status !== "ready"
              )
                return;
              event.preventDefault();
              // A draft from another revision than the values on screen
              // would save them over its own base: refused here, as the
              // node refuses a stale revision (node review R1).
              if (draft.revision !== state.revision) {
                refuseStale(at, control.key);
                return;
              }
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
        </span>
      }
    />
  );
}
