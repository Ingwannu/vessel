// TextField (DESIGN.md §6.2): one line of text in one shell.
//
// [Decision Log]
// - Purpose: the owner pointed at the sidebar search, whose pill drew a
//   second box inside it: the inner <input> drew its own focus outline.
//   A field here is one shell: the shell has the edge, the fill and the
//   focus ring (`:focus-within`); the input inside has no border, no
//   outline and no fill of its own (styles/kit-fields.css).
// - Choice: Base UI's Input, so a Field's label, hint and error reach it.
//   `type` is text or url only: a `search` input draws the browser's own
//   clear button, which this design does not have.
// - Values: agent-orchestrator's settings input (a 4% fill inside an 8%
//   hairline, radius 8) at its control heights, 32 and 28.

import "./field-shell.css";

import { Input } from "@base-ui/react/input";
import type { KeyboardEvent, ReactNode, Ref } from "react";

import { Icon, type IconName } from "./icon";

export interface TextFieldProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  /** For machine text a person could type: paths, links, ids (DESIGN.md
   * §1, principle 5). */
  mono?: boolean;
  size?: "md" | "sm";
  /** `url` brings the right keyboard on a phone; nothing else differs. */
  type?: "text" | "url";
  /** A leading icon, inside the shell. */
  icon?: IconName;
  /** Something at the end, inside the shell: a shortcut, a clear button. */
  end?: ReactNode;
  /** The control's name when it is not inside a Field. */
  "aria-label"?: string;
  /** Marks a search box for assistive technology. */
  role?: "searchbox" | "combobox";
  disabled?: boolean;
  readOnly?: boolean;
  autoFocus?: boolean;
  spellCheck?: boolean;
  onKeyDown?: (event: KeyboardEvent<HTMLInputElement>) => void;
  /** Focus left the field (an edit in place saves then). */
  onBlur?: () => void;
  inputRef?: Ref<HTMLInputElement>;
}

function shellClass(size: "md" | "sm", mono: boolean): string {
  return `text-field${size === "sm" ? " sm" : ""}${mono ? " mono" : ""}`;
}

export function TextField({
  value,
  onChange,
  placeholder,
  mono = false,
  size = "md",
  type = "text",
  icon,
  end,
  disabled,
  readOnly,
  autoFocus,
  spellCheck = false,
  onKeyDown,
  onBlur,
  inputRef,
  role,
  "aria-label": ariaLabel,
}: TextFieldProps) {
  return (
    <span
      className={shellClass(size, mono)}
      data-disabled={disabled === true ? "" : undefined}
    >
      {icon === undefined ? null : (
        <Icon name={icon} size={size === "sm" ? 14 : 16} />
      )}
      <Input
        className="text-field-input"
        ref={inputRef}
        type={type}
        value={value}
        onValueChange={(next) => {
          onChange(next);
        }}
        placeholder={placeholder}
        disabled={disabled}
        readOnly={readOnly}
        autoFocus={autoFocus}
        spellCheck={spellCheck}
        autoComplete="off"
        onKeyDown={onKeyDown}
        onBlur={onBlur}
        role={role}
        aria-label={ariaLabel}
      />
      {end}
    </span>
  );
}
