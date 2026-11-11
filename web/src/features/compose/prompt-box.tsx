// The raised box a prompt is written in (agent-orchestrator's composer,
// GOAT's for a running session): the text, then a row of controls on the
// left and Send at the right; a panel may be docked over its top edge (a
// permission prompt) and a tray hung under it (where the session runs).
//
// Enter sends and Shift+Enter starts a line, as in both references; ⌘ or
// Ctrl with Enter sends too, as milestone 9 did. A key that ends an input
// method's composition does not send. A click on the box's padding puts
// the caret in the text, as the reference's does.

import "./prompt-box.css";

import {
  type KeyboardEvent,
  type ReactNode,
  type RefObject,
  useRef,
} from "react";

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
}

/** Whether `event` asks to send. */
function sends(event: KeyboardEvent<HTMLTextAreaElement>): boolean {
  if (event.key !== "Enter" || event.nativeEvent.isComposing) {
    return false;
  }
  return event.metaKey || event.ctrlKey || !(event.shiftKey || event.altKey);
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
}: PromptBoxProps) {
  const own = useRef<HTMLTextAreaElement>(null);
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
            if (sends(event)) {
              event.preventDefault();
              onSend();
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
    </div>
  );
}
