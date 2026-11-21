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
// - TRACKER 834: over the list, a card for the active command that says
//   what it does (Paseo's `detailCard`, components/ui/autocomplete.tsx
//   247-261, styles 297-323): its name, its description in two lines at
//   most, then the input it takes. Paseo draws it whenever the selected
//   option is a command with a description; the native app's kit has it
//   (`autocomplete::detail_card`). The card repeats the active row's
//   words, which the text already names through `aria-activedescendant`,
//   so it is hidden from assistive technology rather than read twice.

import "./suggestions.css";

import type { ReactNode } from "react";

import { Icon, type IconName } from "./icon";

/** What the card over the list says of the active command. */
export interface SuggestionCard {
  /** The command as typed ("/review"). */
  title: string;
  /** What it does; the card is drawn only when this is not empty. */
  description: string;
  /** The input it takes ("text", "<optional instructions>"). */
  hint?: string;
}

export interface SuggestionItem {
  key: string;
  /** The row's name ("/review", "main.rs"). */
  label: ReactNode;
  /** A muted second part: a description, a file's folder. */
  detail?: ReactNode;
  /** A small tag at the right end. */
  badge?: string;
  icon?: IconName;
  /** A command's card, shown over the list while the row is active. */
  card?: SuggestionCard;
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
  const card = items[active]?.card;
  return (
    <div className="suggestions-stack">
      {card === undefined || card.description === "" ? null : (
        <div className="suggestion-card popup" aria-hidden="true">
          <p className="suggestion-card-title">{card.title}</p>
          <p className="suggestion-card-description">{card.description}</p>
          {card.hint === undefined || card.hint === "" ? null : (
            <p className="suggestion-card-hint">{card.hint}</p>
          )}
        </div>
      )}
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
    </div>
  );
}
