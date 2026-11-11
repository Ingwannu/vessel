// EmptyState (DESIGN.md §6.11): what a list says when it has nothing.
// Specific copy ("No sessions on studio"), one sentence, one action.

import "./empty-state.css";

import type { ReactNode } from "react";

import { Icon, type IconName } from "./icon";

export function EmptyState({
  title,
  icon = "dot",
  children,
  action,
}: {
  title: string;
  icon?: IconName;
  /** One sentence: why it is empty, or what fills it. */
  children?: ReactNode;
  /** One action, a kit Button. */
  action?: ReactNode;
}) {
  return (
    <div className="empty">
      <Icon name={icon} size={16} />
      <h2>{title}</h2>
      {children === undefined ? null : <p>{children}</p>}
      {action}
    </div>
  );
}
