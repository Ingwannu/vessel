// The virtual key bar under a page terminal at compact widths (P4), as
// Paseo's `TerminalPane` draws it (`components/terminal-pane.tsx`
// 161-257, 1241-1257 and its `keyboard*`/`keyButton*` styles): rows of
// keys and sticky modifiers, picked by the bar's own width
// (`data/terminal-keys.ts`), and the soft keyboard's toggle. No Paste, as
// Paseo's web has none.

import {
  type MouseEvent,
  useEffect,
  useRef,
  useState,
  useSyncExternalStore,
} from "react";

import {
  type Control,
  controlId,
  KEY_LABELS,
  keyBarRows,
  MODIFIER_LABELS,
} from "../../data/terminal-keys";
import { Icon } from "../../ui/kit";
import type { TerminalKeyBar } from "./mount-terminal";

export function KeyBar({ bar }: { bar: TerminalKeyBar }) {
  const sticky = useSyncExternalStore(bar.subscribe, bar.sticky);
  const shown = useSyncExternalStore(bar.subscribe, bar.keyboardShown);
  const rowsRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);
  useEffect(() => {
    const rows = rowsRef.current;
    if (rows === null) {
      return undefined;
    }
    const observer = new ResizeObserver(() => {
      setWidth(rows.clientWidth);
    });
    observer.observe(rows);
    return () => {
      observer.disconnect();
    };
  }, []);
  // A press keeps the terminal's focus (and the soft keyboard) where it
  // is; the bar gives the focus back itself.
  const keep = (event: MouseEvent) => {
    event.preventDefault();
  };
  const control = (one: Control) => {
    const id = controlId(one);
    switch (one.type) {
      case "key":
        return (
          <button
            key={id}
            type="button"
            className="terminal-key"
            data-testid={id}
            onMouseDown={keep}
            onClick={() => {
              bar.press(one.key);
            }}
          >
            {KEY_LABELS[one.key]}
          </button>
        );
      case "modifier":
        return (
          <button
            key={id}
            type="button"
            className="terminal-key"
            data-testid={id}
            aria-pressed={sticky[one.modifier]}
            onMouseDown={keep}
            onClick={() => {
              bar.toggle(one.modifier);
            }}
          >
            {MODIFIER_LABELS[one.modifier]}
          </button>
        );
      case "paste":
        return null;
      case "keyboardToggle":
        return (
          <button
            key={id}
            type="button"
            className="terminal-key"
            data-testid={id}
            aria-label={shown ? "Hide keyboard" : "Show keyboard"}
            onMouseDown={keep}
            onClick={() => {
              bar.toggleKeyboard();
            }}
          >
            <Icon name={shown ? "keyboard-off" : "keyboard"} size={16} />
          </button>
        );
    }
  };
  return (
    <div className="terminal-key-bar" data-testid="terminal-virtual-keyboard">
      <div className="terminal-key-rows" ref={rowsRef}>
        {keyBarRows(width, true).map((row) => (
          <div key={row.map(controlId).join(":")} className="terminal-key-row">
            {row.map(control)}
          </div>
        ))}
      </div>
    </div>
  );
}
