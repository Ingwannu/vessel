// Alert (Paseo's `components/ui/alert.tsx`, sizes from its
// `control-geometry.ts`): a notice inside the flow of a page, as Paseo
// draws every notice on its settings pages. Banner stays the strip across
// the top of a pane.
//
// [Decision Log]
// - Purpose: Paseo's plugins page (and its other settings pages) say the
//   trust notice, a change's result, a list's error and a machine's
//   offline state with one Alert; the coordinator's P2 review asked for the
//   same component rather than the pane-top Banner.
// - Look, Paseo's: transparent, a 1px border (the variant's accent at half
//   strength, the plain border for success and default), its radius; a
//   lead row of the variant's icon and the title in the accent; the
//   description under it, indented by the icon and its gap; children (a
//   Retry) below. Without a title the description leads.
// - Sizes (padding vertical x horizontal): xs 8x12, sm 12x16, md 16x24,
//   lg 24x32; xs text 12px, the others 14px; icons 12, 14, 16, 20.
// - Role: an error is `alert` (announced at once); the others `status`.

import "./alert.css";

import type { ReactNode } from "react";

import { Icon, type IconName, type IconSize } from "./icon";

export type AlertVariant = "default" | "info" | "success" | "warning" | "error";
export type AlertSize = "xs" | "sm" | "md" | "lg";

const ICONS: Record<Exclude<AlertVariant, "default">, IconName> = {
  info: "info",
  success: "circle-check",
  warning: "warn",
  error: "circle-x",
};

const ICON_SIZES: Record<AlertSize, IconSize> = {
  xs: 12,
  sm: 14,
  md: 16,
  lg: 20,
};

export function Alert({
  title,
  description,
  variant = "default",
  size = "md",
  children,
}: {
  title?: string;
  description?: ReactNode;
  variant?: AlertVariant;
  size?: AlertSize;
  /** Actions under the text (a kit Button). */
  children?: ReactNode;
}) {
  const icon = variant === "default" ? null : ICONS[variant];
  const described =
    description === undefined || description === "" ? null : (
      <div className="alert-description">{description}</div>
    );
  const lead =
    title === undefined ? (
      described
    ) : (
      <div className="alert-title">{title}</div>
    );
  const below = title === undefined ? null : described;
  return (
    <div
      className={`alert ${size} ${variant}`}
      role={variant === "error" ? "alert" : "status"}
    >
      <div className="alert-lead">
        {icon === null ? null : (
          <span className="alert-icon">
            <Icon name={icon} size={ICON_SIZES[size]} />
          </span>
        )}
        {lead === null ? null : <div className="alert-text">{lead}</div>}
      </div>
      {below === null && children === undefined ? null : (
        <div className={icon === null ? "alert-body" : "alert-body indent"}>
          {below}
          {children === undefined ? null : (
            <div className="alert-actions">{children}</div>
          )}
        </div>
      )}
    </div>
  );
}
