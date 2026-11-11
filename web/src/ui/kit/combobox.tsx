// Combobox (DESIGN.md §6.13): a text field that suggests, and still takes
// whatever is typed. agent-orchestrator has no such control; the look is
// GOAT's (T3 Code's) combobox in this kit's popup: the field is a
// TextField's shell, the suggestions are menu rows under it, filtered by
// what has been typed, the first one highlighted.
//
// Behaviour is Base UI's Autocomplete (an ARIA combobox whose value is the
// text): arrow keys move through the suggestions, Enter takes one, Escape
// closes the list and keeps the text.

import "./field-shell.css";
import "./popup.css";
import "./combobox.css";

import { Autocomplete } from "@base-ui/react/autocomplete";

import { Icon, type IconName } from "./icon";

export interface Suggestion {
  /** What choosing it puts in the field. */
  value: string;
  /** A muted note beside it: where it came from, when it was used. */
  detail?: string;
}

export interface ComboboxProps {
  value: string;
  onChange: (value: string) => void;
  suggestions: readonly Suggestion[];
  placeholder?: string;
  /** For paths, links and ids (DESIGN.md §1, principle 5). */
  mono?: boolean;
  size?: "md" | "sm";
  /** `field` is a form's shell; `ghost` a composer's quiet field, with no
   * fill or edge until hovered or focused (agent-orchestrator's tray holds
   * only quiet pills). */
  look?: "field" | "ghost";
  icon?: IconName;
  "aria-label"?: string;
  disabled?: boolean;
}

export function Combobox({
  value,
  onChange,
  suggestions,
  placeholder,
  mono = false,
  size = "md",
  look = "field",
  icon,
  disabled,
  "aria-label": ariaLabel,
}: ComboboxProps) {
  return (
    <Autocomplete.Root
      items={suggestions}
      value={value}
      onValueChange={(next) => {
        onChange(next);
      }}
      itemToStringValue={(suggestion: Suggestion) => suggestion.value}
      openOnInputClick
      autoHighlight
      disabled={disabled}
    >
      <Autocomplete.InputGroup
        className={`text-field${size === "sm" ? " sm" : ""}${mono ? " mono" : ""}${look === "ghost" ? " ghost" : ""}`}
      >
        {icon === undefined ? null : (
          <Icon name={icon} size={size === "sm" ? 14 : 16} />
        )}
        <Autocomplete.Input
          className="text-field-input"
          placeholder={placeholder}
          aria-label={ariaLabel}
          spellCheck={false}
          autoComplete="off"
        />
      </Autocomplete.InputGroup>
      <Autocomplete.Portal>
        <Autocomplete.Positioner
          className="popup-positioner"
          sideOffset={6}
          align="start"
          collisionPadding={8}
        >
          <Autocomplete.Popup className="popup combobox-popup">
            <Autocomplete.List className="popup-scroll">
              {(suggestion: Suggestion) => (
                <Autocomplete.Item
                  key={suggestion.value}
                  value={suggestion}
                  className={mono ? "menu-item mono" : "menu-item"}
                >
                  <span className="label">{suggestion.value}</span>
                  {suggestion.detail === undefined ? null : (
                    <span className="detail">{suggestion.detail}</span>
                  )}
                </Autocomplete.Item>
              )}
            </Autocomplete.List>
          </Autocomplete.Popup>
        </Autocomplete.Positioner>
      </Autocomplete.Portal>
    </Autocomplete.Root>
  );
}
