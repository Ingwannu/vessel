// Recording a key for a shortcut (agent-orchestrator's recorder, in the
// row): "Press shortcut · Esc to cancel". The first valid chord is taken
// at once; Escape or leaving the window cancels; Tab moves on without
// saving; a modifier alone waits. While it records, no shortcut runs.

import { useEffect } from "react";

import { type Chord, chordOf } from "../../data/shortcuts/keys";
import { currentPlatform } from "../../data/shortcuts/platform";
import { Button } from "../../ui/kit";
import { pageOwnsFocus } from "../../ui/page-focus";
import { recording } from "../shortcuts/handlers";

export function ShortcutRecorder({
  label,
  error,
  onChord,
  onCancel,
}: {
  /** The action's name, for the recorder's own. */
  label: string;
  /** Why the last key was refused, shown in place. */
  error: string | null;
  onChord: (chord: Chord) => void;
  onCancel: () => void;
}) {
  useEffect(() => {
    recording.set(true);
    const platform = currentPlatform();
    const onKey = (event: KeyboardEvent) => {
      // A modal owns its keys even before its initial focus frame. Keep
      // the pending recording, but let Base UI handle Tab/Escape and IME.
      if (!pageOwnsFocus(document)) return;
      if (event.key === "Tab") {
        onCancel();
        return;
      }
      event.preventDefault();
      event.stopImmediatePropagation();
      if (event.key === "Escape") {
        onCancel();
        return;
      }
      if (event.repeat || event.isComposing) {
        return;
      }
      const chord = chordOf(event, platform);
      if (chord !== null) {
        onChord(chord);
      }
    };
    const onBlur = () => {
      onCancel();
    };
    window.addEventListener("keydown", onKey, { capture: true });
    window.addEventListener("blur", onBlur);
    return () => {
      recording.set(false);
      window.removeEventListener("keydown", onKey, { capture: true });
      window.removeEventListener("blur", onBlur);
    };
  }, [onChord, onCancel]);
  return (
    <div
      className="shortcut-recorder"
      role="group"
      aria-label={`Record shortcut for ${label}`}
      aria-live="polite"
    >
      <span className="shortcut-recorder-prompt">
        Press shortcut · Esc to cancel
      </span>
      <span
        className={
          error === null
            ? "shortcut-recorder-help"
            : "shortcut-recorder-help error"
        }
      >
        {error ??
          "Use a modifier with a key. Escape cancels; Tab moves on without saving."}
      </span>
      <Button size="sm" variant="ghost" onClick={onCancel}>
        Cancel
      </Button>
    </div>
  );
}
