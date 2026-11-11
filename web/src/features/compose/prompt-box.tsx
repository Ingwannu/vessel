// The raised box a prompt is written in (agent-orchestrator's composer,
// GOAT's for a running session): the text, then a row of controls on the
// left and Send at the right; a panel may be docked over its top edge (a
// permission prompt) and a tray hung under it (where the session runs).
//
// The send key is the person's (Settings › General, data/compose-keys.ts):
// Enter or Mod+Enter, Shift+Enter always a new line, never while an input
// method composes. Up and Down are offered to `onArrow` first (recalling
// sent prompts, data/recall.ts) and move the caret otherwise. A click on
// the box's padding puts the caret in the text, as the reference's does.

import "./prompt-box.css";

import {
  type KeyboardEvent,
  type ReactNode,
  type RefObject,
  useRef,
} from "react";

import { sends } from "../../data/compose-keys";
import { prefs } from "../../data/prefs";
import type { Caret } from "../../data/recall";
import { useStore } from "../../data/store";
import { TextArea } from "../../ui/kit";

export interface PromptBoxProps {
  value: string;
  onChange: (value: string) => void;
  /** What the box asks for; also what it says while disabled. */
  placeholder: string;
  /** The form's and the text's name. */
  label: string;
  disabled?: boolean;
  onSend: () => void;
  /** The foot row's left group: the add button and the agent's controls. */
  controls: ReactNode;
  /** The foot row's right end: Send or Stop. */
  action: ReactNode;
  /** Docked over the box's top edge. */
  above?: ReactNode;
  /** Hung under the box. */
  tray?: ReactNode;
  textAreaRef?: RefObject<HTMLTextAreaElement | null>;
  /** An arrow pressed with no modifier: true when it was used (a recalled
   * prompt shown), so the caret does not move. */
  onArrow?: (key: "up" | "down", caret: Caret) => boolean;
  /** A quiet line under the box (a draft too long to keep). */
  note?: ReactNode;
}

/** The arrow `event` is, when it is a plain one. */
function arrowOf(
  event: KeyboardEvent<HTMLTextAreaElement>,
): "up" | "down" | null {
  if (
    event.nativeEvent.isComposing ||
    event.shiftKey ||
    event.altKey ||
    event.metaKey ||
    event.ctrlKey
  ) {
    return null;
  }
  return event.key === "ArrowUp"
    ? "up"
    : event.key === "ArrowDown"
      ? "down"
      : null;
}

export function PromptBox({
  value,
  onChange,
  placeholder,
  label,
  disabled = false,
  onSend,
  controls,
  action,
  above,
  tray,
  textAreaRef,
  onArrow,
  note,
}: PromptBoxProps) {
  const own = useRef<HTMLTextAreaElement>(null);
  const mode = useStore(prefs, (state) => state.values.composer.send);
  const text = textAreaRef ?? own;
  return (
    <div className={tray === undefined ? "prompt-dock" : "prompt-dock trayed"}>
      {above}
      <form
        className="prompt-box"
        aria-label={label}
        onSubmit={(event) => {
          event.preventDefault();
          onSend();
        }}
        onMouseDown={(event) => {
          if (event.target === event.currentTarget && event.button === 0) {
            event.preventDefault();
            text.current?.focus();
          }
        }}
      >
        <TextArea
          look="bare"
          minRows={3}
          maxRows={7}
          aria-label={label}
          placeholder={placeholder}
          disabled={disabled}
          value={value}
          onChange={onChange}
          textAreaRef={text}
          onKeyDown={(event) => {
            if (
              sends(
                { ...event, isComposing: event.nativeEvent.isComposing },
                mode,
              )
            ) {
              event.preventDefault();
              onSend();
              return;
            }
            const arrow = arrowOf(event);
            const area = event.currentTarget;
            if (
              arrow !== null &&
              onArrow?.(arrow, {
                text: area.value,
                start: area.selectionStart,
                end: area.selectionEnd,
              }) === true
            ) {
              event.preventDefault();
            }
          }}
        />
        <div className="prompt-foot">
          <div
            className="prompt-tools"
            role="group"
            aria-label="Session settings"
          >
            {controls}
          </div>
          <div className="prompt-send">{action}</div>
        </div>
      </form>
      {tray === undefined ? null : <div className="prompt-tray">{tray}</div>}
      {note === undefined ? null : <p className="prompt-note">{note}</p>}
    </div>
  );
}
