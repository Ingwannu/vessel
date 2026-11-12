// The conditions the page's load is measured under (perf.ts), each
// applied to a fresh browser context through Chromium's DevTools
// protocol, and what the page transferred under them.
//
// The slow link is Lighthouse's mobile "slow 4G" as Lighthouse states it
// (1.6 Mbit/s down, 750 kbit/s up, 150 ms round trip). Chromium applies
// the round trip as a fixed delay between a request and its response's
// headers, per request, with no TCP slow start: milder than a real link of
// that round trip, and the same every run, which is what a before and an
// after need.

import type { BrowserContextOptions, CDPSession } from "@playwright/test";

export type ProfileName = "desktop" | "slow-link" | "phone";

export interface Conditions {
  readonly name: ProfileName;
  /** What it stands for, as the report states it. */
  readonly description: string;
  readonly context: BrowserContextOptions;
  /** `null`: the loopback link as it is. */
  readonly network: {
    readonly latencyMs: number;
    readonly downBitsPerSecond: number;
    readonly upBitsPerSecond: number;
  } | null;
  /** How many times slower the CPU runs (1: as it is). */
  readonly cpuSlowdown: number;
}

const SLOW_4G = {
  latencyMs: 150,
  downBitsPerSecond: 1_600_000,
  upBitsPerSecond: 750_000,
};

/** The desktop window every measurement but the load's phone uses. */
export const DESKTOP: BrowserContextOptions = {
  viewport: { width: 1440, height: 900 },
  deviceScaleFactor: 1,
  colorScheme: "light",
};

export const PROFILES: readonly Conditions[] = [
  {
    name: "desktop",
    description:
      "1440×900 at 1×, the loopback link unthrottled, the CPU as it is",
    context: DESKTOP,
    network: null,
    cpuSlowdown: 1,
  },
  {
    name: "slow-link",
    description:
      "1440×900 at 1×, 1.6 Mbit/s down, 750 kbit/s up, 150 ms per request, the CPU as it is",
    context: DESKTOP,
    network: SLOW_4G,
    cpuSlowdown: 1,
  },
  {
    name: "phone",
    description:
      "390×844 at 3×, touch, the slow link's network, the CPU 4× slower",
    context: {
      viewport: { width: 390, height: 844 },
      deviceScaleFactor: 3,
      isMobile: true,
      hasTouch: true,
      colorScheme: "light",
    },
    network: SLOW_4G,
    cpuSlowdown: 4,
  },
];

/** Apply `conditions` to the page behind `cdp`, before it navigates. */
export async function applyConditions(
  cdp: CDPSession,
  conditions: Conditions,
): Promise<void> {
  const network = conditions.network;
  if (network !== null) {
    await cdp.send("Network.emulateNetworkConditions", {
      offline: false,
      latency: network.latencyMs,
      // The protocol counts bytes a second.
      downloadThroughput: network.downBitsPerSecond / 8,
      uploadThroughput: network.upBitsPerSecond / 8,
    });
  }
  if (conditions.cpuSlowdown !== 1) {
    await cdp.send("Emulation.setCPUThrottlingRate", {
      rate: conditions.cpuSlowdown,
    });
  }
}

/** Bytes a page received, by what they were. */
export interface Transfer {
  readonly document: number;
  readonly script: number;
  readonly stylesheet: number;
  readonly font: number;
  readonly other: number;
  /** Every HTTP response: the five above together. */
  readonly total: number;
  /** Payloads of the WebSocket frames received (the bridge), which the
   * HTTP total leaves out. */
  readonly websocket: number;
}

type Kind = "document" | "script" | "stylesheet" | "font" | "other";

/** Counts what the page behind `cdp` receives from when it is made
 * (after `Network.enable`): `Network.loadingFinished`'s
 * `encodedDataLength` (the bytes on the wire, headers included), by the
 * resource type `Network.responseReceived` gave the request. */
export class TransferCount {
  readonly #kinds = new Map<string, Kind>();
  readonly #bytes = new Map<Kind, number>();
  #websocket = 0;

  constructor(cdp: CDPSession) {
    cdp.on("Network.responseReceived", (event) => {
      this.#kinds.set(event.requestId, kindOf(event.type));
    });
    cdp.on("Network.loadingFinished", (event) => {
      const kind = this.#kinds.get(event.requestId) ?? "other";
      this.#bytes.set(
        kind,
        (this.#bytes.get(kind) ?? 0) + event.encodedDataLength,
      );
    });
    cdp.on("Network.webSocketFrameReceived", (event) => {
      const frame = event.response;
      // Text frames are UTF-8; binary ones arrive in base64.
      this.#websocket +=
        frame.opcode === 2
          ? Buffer.from(frame.payloadData, "base64").length
          : Buffer.byteLength(frame.payloadData, "utf8");
    });
  }

  /** What was received so far. */
  now(): Transfer {
    const of = (kind: Kind): number => this.#bytes.get(kind) ?? 0;
    return {
      document: of("document"),
      script: of("script"),
      stylesheet: of("stylesheet"),
      font: of("font"),
      other: of("other"),
      total: [...this.#bytes.values()].reduce((sum, bytes) => sum + bytes, 0),
      websocket: this.#websocket,
    };
  }
}

function kindOf(type: string): Kind {
  switch (type) {
    case "Document":
      return "document";
    case "Script":
      return "script";
    case "Stylesheet":
      return "stylesheet";
    case "Font":
      return "font";
    default:
      return "other";
  }
}
