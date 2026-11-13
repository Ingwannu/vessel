// The raised box a prompt is written in (agent-orchestrator's composer,
// GOAT's for a running session): the text, then a row of controls on the
// left and Send at the right; a panel may be docked over its top edge (a
// permission prompt) and a tray hung under it (where the session runs).
//
// The send key is the person's (Settings › General, data/compose-keys.ts):
// Enter or Mod+Enter, Shift+Enter always a new line, never while an input
// method composes; the other key sends the other way while a turn runs.
// A key goes to `onKey` first (the suggestion menu's Up, Down, Enter,
// Escape), then Up and Down to `onArrow` (recalling sent prompts,
// data/recall.ts), else they move the caret. A click on the box's padding
// puts the caret in the text, as the reference's does.
//
// Files pasted or dropped on the box go to `onFiles` (milestone 11): the
// box takes agent-orchestrator's accent tint while something is dragged
// over it. A strip of attached files sits over the text, and a menu
// (the `/` and `@` list) over the box's top edge.

import "./prompt-box.css";

import {
  type DragEvent,
  type KeyboardEvent,
  type ReactNode,
  type RefObject,
  useRef,
  useState,
} from "react";

import { sends, sendsOther } from "../../data/compose-keys";
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
  /** Send; `other` when the other key (or ⌘/Ctrl with the click) asked
   * for the other way. */
  onSend: (other: boolean) => void;
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
  /** Over the text, inside the box: the attached files. */
  strip?: ReactNode;
  /** Over the box's top edge: the suggestion list. */
  menu?: ReactNode;
  /** A key, before the box reads it: true when it was used. */
  onKey?: (event: KeyboardEvent<HTMLTextAreaElement>) => boolean;
  /** The text or the caret moved (for the suggestion list). */
  onCaret?: (text: string, caret: number) => void;
  /** Files pasted or dropped on the box; absent where none are taken. */
  onFiles?: (files: File[]) => void;
}

/** The files a drag carries, when it carries files. */
function draggedFiles(event: DragEvent<HTMLElement>): boolean {
  return Array.from(event.dataTransfer.types).includes("Files");
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
  strip,
  menu,
  onKey,
  onCaret,
  onFiles,
}: PromptBoxProps) {
  const own = useRef<HTMLTextAreaElement>(null);
  const mode = useStore(prefs, (state) => state.values.composer.send);
  const text = textAreaRef ?? own;
  // Enter and leave fire for every child the drag crosses: a count, so
  // the tint does not flicker between them.
  const [dragging, setDragging] = useState(0);
  const caret = () => {
    const area = text.current;
    if (area !== null) {
      onCaret?.(area.value, area.selectionStart);
    }
  };
  const drop =
    onFiles === undefined
      ? {}
      : {
          onDragEnter: (event: DragEvent<HTMLFormElement>) => {
            if (draggedFiles(event)) {
              setDragging((count) => count + 1);
            }
          },
          onDragLeave: (event: DragEvent<HTMLFormElement>) => {
            if (draggedFiles(event)) {
              setDragging((count) => Math.max(0, count - 1));
            }
          },
          onDragOver: (event: DragEvent<HTMLFormElement>) => {
            if (draggedFiles(event)) {
              event.preventDefault();
            }
          },
          onDrop: (event: DragEvent<HTMLFormElement>) => {
            setDragging(0);
            const files = Array.from(event.dataTransfer.files);
            if (files.length > 0) {
              event.preventDefault();
              onFiles(files);
            }
          },
        };
  return (
    <div className={tray === undefined ? "prompt-dock" : "prompt-dock trayed"}>
      {menu === undefined ? null : <div className="prompt-menu">{menu}</div>}
      {above}
      <form
        className={dragging > 0 ? "prompt-box dropping" : "prompt-box"}
        aria-label={label}
        onSubmit={(event) => {
          event.preventDefault();
          onSend(false);
        }}
        onMouseDown={(event) => {
          if (event.target === event.currentTarget && event.button === 0) {
            event.preventDefault();
            text.current?.focus();
          }
        }}
        onPaste={(event) => {
          const files = Array.from(event.clipboardData.files);
          if (onFiles !== undefined && files.length > 0) {
            event.preventDefault();
            onFiles(files);
          }
        }}
        onKeyUp={caret}
        onMouseUp={caret}
        {...drop}
      >
        {strip}
        <TextArea
          look="bare"
          minRows={3}
          maxRows={7}
          aria-label={label}
          placeholder={placeholder}
          disabled={disabled}
          value={value}
          onChange={(next) => {
            onChange(next);
            // The caret is where the change left it once React has drawn.
            window.requestAnimationFrame(caret);
          }}
          textAreaRef={text}
          onKeyDown={(event) => {
            if (onKey?.(event) === true) {
              event.preventDefault();
              return;
            }
            const press = {
              ...event,
              isComposing: event.nativeEvent.isComposing,
            };
            if (sendsOther(press, mode)) {
              event.preventDefault();
              onSend(true);
              return;
            }
            if (sends(press, mode)) {
              event.preventDefault();
              onSend(false);
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
