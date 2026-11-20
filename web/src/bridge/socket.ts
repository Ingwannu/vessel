// The page's one websocket to `vessel ui` (ADR 0017): dialling, the
// mutual proof, redialling, and when to stop. What the frames mean is
// `client.ts`'s; this hands it each verified frame and every change of
// the link's state.
//
// [Decision Log]
// - Purpose: every screen talks to nodes through this one socket. It must
//   survive the socket dropping (a laptop sleeping) and never carry a
//   frame before the server has proved itself.
// - The socket redials with a capped backoff. Only a refusal ends the
//   tab's credential: the bridge closing with 4401, a server that fails
//   its proof, or a different launch (a restarted UI). Those are
//   `refused`, and the app forgets the session key.
// - A connection that keeps dropping (or never completes its handshake)
//   is not a refusal: after `MAX_RETRIES` the tab stops redialling and
//   says it is `disconnected`, keeping the key, so the Retry button or a
//   reload resumes the same launch once it answers again. A restarted UI
//   still fails the Host check or the proof, so stopping keeps every
//   guarantee the refusal gave. (A tab once deleted a valid key after five
//   transient closes, and nothing could recover it: verification finding
//   1, M7.)
// - Divided from `client.ts` (milestone 11, track W): the lifecycle of
//   the socket is one job, the meaning of its frames another.

import { Handshake } from "./auth";
import { rememberSession, type Credential } from "./token";
import * as frames from "./frames";
import type { ClientFrame, ServerFrame } from "../wire/bridge";

/** Where the page's link to `vessel ui` is. */
export type LinkState =
  | { kind: "connecting"; attempt: number }
  | { kind: "open"; version: string }
  | { kind: "closed"; retryInMs: number; attempt: number }
  /** Stopped redialling after repeated failures; the key is kept. */
  | { kind: "disconnected"; attempts: number }
  /** The bridge refused this tab, or a server failed to prove itself. */
  | { kind: "refused" };

/** What the socket tells its owner. */
export interface SocketEvents {
  /** A frame after the welcome (the welcome itself opens the link). */
  frame: (frame: ServerFrame) => void;
  /** The link's state changed. */
  link: (link: LinkState) => void;
  /** The socket went: whatever waited on an answer will not get one. */
  lost: () => void;
}

const CLOSE_UNAUTHORIZED = 4401;
const FIRST_RETRY_MS = 500;
const MAX_RETRY_MS = 10_000;
const MAX_RETRIES = 5;
/** How long a handshake may take before the attempt counts as dropped. */
const HANDSHAKE_MS = 10_000;

/** The websocket address of the page's own server. */
export function socketUrl(): string {
  const scheme = window.location.protocol === "https:" ? "wss:" : "ws:";
  return `${scheme}//${window.location.host}/api/ws`;
}

export class BridgeSocket {
  #credential: Credential;
  readonly #url: string;
  readonly #events: SocketEvents;
  #socket: WebSocket | null = null;
  #attempt = 0;
  #stopped = false;
  #retry: ReturnType<typeof setTimeout> | null = null;
  #link: LinkState = { kind: "connecting", attempt: 0 };

  constructor(credential: Credential, url: string, events: SocketEvents) {
    this.#credential = credential;
    this.#url = url;
    this.#events = events;
  }

  get link(): LinkState {
    return this.#link;
  }

  /** The session key, only while a welcomed socket has just proved this
   * is our server: the HTML preview's post carries it as a bearer (ADR
   * 0122), so it is never sent to a server that has not proved itself. */
  get sessionKey(): string | null {
    return this.#link.kind === "open" && this.#credential.kind === "session"
      ? this.#credential.key
      : null;
  }

  /** Open the socket (and keep it open). */
  start(): void {
    this.#stopped = false;
    this.#connect();
  }

  /** Dial again now (after `disconnected`), with the credential kept. */
  retry(): void {
    if (this.#link.kind === "refused") {
      return;
    }
    if (this.#retry !== null) {
      clearTimeout(this.#retry);
      this.#retry = null;
    }
    this.#attempt = 0;
    this.#stopped = false;
    if (this.#socket === null) {
      this.#connect();
    }
  }

  /** Close for good. */
  stop(): void {
    this.#stopped = true;
    if (this.#retry !== null) {
      clearTimeout(this.#retry);
    }
    this.#socket?.close();
    this.#socket = null;
  }

  /** Send `frame`; false when there is no open, welcomed socket (before
   * the welcome only the hello may go). */
  send<M extends frames.Method>(
    frame: ClientFrame | frames.CallFrame<M>,
  ): boolean {
    const socket = this.#socket;
    if (socket?.readyState !== WebSocket.OPEN) {
      return false;
    }
    if (frame.type !== "hello" && this.#link.kind !== "open") {
      return false;
    }
    socket.send(JSON.stringify(frame));
    return true;
  }

  #setLink(link: LinkState): void {
    this.#link = link;
    this.#events.link(link);
  }

  #connect(): void {
    this.#setLink({ kind: "connecting", attempt: this.#attempt });
    const socket = new WebSocket(this.#url);
    this.#socket = socket;
    const handshake = new Handshake(this.#credential);
    let verified = false;
    let messages = Promise.resolve();
    // A handshake that never completes is a dropped connection, not a
    // refusal: try again, keeping the key.
    const deadline = setTimeout(() => {
      if (this.#socket === socket && !verified) {
        socket.close();
        this.#dropped(socket);
      }
    }, HANDSHAKE_MS);
    socket.addEventListener("message", (event: MessageEvent<unknown>) => {
      if (!this.#current(socket)) return;
      const data = event.data;
      messages = messages
        .then(async () => {
          if (!this.#current(socket)) return;
          let frame: ServerFrame;
          try {
            frame = frames.parse(data);
          } catch (error) {
            if (!verified) throw error;
            // After the server proved itself, a frame that is not JSON is
            // a broken connection, not a refusal: redial, keeping the key.
            socket.close();
            this.#dropped(socket);
            return;
          }
          if (verified) {
            if (frame.type === "challenge" || frame.type === "welcome")
              throw new Error("Repeated handshake");
            // Once the server proved itself, a frame this page fails to
            // handle (a store or reducer bug) is the page's fault, not a
            // refusal: report it and keep the connection and the key.
            try {
              this.#events.frame(frame);
            } catch (error) {
              console.error("vessel ui could not handle a frame", error);
            }
          } else if (frame.type === "challenge") {
            const hello = await handshake.hello(frame);
            if (this.#current(socket)) this.send(hello);
          } else if (frame.type === "welcome") {
            const credential = await handshake.welcome(frame);
            if (!this.#current(socket)) return;
            this.#credential = credential;
            rememberSession(credential);
            verified = true;
            clearTimeout(deadline);
            this.#attempt = 0;
            this.#setLink({ kind: "open", version: frame.version });
          } else {
            throw new Error("Server has not proved its identity");
          }
        })
        .catch(() => {
          // A failed handshake, or a repeated one: the key is dropped.
          if (this.#socket === socket) this.#refuse();
        });
    });
    socket.addEventListener("close", (event) => {
      clearTimeout(deadline);
      if (this.#socket !== socket) {
        return;
      }
      if (event.code === CLOSE_UNAUTHORIZED) {
        this.#socket = null;
        this.#events.lost();
        this.#setLink({ kind: "refused" });
        return;
      }
      this.#dropped(socket);
    });
  }

  /** `socket` went without a refusal: redial with backoff, or, after
   * `MAX_RETRIES`, stop and say so, keeping the credential. */
  #dropped(socket: WebSocket): void {
    if (this.#socket !== socket) {
      return;
    }
    this.#socket = null;
    this.#events.lost();
    if (this.#stopped) {
      return;
    }
    this.#attempt += 1;
    // A restarted process rejects this launch's old hostname at HTTP
    // upgrade, where browsers hide the response status: those failures
    // end here too, instead of redialling an obsolete tab forever.
    if (this.#attempt >= MAX_RETRIES) {
      this.#setLink({ kind: "disconnected", attempts: this.#attempt });
      return;
    }
    const retryInMs = Math.min(
      FIRST_RETRY_MS * 2 ** (this.#attempt - 1),
      MAX_RETRY_MS,
    );
    this.#setLink({ kind: "closed", retryInMs, attempt: this.#attempt });
    this.#retry = setTimeout(() => {
      this.#retry = null;
      this.#connect();
    }, retryInMs);
  }

  #current(socket: WebSocket): boolean {
    return this.#socket === socket && !this.#stopped;
  }

  #refuse(): void {
    this.stop();
    this.#events.lost();
    this.#setLink({ kind: "refused" });
  }
}
