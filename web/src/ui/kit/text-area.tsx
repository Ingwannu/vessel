// TextArea (DESIGN.md §6.2): several lines in one shell, growing with its
// text up to a limit, then scrolling.
//
// Why a measured height and not CSS `field-sizing: content`: Firefox, one
// of the browsers ADR 0017 requires, does not grow a textarea by CSS. The
// height is set from `scrollHeight` after each change, capped at
// `maxRows`; the shell is the same as TextField's (one box, the ring on
// the shell), so a focused composer is not a box inside a box.
// Values are T3 Code's textarea (no such control in agent-orchestrator):
// three lines at rest, padding 6px by 12px.

import "./field-shell.css";

import { Field } from "@base-ui/react/field";
import {
  type KeyboardEvent,
  type RefObject,
  useLayoutEffect,
  useRef,
} from "react";

export interface TextAreaProps {
  /** `field` is a field's shell; `bare` draws no shell, for a box that is
   * the shell itself (a composer, whose box carries the edge and ring). */
  look?: "field" | "bare";
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  mono?: boolean;
  /** Lines shown at rest. */
  minRows?: number;
  /** Lines it grows to before it scrolls. */
  maxRows?: number;
  "aria-label"?: string;
  disabled?: boolean;
  /** Shown and selectable, not changed (an editor while its save is on
   * its way: a key typed then would be lost when the save closes it). */
  readOnly?: boolean;
  onKeyDown?: (event: KeyboardEvent<HTMLTextAreaElement>) => void;
  /** For a caller that moves focus into it. */
  textAreaRef?: RefObject<HTMLTextAreaElement | null>;
  /** Fill its container's height and scroll inside, instead of growing
   * with its text (an editor of a whole file: measuring a megabyte of
   * text on every key would cost a layout each time). */
  fill?: boolean;
}

/** The pixels one line takes in `element`, from its computed style. */
function lineHeight(element: HTMLElement): number {
  const parsed = Number.parseFloat(getComputedStyle(element).lineHeight);
  return Number.isFinite(parsed) ? parsed : 20;
}

export function TextArea({
  look = "field",
  value,
  onChange,
  placeholder,
  mono = false,
  minRows = 3,
  maxRows = 10,
  disabled,
  readOnly,
  onKeyDown,
  textAreaRef,
  fill = false,
  "aria-label": ariaLabel,
}: TextAreaProps) {
  const fallback = useRef<HTMLTextAreaElement>(null);
  const own = textAreaRef ?? fallback;
  useLayoutEffect(() => {
    const element = own.current;
    if (element === null || fill) {
      return;
    }
    const line = lineHeight(element);
    const style = getComputedStyle(element);
    const padding =
      Number.parseFloat(style.paddingTop) +
      Number.parseFloat(style.paddingBottom);
    const min = line * minRows + padding;
    const max = line * maxRows + padding;
    // Shrink first so a deleted line gives its height back.
    element.style.height = "auto";
    const wanted = Math.min(Math.max(element.scrollHeight, min), max);
    element.style.height = `${String(wanted)}px`;
    element.style.overflowY = element.scrollHeight > max ? "auto" : "hidden";
  }, [own, value, minRows, maxRows, fill]);
  return (
    <span
      className={`text-field area${look === "bare" ? " bare" : ""}${mono ? " mono" : ""}${fill ? " fill" : ""}`}
      data-disabled={disabled === true ? "" : undefined}
    >
      <Field.Control
        render={
          <textarea
            ref={own}
            className="text-field-input"
            rows={minRows}
            value={value}
            placeholder={placeholder}
            disabled={disabled}
            readOnly={readOnly}
            spellCheck={false}
            aria-label={ariaLabel}
            onKeyDown={onKeyDown}
            onChange={(event) => {
              onChange(event.target.value);
            }}
          />
        }
      />
    </span>
  );
}
