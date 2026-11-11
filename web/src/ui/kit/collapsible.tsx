// Collapsible (DESIGN.md §6.13): a titled section a person opens and
// closes, with no browser disclosure triangle.
//
// agent-orchestrator's inspector section: a 14px title in the muted
// colour with a 12px chevron at the right that turns while open, the row
// on the sidebar's growing pill (radius 10), the body 12px in from the
// sides and 14px over the next section. Base UI's Collapsible gives the
// button its `aria-expanded` and ties it to the panel; the panel grows
// from nothing over 220ms (none under reduced motion), as the reference's.

import "./collapsible.css";

import { Collapsible as Base } from "@base-ui/react/collapsible";
import type { ReactNode } from "react";

import { Icon } from "./icon";

export function Collapsible({
  title,
  defaultOpen = true,
  end,
  children,
}: {
  title: string;
  /** Closed at first for detail a person opens on demand. */
  defaultOpen?: boolean;
  /** A quiet note before the chevron: a count. */
  end?: ReactNode;
  children: ReactNode;
}) {
  return (
    <Base.Root className="collapsible" defaultOpen={defaultOpen}>
      <Base.Trigger className="collapsible-trigger">
        <span className="collapsible-title">{title}</span>
        {end === undefined ? null : (
          <span className="collapsible-end">{end}</span>
        )}
        <span className="collapsible-chevron">
          <Icon name="chevron" size={12} />
        </span>
      </Base.Trigger>
      <Base.Panel className="collapsible-panel">
        <div className="collapsible-body">{children}</div>
      </Base.Panel>
    </Base.Root>
  );
}
