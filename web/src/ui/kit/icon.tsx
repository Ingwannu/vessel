// Icons (DESIGN.md §6.13): one stroke drawing per name, drawn by
// styles/icons.css as a CSS mask in the current text colour, so an icon
// takes its row's colour and state without a prop.
//
// Why masks and not an icon package or inline <svg>: the set is small and
// ours; a mask keeps each drawing in one CSS rule, costs no React nodes,
// and follows `color` for hover, disabled and selected states for free.

import "./icon.css";

/** Every drawing the set has, once each: the type below and the gallery
 * (/kit) both read this list, so a drawing cannot be missing from the
 * gallery, and a test checks each has its `.ico-<name>` rule (a name
 * without one draws an empty square). Two names for one drawing are not
 * kept: a new use takes the name of the drawing it looks like. */
export const ICON_NAMES = [
  "sync",
  "settings",
  "plus",
  "search",
  "close",
  "chevron",
  "chevron-down",
  "chevron-up",
  "arrow-up",
  "back",
  "stop",
  "shield",
  "folder",
  "machine",
  "board",
  "compose",
  "terminal",
  "message",
  "copy",
  "more",
  "home",
  "warn",
  "check",
  "kill",
  "move",
  "link",
  "plan",
  "read",
  "edit",
  "exec",
  "qr",
  "archive",
  "dot",
  "sun",
  "moon",
  "monitor",
  "circle-check",
  "circle-x",
  "circle-dashed",
  "git-branch",
  "git-compare",
  "git-commit",
  "git-fork",
  "file",
  "file-code",
  "folder-open",
  "folder-plus",
  "undo",
  "refresh",
  "pilcrow",
  "unfold",
  "fold",
  "upload",
  "download",
  "minus",
  "trash",
  "keyboard",
  "bell",
  "sidebar",
  "list",
  "sliders",
  "menu",
  "paperclip",
  "file",
  "trash",
  "pin",
  "grip",
  "steer",
  "queue",
  "restore",
  "enter",
  "circle",
  "help",
  "tag",
] as const;

export type IconName = (typeof ICON_NAMES)[number];

/** The sizes the reference draws at: 12 and 13 in dense chrome, 14 in a
 * composer's controls, 15 in menus, 16 in buttons and tabs, 20 for an
 * empty state's mark. */
export type IconSize = 12 | 13 | 14 | 15 | 16 | 20;

/** A stroke icon in the current text colour. Decorative: the control it
 * sits in carries the name. */
export function Icon({ name, size = 16 }: { name: IconName; size?: IconSize }) {
  return <span className={`ico ico-${name} s${size}`} aria-hidden="true" />;
}
