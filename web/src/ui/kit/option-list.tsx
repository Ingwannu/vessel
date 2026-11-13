// A list of answers to pick from (milestone 11's questions):
// agent-orchestrator's elicitation choices, each row a number badge, its
// label and a muted description; one picked (`radio`) or several
// (`checkbox`); the digit keys 1 to 9 pick a row while focus is in the
// list. Rows, not radio buttons: the reference's, and an answer with a
// description reads as one target.

import "./option-list.css";

import type { KeyboardEvent } from "react";

export interface OptionRow {
  value: string;
  label: string;
  description?: string | null;
}

export function OptionList({
  label,
  options,
  picked,
  multiple,
  onChange,
}: {
  /** What is asked, for the group's name. */
  label: string;
  options: readonly OptionRow[];
  picked: readonly string[];
  multiple: boolean;
  onChange: (picked: string[]) => void;
}) {
  const toggle = (value: string) => {
    if (!multiple) {
      onChange([value]);
      return;
    }
    onChange(
      picked.includes(value)
        ? picked.filter((other) => other !== value)
        : [...picked, value],
    );
  };
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const digit = Number(event.key);
    const option = options[digit - 1];
    if (Number.isInteger(digit) && digit >= 1 && option !== undefined) {
      event.preventDefault();
      toggle(option.value);
    }
  };
  return (
    <div
      className="option-list"
      role={multiple ? "group" : "radiogroup"}
      aria-label={label}
      onKeyDown={onKeyDown}
    >
      {options.map((option, index) => {
        const on = picked.includes(option.value);
        return (
          <button
            key={option.value}
            type="button"
            role={multiple ? "checkbox" : "radio"}
            aria-checked={on}
            className="option-row"
            onClick={() => {
              toggle(option.value);
            }}
          >
            <span className="option-badge" aria-hidden="true">
              {index < 9 ? String(index + 1) : ""}
            </span>
            <span className="option-text">
              <span className="option-label">{option.label}</span>
              {option.description === undefined ||
              option.description === null ? null : (
                <span className="option-description">{option.description}</span>
              )}
            </span>
          </button>
        );
      })}
    </div>
  );
}
