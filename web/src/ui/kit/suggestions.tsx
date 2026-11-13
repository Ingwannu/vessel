// The composer's suggestion list (milestone 11, Track A-page): `/` and
// `@` in one menu, agent-orchestrator's `ComposerSuggestMenu`. It sits on
// the composer's top edge, keyboard first: the text keeps focus and
// drives it (Up, Down, Tab or Enter, Escape), the list says which row is
// active through `aria-activedescendant` on the text.
//
// [Decision Log]
// - Purpose: one list for the agent's commands and the session's files,
//   in the kit so the two menus and any later one cannot drift apart.
// - Constraint: the text must keep focus while the list is open (typing
//   narrows it), so it cannot be a Base UI menu, which takes focus.
// - Alternatives: a list anchored to the caret (measured through a mirror
//   of the text area); the three references all anchor it to the
//   composer's top-left edge instead, as here.
// - Choice: a listbox the caller positions over the composer, with an
//   id per option for the text's `aria-activedescendant`; rows of a
//   label, a muted detail and a badge, the active one showing the key
//   that inserts it. A note row stands in for an empty or loading list.
// - Cost: the caller owns the keys (`moveActive`) and the anchoring.

import "./suggestions.css";

import type { ReactNode } from "react";

import { Icon, type IconName } from "./icon";

export interface SuggestionItem {
  key: string;
  /** The row's name ("/review", "main.rs"). */
  label: ReactNode;
  /** A muted second part: a description, a file's folder. */
  detail?: ReactNode;
  /** A small tag at the right end. */
  badge?: string;
  icon?: IconName;
}

/** The index after `active` moved by `step` in a list of `count`,
 * wrapping at both ends (agent-orchestrator's). */
export function moveActive(
  active: number,
  step: 1 | -1,
  count: number,
): number {
  if (count === 0) {
    return 0;
  }
  return (active + step + count) % count;
}

/** The id of option `index` of the list `id`, for the text's
 * `aria-activedescendant`. */
export function suggestionId(id: string, index: number): string {
  return `${id}-${String(index)}`;
}

export function Suggestions({
  id,
  label,
  items,
  active,
  note,
  footer,
  onPick,
  onHover,
}: {
  id: string;
  /** What the list is ("Commands", "Files"). */
  label: string;
  items: readonly SuggestionItem[];
  active: number;
  /** Said instead of rows: loading, nothing found, nothing to search. */
  note?: string;
  /** A quiet line under the rows (a list cut short). */
  footer?: string;
  onPick: (index: number) => void;
  onHover: (index: number) => void;
}) {
  return (
    <div className="suggestions popup">
      <ul
        id={id}
        role="listbox"
        aria-label={label}
        className="suggestions-list"
      >
        {items.map((item, index) => (
          <li
            key={item.key}
            id={suggestionId(id, index)}
            role="option"
            aria-selected={index === active}
            className="suggestion"
            data-active={index === active ? "" : undefined}
            // The text keeps focus: a press picks without taking it.
            onMouseDown={(event) => {
              event.preventDefault();
            }}
            onClick={() => {
              onPick(index);
            }}
            onMouseMove={() => {
              if (index !== active) {
                onHover(index);
              }
            }}
          >
            {item.icon === undefined ? null : (
              <span className="suggestion-icon">
                <Icon name={item.icon} size={14} />
              </span>
            )}
            <span className="suggestion-text">
              <span className="suggestion-label">{item.label}</span>
              {item.detail === undefined ? null : (
                <span className="suggestion-detail">{item.detail}</span>
              )}
            </span>
            {item.badge === undefined ? null : (
              <span className="suggestion-badge">{item.badge}</span>
            )}
            {index === active ? (
              <span className="suggestion-key" aria-hidden="true">
                <Icon name="enter" size={12} />
              </span>
            ) : null}
          </li>
        ))}
      </ul>
      {note === undefined ? null : (
        <p className="suggestions-note" role="status">
          {note}
        </p>
      )}
      {footer === undefined ? null : (
        <p className="suggestions-foot">{footer}</p>
      )}
    </div>
  );
}
