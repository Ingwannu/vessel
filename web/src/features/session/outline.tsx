// The outline beside a session's timeline (GOAT's minimap: a rail of
// short marks on the conversation's left edge, one per prompt; Paseo's
// "Chat outline" lists the same). Hovering or focusing a mark shows the
// prompt and the agent's last reply; choosing it brings the prompt into
// view through the timeline's one seam (`reveal.ts`). Marks whose prompt
// is on screen are drawn darker. Shown from two prompts on, and hidden
// on a narrow window, where there is no gutter for it.

import "./outline.css";

import { memo, useEffect, useState } from "react";

import { OUTLINE_MIN, type OutlineItem } from "../../data/outline";

/** Whether two sets of keys hold the same keys. */
function sameKeys(a: ReadonlySet<string>, b: ReadonlySet<string>): boolean {
  return a.size === b.size && [...a].every((key) => b.has(key));
}

/** Drawn again only when its items, its scroller or its jump change: not
 * for a batch of records that changed no prompt (structured-session.tsx
 * keeps the items' array then). */
export const Outline = memo(function Outline({
  items,
  scroller,
  onJump,
}: {
  items: readonly OutlineItem[];
  /** The timeline's scrolling element, to tell which prompts are seen. */
  scroller: HTMLElement | null;
  onJump: (key: string) => void;
}) {
  const [seen, setSeen] = useState<ReadonlySet<string>>(new Set());
  // The one card shown, for the mark hovered or focused: drawn only then,
  // so a long session's outline is one button per prompt in the page.
  const [shown, setShown] = useState<string | null>(null);
  useEffect(() => {
    if (scroller === null) {
      return undefined;
    }
    let frame: number | null = null;
    const measure = () => {
      frame = null;
      const bounds = scroller.getBoundingClientRect();
      const visible = new Set<string>();
      for (const element of scroller.querySelectorAll<HTMLElement>(
        "[data-entry][data-kind='prompt']",
      )) {
        const box = element.getBoundingClientRect();
        if (box.bottom > bounds.top && box.top < bounds.bottom) {
          visible.add(element.dataset["entry"] ?? "");
        }
      }
      // The same prompts in view: nothing to draw.
      setSeen((known) => (sameKeys(known, visible) ? known : visible));
    };
    const onScroll = () => {
      frame ??= window.requestAnimationFrame(measure);
    };
    measure();
    scroller.addEventListener("scroll", onScroll, { passive: true });
    return () => {
      scroller.removeEventListener("scroll", onScroll);
      if (frame !== null) {
        window.cancelAnimationFrame(frame);
      }
    };
  }, [scroller, items]);
  if (items.length < OUTLINE_MIN) {
    return null;
  }
  return (
    <nav className="outline" aria-label="Prompts in this session">
      <ol className="outline-rail">
        {items.map((item, index) => (
          <li key={item.key} className="outline-item">
            <button
              type="button"
              className="outline-mark"
              data-in-view={seen.has(item.key) ? "" : undefined}
              aria-label={`Jump to prompt ${String(index + 1)} of ${String(items.length)}: ${item.prompt}`}
              onClick={() => {
                onJump(item.key);
              }}
              onMouseEnter={() => {
                setShown(item.key);
              }}
              onMouseLeave={() => {
                setShown(null);
              }}
              onFocus={() => {
                setShown(item.key);
              }}
              onBlur={() => {
                setShown(null);
              }}
            />
            {shown === item.key ? (
              <div className="outline-card" aria-hidden="true">
                <p className="outline-prompt">{item.prompt}</p>
                {item.reply === null ? null : (
                  <p className="outline-reply">{item.reply}</p>
                )}
              </div>
            ) : null}
          </li>
        ))}
      </ol>
    </nav>
  );
});
