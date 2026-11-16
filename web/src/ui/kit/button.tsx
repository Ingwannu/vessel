// Button and IconButton (DESIGN.md §6.1): the page's one action control.
// Every action is one of these; rows and cards open things with links.
//
// [Decision Log]
// - Purpose: one place for the button's look and its states, so a screen
//   cannot invent its own variant or forget the disabled style.
// - Constraint: Base UI composes its triggers onto the element a caller
//   renders (`render={<Button />}`), merging its props (`onClick`,
//   `aria-*`, `data-popup-open`, `ref`) into ours. So every prop that is
//   not the button's own design is passed through to the <button>.
// - Choice: variants and sizes are closed unions mapped to classes in one
//   exhaustive switch each; the look is styles/kit-buttons.css. No
//   `className`: placement belongs to the parent's layout, the look to
//   the kit.
// - Values: agent-orchestrator's: 32, 28 and 24px tall, radius 8, a
//   filled secondary at rest, the primary in the ink with an inset edge
//   and no outer shadow, ghosts with its growing hover pill.
// - `outline` (ADR 0105): Paseo's outline button (`button.tsx`, no fill,
//   a strong border), for the choices inside a callout that has its own
//   fill, where a filled secondary would sit as a block on a block (the
//   file conflict alert's Overwrite and Reload).

import "./button.css";

import type { ComponentProps, ReactNode } from "react";

import { Icon, type IconName } from "./icon";
import { Spinner } from "./spinner";

/** How loud the action is. One `primary` per view. */
export type ButtonVariant =
  "secondary" | "primary" | "ghost" | "outline" | "danger" | "danger-solid";

/** 32px (a form's), 28px (a row's or a toolbar's), 24px (dense rows). */
export type ButtonSize = "md" | "sm" | "xs";

/** Its corners: a control's 8px; `soft`, the 10px of a composer's quiet
 * triggers (agent-orchestrator's `rounded-lg`); `round`, a pill, or a
 * circle for an icon (its Send and "+"). */
export type ButtonShape = "rect" | "soft" | "round";

type NativeButton = Omit<
  ComponentProps<"button">,
  "className" | "style" | "type" | "children"
>;

function variantClass(variant: ButtonVariant): string {
  switch (variant) {
    case "secondary":
      return "";
    case "primary":
      return " primary";
    case "ghost":
      return " ghost";
    case "outline":
      // Not `outline`: the session's outline rail owns that class.
      return " edged";
    case "danger":
      return " danger";
    case "danger-solid":
      return " danger solid";
  }
}

function shapeClass(shape: ButtonShape): string {
  switch (shape) {
    case "rect":
      return "";
    case "soft":
      return " soft";
    case "round":
      return " round";
  }
}

function sizeClass(size: ButtonSize): string {
  switch (size) {
    case "md":
      return "";
    case "sm":
      return " sm";
    case "xs":
      return " xs";
  }
}

/** The icon size that sits in a button of `size`: the reference's 16 in
 * buttons, 14 in the densest (24px) ones. */
function iconSize(size: ButtonSize): 16 | 14 {
  switch (size) {
    case "md":
    case "sm":
      return 16;
    case "xs":
      return 14;
  }
}

export interface ButtonProps extends NativeButton {
  variant?: ButtonVariant;
  size?: ButtonSize;
  shape?: ButtonShape;
  /** `submit` only inside a form that handles its own submit event. */
  type?: "button" | "submit";
  /** An icon before the label. */
  icon?: IconName;
  /** Something after the label: a shortcut hint, a count, a chevron. */
  end?: ReactNode;
  /** Shown instead of the icon while an action runs. The button keeps its
   * width and says so to a screen reader with `aria-busy`. */
  busy?: boolean;
  children: ReactNode;
}

export function Button({
  variant = "secondary",
  size = "md",
  shape = "rect",
  type = "button",
  icon,
  end,
  busy = false,
  children,
  ...rest
}: ButtonProps) {
  return (
    <button
      {...rest}
      type={type}
      aria-busy={busy || undefined}
      className={`btn${variantClass(variant)}${sizeClass(size)}${shapeClass(shape)}`}
    >
      {busy ? (
        <Spinner size={iconSize(size) === 16 ? 14 : 12} />
      ) : icon === undefined ? null : (
        <Icon name={icon} size={iconSize(size)} />
      )}
      {children}
      {end}
    </button>
  );
}

export interface IconButtonProps extends NativeButton {
  /** The action's name: the button's accessible name, and its tooltip
   * where the caller wraps it in one. */
  label: string;
  icon: IconName;
  /** `primary` only for a view's one primary action drawn as an icon (a
   * composer's Send). */
  variant?: Exclude<ButtonVariant, "danger-solid">;
  size?: ButtonSize;
  shape?: ButtonShape;
  /** `submit` only inside a form that handles its own submit event. */
  type?: "button" | "submit";
}

/** A square button with only an icon. It must have a name (`label`):
 * the reference shows a tooltip for one whose meaning is not obvious, and
 * a screen reader always needs it. */
export function IconButton({
  label,
  icon,
  variant = "ghost",
  size = "sm",
  shape = "rect",
  type = "button",
  ...rest
}: IconButtonProps) {
  return (
    <button
      {...rest}
      type={type}
      aria-label={label}
      className={`btn icon-btn${variantClass(variant)}${sizeClass(size)}${shapeClass(shape)}`}
    >
      <Icon name={icon} size={iconSize(size)} />
    </button>
  );
}
