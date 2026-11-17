// The frame every Settings section is drawn in (track W2):
// agent-orchestrator's settings page, a centred column with one lead
// sentence, then sections, each a heading over a card of rows.
//
// - The column is GOAT's readable width (896px), centred, with
//   agent-orchestrator's padding (24px, 48px below); sections are 40px
//   apart, a heading 12px above its card (its `--size-settings-*`).
// - The page's own title is the frame's title bar (the last crumb), as
//   the reference puts it in its top bar; a section's heading is 14px
//   medium, inset to the rows' text edge, with one action at its right
//   (a ghost button, a machine picker).

import "./settings-page.css";

import { type ReactNode, useId } from "react";

export function SettingsPage({
  lead,
  children,
}: {
  /** One sentence: what this section is for. A page whose first section
   * is not what the sentence describes (Usage, where Paseo's plan usage
   * comes before the ledger) puts it in that section instead. */
  lead?: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="settings-page">
      <div className="settings-column">
        {lead === undefined ? null : <p className="settings-lead">{lead}</p>}
        {children}
      </div>
    </div>
  );
}

export function SettingsSection({
  title,
  action,
  children,
}: {
  title: string;
  /** At the right of the heading line. */
  action?: ReactNode;
  children: ReactNode;
}) {
  const id = useId();
  return (
    <section className="settings-section" aria-labelledby={id}>
      <div className="settings-section-head">
        <h2 id={id}>{title}</h2>
        {action === undefined ? null : (
          <div className="settings-section-action">{action}</div>
        )}
      </div>
      {children}
    </section>
  );
}

/** A label and its value, as a row of a facts card (the reference's
 * settings row with a value at the right). `mono` for a literal: an
 * address, an id, a version. */
export function FactValue({
  children,
  mono = false,
}: {
  children: ReactNode;
  mono?: boolean;
}) {
  return (
    <span className={mono ? "fact-value mono" : "fact-value"}>{children}</span>
  );
}
