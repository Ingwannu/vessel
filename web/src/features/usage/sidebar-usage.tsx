// Paseo's sidebar usage item (usage/sidebar-item.tsx): above the sidebar's
// foot while "Show in sidebar" is on, the pinned windows of one host's
// reports (Paseo pinned.ts): per source its mark, per window a meter,
// its percent and short label, in the richest layout that fits the row
// (N2's quotaPinnedLayout, measured off screen as Paseo measures). A click
// opens the usage modal.
import "./sidebar-usage.css";

import { Fragment, useLayoutEffect, useRef, useState } from "react";

import { type QuotaMark, quotaCards } from "../../data/account-usage/cards";
import { quotaSummaryHost } from "../../data/account-usage/hosts";
import { effectivePins } from "../../data/account-usage/pins";
import {
  type QuotaPinnedLayout,
  quotaPinnedLayout,
} from "../../data/account-usage/pinned-layout";
import type { QuotaPercent, QuotaTone } from "../../data/account-usage/values";
import { Icon, ProviderMark, QuotaMeter } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import { UsageModal } from "./usage-modal";
import { useAccountReader, useQuotaClock } from "./use-account-usage";
import { useUsageHosts } from "./use-usage-hosts";
import { useUsagePrefs } from "./use-usage-prefs";

/** A pinned window, as the line draws it. */
export interface SummaryWindow {
  key: string;
  percent: QuotaPercent;
  tone: QuotaTone;
  short: string;
}
/** A source and its pinned windows, as the line draws them. */
export interface SummarySource {
  key: string;
  mark: QuotaMark | null;
  windows: SummaryWindow[];
}

export function SidebarUsage() {
  const { inSidebar } = useUsagePrefs();
  const { hosts, active, picked } = useUsageHosts();
  const node = quotaSummaryHost({ picked, active, hosts });
  if (!inSidebar || node === null) return null;
  return <SummaryLine node={node} />;
}

function SummaryLine({ node }: { node: NodeKey }) {
  const { read } = useAccountReader(node, true, null);
  const { usage } = useUsagePrefs();
  const { now, locale } = useQuotaClock();
  const [open, setOpen] = useState(false);
  const sources: SummarySource[] = [];
  if (read.kind === "ready") {
    const pins = effectivePins(usage.pins, read.reports, usage.display);
    for (const card of quotaCards(read.reports, usage.display, locale, now)) {
      const windows = card.windows.flatMap((window): SummaryWindow[] => {
        const pinned = pins.some(
          (pin) => pin.source === card.source && pin.window === window.key,
        );
        return pinned && window.percent !== null
          ? [
              {
                key: window.key,
                percent: window.percent,
                tone: window.tone,
                short: window.short,
              },
            ]
          : [];
      });
      if (windows.length > 0)
        sources.push({ key: card.key, mark: card.mark, windows });
    }
  }
  if (sources.length === 0) return null;
  return (
    <>
      <SummaryRow
        sources={sources}
        locale={locale}
        onOpen={() => {
          setOpen(true);
        }}
      />
      {open ? (
        <UsageModal
          initial={node}
          onClose={() => {
            setOpen(false);
          }}
        />
      ) : null}
    </>
  );
}

/** The row from its sources, in the richest layout that fits it; also
 * the kit gallery's specimen at the sidebar's width. */
export function SummaryRow({
  sources,
  locale,
  onOpen,
}: {
  sources: readonly SummarySource[];
  locale: string;
  onOpen: () => void;
}) {
  const line = useRef<HTMLSpanElement>(null);
  const labels = useRef<HTMLSpanElement>(null);
  const percents = useRef<HTMLSpanElement>(null);
  const [layout, setLayout] = useState<QuotaPinnedLayout>("labels");
  const count = sources.reduce((sum, source) => sum + source.windows.length, 0);
  // Paseo re-renders after onLayout reports the widths; here they are read
  // before paint, and again whenever the row resizes.
  useLayoutEffect(() => {
    const row = line.current;
    if (row === null) return undefined;
    const measure = () => {
      setLayout(
        quotaPinnedLayout({
          availableWidth: row.clientWidth,
          labelsWidth: labels.current?.scrollWidth ?? null,
          percentsWidth: percents.current?.scrollWidth ?? null,
          windowCount: count,
        }),
      );
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(row);
    return () => {
      observer.disconnect();
    };
  }, [count, sources.length, locale]);
  return (
    <button
      type="button"
      className="side-usage"
      aria-label="Open usage"
      onClick={onOpen}
    >
      <span ref={line} className="side-usage-line">
        <Line sources={sources} layout={layout} locale={locale} />
      </span>
      <span className="side-usage-measure" aria-hidden="true">
        <span ref={labels}>
          <Line sources={sources} layout="labels" locale={locale} />
        </span>
        <span ref={percents}>
          <Line sources={sources} layout="percents" locale={locale} />
        </span>
      </span>
    </button>
  );
}

function Line({
  sources,
  layout,
  locale,
}: {
  sources: readonly SummarySource[];
  layout: QuotaPinnedLayout;
  locale: string;
}) {
  // Paseo's PinnedUsageLine: every item a direct child of the one row,
  // packed from the left; only the meter slots grow (in "meters"), so the
  // meters share the leftover width and nothing else spreads or is cut.
  return sources.map((source, index) => (
    <Fragment key={source.key}>
      <span
        className={index === 0 ? "side-usage-source" : "side-usage-source gap"}
      >
        {source.mark === null ? (
          <Icon name="plan" size={14} />
        ) : (
          <ProviderMark name={source.mark} size={14} />
        )}
      </span>
      {source.windows.map((window, windowIndex) => {
        const lead = windowIndex === 0 ? "after-mark" : "after-window";
        return (
          <Fragment key={window.key}>
            {layout === "meters" ? (
              <span className={`side-usage-meter ${lead}`}>
                <QuotaMeter
                  value={window.percent}
                  label={window.short}
                  tone={window.tone}
                  emphasis="quiet"
                />
              </span>
            ) : null}
            <span
              className={
                layout === "meters"
                  ? "side-usage-percent"
                  : `side-usage-percent ${lead}`
              }
            >
              {window.percent.format(locale)}
              {layout === "percents" || window.short === "" ? null : (
                <span className="side-usage-label">{` ${window.short}`}</span>
              )}
            </span>
          </Fragment>
        );
      })}
    </Fragment>
  ));
}
