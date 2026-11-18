// Settings › Shortcuts (agent-orchestrator's "Keyboard shortcuts"): a
// search over names and keys, "All commands" or "Modified", Reset all;
// the actions by group, each rebound by pressing the keys. A key another
// action holds is offered to be moved ("Shortcut already in use"); a
// fixed one, or one the browser keeps, is refused in place. Every change
// is kept by the bridge (ADR 0028) and every tab follows it.

import "./shortcuts-section.css";

import { useCallback, useMemo, useState } from "react";

import { useFleet } from "../../context";
import { prefer, prefs } from "../../data/prefs";
import {
  assign,
  clashOf,
  clashText,
  refusal,
  removeKey,
  restore,
  type ShortcutEdit,
} from "../../data/shortcuts/edit";
import { bindingWords, type Chord } from "../../data/shortcuts/keys";
import {
  modifiedCount,
  type ShortcutFilter,
  shortcutRows,
} from "../../data/shortcuts/listing";
import { currentPlatform } from "../../data/shortcuts/platform";
import {
  definition,
  SHORTCUT_GROUPS,
  type ShortcutAction,
} from "../../data/shortcuts/registry";
import { useStore } from "../../data/store";
import { Button, Dialog, Segmented, TextField } from "../../ui/kit";
import { toast } from "../../ui/toasts";
import { KeptNote } from "./kept-note";
import { SettingsPage } from "./settings-page";
import { ShortcutRecorder } from "./shortcut-recorder";
import { ShortcutRowView } from "./shortcut-row";

/** A row recording: which action, and which of its keys it replaces
 * (`null`: another key). */
interface Recording {
  action: ShortcutAction;
  replace: number | null;
}

/** A recorded key that another action holds, waiting for "Reassign". */
interface Reassign {
  action: ShortcutAction;
  chord: Chord;
  replace: number | null;
  from: ShortcutAction;
}

export function ShortcutsSection() {
  const fleet = useFleet();
  const platform = currentPlatform();
  const overrides = useStore(prefs, (state) => state.values.shortcuts);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<ShortcutFilter>("all");
  const [recording, setRecording] = useState<Recording | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [reassign, setReassign] = useState<Reassign | null>(null);
  const [resetting, setResetting] = useState(false);
  const changed = modifiedCount(overrides);

  const save = useCallback(
    (edits: ShortcutEdit[], said: string) => {
      void prefer(fleet, { set: "shortcuts", value: { edits } }).then(
        (outcome) => {
          toast(
            outcome.ok
              ? said
              : "Could not save. Your previous shortcuts are still active.",
            outcome.ok ? "success" : "error",
          );
        },
      );
    },
    [fleet],
  );
  const cancel = useCallback(() => {
    setRecording(null);
    setError(null);
  }, []);
  const recorded = useCallback(
    (chord: Chord) => {
      if (recording === null) {
        return;
      }
      const refused = refusal(chord, platform);
      if (refused !== null) {
        setError(refused);
        return;
      }
      const words = bindingWords([chord], platform);
      const { action, replace } = recording;
      const clash = clashOf(action, [chord], overrides, platform);
      if (clash !== null && clash.kind !== "taken") {
        setError(clashText(clash, words, action));
        return;
      }
      setRecording(null);
      setError(null);
      if (clash !== null) {
        setReassign({ action, chord, replace, from: clash.action });
        return;
      }
      save(
        assign(action, [chord], overrides, replace, null, platform),
        `Shortcut updated: ${definition(action).label} → ${words}`,
      );
    },
    [recording, platform, overrides, save],
  );

  const groups = useMemo(
    () =>
      SHORTCUT_GROUPS.map((group) => ({
        group,
        rows: shortcutRows(group, overrides, platform, query, filter),
      })).filter((entry) => entry.rows.length > 0),
    [overrides, platform, query, filter],
  );
  return (
    <SettingsPage lead="Find your shortcuts, then choose Edit to record a new key combination.">
      <KeptNote />
      <div className="shortcuts-body">
        <div className="shortcuts-toolbar">
          <TextField
            size="sm"
            icon="search"
            aria-label="Search commands or key combinations"
            placeholder="Search commands or key combinations"
            value={query}
            onChange={setQuery}
          />
          <Segmented
            label="Filter shortcuts"
            size="sm"
            value={filter}
            onChange={setFilter}
            options={[
              { value: "all", label: "All commands" },
              { value: "modified", label: `Modified ${String(changed)}` },
            ]}
          />
          <Button
            size="sm"
            variant="ghost"
            disabled={changed === 0}
            onClick={() => {
              setResetting(true);
            }}
          >
            Reset all
          </Button>
        </div>
        {groups.length === 0 ? (
          <p className="shortcuts-empty">
            {filter === "modified" && query === ""
              ? "No custom shortcuts yet."
              : "No matching commands."}
          </p>
        ) : (
          groups.map(({ group, rows }) => (
            <section key={group} className="shortcuts-group" aria-label={group}>
              <h2 className="shortcuts-group-head">{group}</h2>
              <ul className="shortcuts-list">
                {rows.map((row) => (
                  <ShortcutRowView
                    key={row.action}
                    row={row}
                    recorder={
                      recording?.action === row.action ? (
                        <ShortcutRecorder
                          label={row.label}
                          error={error}
                          onChord={recorded}
                          onCancel={cancel}
                        />
                      ) : null
                    }
                    onEdit={() => {
                      setError(null);
                      setRecording({
                        action: row.action,
                        replace: row.bindings.length > 0 ? 0 : null,
                      });
                    }}
                    onAdd={() => {
                      setError(null);
                      setRecording({ action: row.action, replace: null });
                    }}
                    onRemove={(index) => {
                      save(
                        removeKey(row.action, index, overrides),
                        `${row.label} no longer uses ${row.words[index] ?? "that key"}.`,
                      );
                    }}
                    onRestore={() => {
                      save(
                        restore(row.action),
                        `${row.label} now uses its original keys.`,
                      );
                    }}
                  />
                ))}
              </ul>
            </section>
          ))
        )}
      </div>
      {reassign === null ? null : (
        <Dialog
          title="Shortcut already in use"
          onClose={() => {
            setReassign(null);
          }}
          footer={
            <>
              <Button
                variant="ghost"
                onClick={() => {
                  setReassign(null);
                }}
              >
                Cancel
              </Button>
              <Button
                variant="primary"
                onClick={() => {
                  const { action, chord, replace, from } = reassign;
                  setReassign(null);
                  save(
                    assign(action, [chord], overrides, replace, from, platform),
                    `${bindingWords([chord], platform)} moved from ${definition(from).label} to ${definition(action).label}.`,
                  );
                }}
              >
                Reassign
              </Button>
            </>
          }
        >
          <p>
            {clashText(
              { kind: "taken", action: reassign.from },
              bindingWords([reassign.chord], platform),
              reassign.action,
            )}
          </p>
        </Dialog>
      )}
      {resetting ? (
        <Dialog
          title="Reset all"
          onClose={() => {
            setResetting(false);
          }}
          footer={
            <>
              <Button
                variant="ghost"
                onClick={() => {
                  setResetting(false);
                }}
              >
                Cancel
              </Button>
              <Button
                variant="danger"
                onClick={() => {
                  setResetting(false);
                  void prefer(fleet, { set: "reset_shortcuts" }).then(
                    (outcome) => {
                      toast(
                        outcome.ok
                          ? "Keyboard shortcuts restored to their original keys."
                          : "Could not reset shortcuts. Try again.",
                        outcome.ok ? "success" : "error",
                      );
                    },
                  );
                }}
              >
                Reset all
              </Button>
            </>
          }
        >
          <p>Restore every shortcut to its original keys?</p>
        </Dialog>
      ) : null}
    </SettingsPage>
  );
}
