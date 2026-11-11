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

/** What leads a row: an icon, or something of its size (a status dot).
 * One or the other, never both. */
export type RowMark = { icon: IconName } | { node: ReactNode };

/** What is under a row's title: a setting's description (13px), or a
 * machine's facts (12px, joined with " · "). One or the other. */
export type RowDetail =
  { description: ReactNode } | { facts: readonly string[] };

function Mark({ mark }: { mark: RowMark }) {
  return "icon" in mark ? (
    <span className="row-icon">
      <Icon name={mark.icon} size={16} />
    </span>
  ) : (
    mark.node
  );
}

function Detail({ detail }: { detail: RowDetail }) {
  if ("description" in detail) {
    return <div className="row-description">{detail.description}</div>;
  }
  const facts = detail.facts.filter((fact) => fact !== "");
  return facts.length === 0 ? null : (
    <div className="row-meta">{facts.join(" · ")}</div>
  );
}

export function Row({
  mark,
  title,
  detail,
  control,
  menu,
  selected = false,
}: {
  mark?: RowMark;
  /** Text, or a link to what the row is. */
  title: ReactNode;
  detail?: RowDetail;
  /** The row's own controls: a switch, a select, a button. */
  control?: ReactNode;
  /** A kit Menu behind a "…" IconButton. */
  menu?: ReactNode;
  selected?: boolean;
}) {
  return (
    <li className="row" data-selected={selected ? "" : undefined}>
      {mark === undefined ? null : <Mark mark={mark} />}
      <div className="row-text">
        <div className="row-title">{title}</div>
        {detail === undefined ? null : <Detail detail={detail} />}
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
