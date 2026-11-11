// Row (DESIGN.md §6.14): one thing in a list of things: a machine, a
// device, an agent, a setting. The owner asked for it after seeing a
// machine drawn as a box with a heavy dark top edge: neither reference
// marks a row with an edge.
//
// - The list is agent-orchestrator's settings card: radius 8 on the card
//   fill, no border, rows of 10px by 16px divided by hairlines.
// - A row is GOAT's machine row: a 16px icon (or a status dot), the title
//   at 14px medium, under it either one 13px description (a setting) or
//   one 12px line of facts joined with " · " (a machine), then the row's
//   controls and its "…" menu at the right.
// - A chosen row is the 7% wash, never an edge.
// - A row that opens something takes a link as its title; the link
//   stretches over the row (styles/kit-marks.css), and the controls at the
//   right sit above it, so nothing interactive is nested in the link.

import "./row.css";

import type { ReactNode } from "react";

import { Icon, type IconName } from "./icon";

export function RowList({
  label,
  children,
}: {
  /** The list's name for a screen reader ("Machines"). */
  label: string;
  children: ReactNode;
}) {
  return (
    <ul className="row-list" aria-label={label}>
      {children}
    </ul>
  );
}

export function Row({
  icon,
  lead,
  title,
  description,
  meta,
  control,
  menu,
  selected = false,
}: {
  icon?: IconName;
  /** In place of the icon, something of its size: a status dot. */
  lead?: ReactNode;
  /** Text, or a link to what the row is. */
  title: ReactNode;
  /** One line under the title, 13px: what a setting does. */
  description?: ReactNode;
  /** Facts under the title, 12px, joined with " · ": a machine's. */
  meta?: readonly string[];
  /** The row's own controls: a switch, a select, a button. */
  control?: ReactNode;
  /** A kit Menu behind a "…" IconButton. */
  menu?: ReactNode;
  selected?: boolean;
}) {
  const facts = meta?.filter((fact) => fact !== "") ?? [];
  return (
    <li className="row" data-selected={selected || undefined}>
      {lead ??
        (icon === undefined ? null : (
          <span className="row-icon">
            <Icon name={icon} size={16} />
          </span>
        ))}
      <div className="row-text">
        <div className="row-title">{title}</div>
        {description === undefined ? null : (
          <div className="row-description">{description}</div>
        )}
        {facts.length === 0 ? null : (
          <div className="row-meta">{facts.join(" · ")}</div>
        )}
      </div>
      {control === undefined && menu === undefined ? null : (
        <div className="row-controls">
          {control}
          {menu}
        </div>
      )}
    </li>
  );
}
