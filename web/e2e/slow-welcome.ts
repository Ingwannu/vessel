// Gate one opaque client hello, so the real server's welcome is delayed.
// Only the discriminator is inspected; bytes and the actual authentication
// proof are unchanged. Neither packets nor credential values are logged.
import type { Page } from "@playwright/test";

export const MAX_WELCOME_HOLD_MS = 5_000;
type Packet = string | Buffer;
type Gate = "waiting" | "held" | "released";

interface WelcomeDelay {
  isHeld: () => boolean;
  didOverrun: () => boolean;
  transportFailed: () => boolean;
  release: () => void;
}

function isHello(packet: Packet): boolean {
  try {
    const value: unknown = JSON.parse(packet.toString());
    return (
      typeof value === "object" &&
      value !== null &&
      "type" in value &&
      value.type === "hello"
    );
  } catch {
    // Opaque packets not owned by this fixture are relayed unchanged.
    return false;
  }
}

export async function slowWelcome(page: Page): Promise<WelcomeDelay> {
  let state: Gate = "waiting";
  let overran = false;
  let failed = false;
  let timer: ReturnType<typeof setTimeout> | null = null;
  let forward: () => void = () => undefined;
  const release = () => {
    if (timer !== null) clearTimeout(timer);
    if (state === "released") return;
    state = "released";
    forward();
    forward = () => undefined;
  };
  await page.routeWebSocket("**/api/ws", (socket) => {
    const server = socket.connectToServer();
    const relay = (packet: Packet) => {
      try {
        server.send(packet);
      } catch {
        // Expose only a failure bit; a transport error must not echo a packet.
        failed = true;
      }
    };
    socket.onMessage((packet) => {
      if (state === "waiting" && isHello(packet)) {
        state = "held";
        forward = () => {
          relay(packet);
        };
        // A safety bound, never the synchronization: the test releases on
        // observed paint/storage events, before the product's auth deadline.
        timer = setTimeout(() => {
          overran = true;
          release();
        }, MAX_WELCOME_HOLD_MS);
        return;
      }
      relay(packet);
    });
  });
  return {
    isHeld: () => state === "held",
    didOverrun: () => overran,
    transportFailed: () => failed,
    release,
  };
}
