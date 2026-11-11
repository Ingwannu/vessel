// Icons (DESIGN.md §6.13): one stroke drawing per name, drawn by
// styles/icons.css as a CSS mask in the current text colour, so an icon
// takes its row's colour and state without a prop.
//
// Why masks and not an icon package or inline <svg>: the set is small and
// ours; a mask keeps each drawing in one CSS rule, costs no React nodes,
// and follows `color` for hover, disabled and selected states for free.

import "./icon.css";

/** Every drawing the set has. A name here without a `.ico-<name>` rule
 * draws an empty square; the gallery (/kit) shows them all. */
export type IconName =
  | "fleet"
  | "sessions"
  | "sync"
  | "devices"
  | "settings"
  | "plus"
  | "search"
  | "close"
  | "chevron"
  | "chevron-down"
  | "chevron-up"
  | "terminal"
  | "message"
  | "copy"
  | "more"
  | "home"
  | "warn"
  | "check"
  | "kill"
  | "move"
  | "link"
  | "plan"
  | "read"
  | "edit"
  | "exec"
  | "search-kind"
  | "qr"
  | "archive"
  | "dot"
  | "sun"
  | "moon"
  | "monitor"
  | "server"
  | "circle-check"
  | "circle-x"
  | "circle-dashed";

/** The sizes the reference draws at: 12 and 13 in dense chrome, 14 in a
 * composer's controls, 15 in menus, 16 in buttons and tabs, 20 for an
 * empty state's mark. */
export type IconSize = 12 | 13 | 14 | 15 | 16 | 20;

/** A stroke icon in the current text colour. Decorative: the control it
 * sits in carries the name. */
export function Icon({ name, size = 16 }: { name: IconName; size?: IconSize }) {
  return <span className={`ico ico-${name} s${size}`} aria-hidden="true" />;
}
