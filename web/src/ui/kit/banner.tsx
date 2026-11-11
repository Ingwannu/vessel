// Banner (DESIGN.md §6.11): a warning or an error across the top of a
// pane, announced at once, with at most one action (Retry).

import "./banner.css";

import type { ReactNode } from "react";

import { Icon } from "./icon";

export function Banner({
  tone = "warn",
  children,
  action,
}: {
  tone?: "warn" | "error";
  children: ReactNode;
  /** One kit Button. */
  action?: ReactNode;
}) {
  return (
    <div
      className={tone === "error" ? "banner error" : "banner"}
      role={tone === "error" ? "alert" : "status"}
      aria-live="assertive"
    >
      <Icon name="warn" size={14} />
      <span>{children}</span>
      {action}
    </div>
  );
}
