// The pace of a page following a node's long reads (setup's output, a
// clone's progress): each read asks the node to wait for something new,
// but the node answers at once when it cannot wait (a call forwarded by
// another node, or all its waits taken), and a loop that asked again
// straight away would ask back to back for as long as setup or the clone
// runs. A read that comes back sooner than `QUIET_MS` with nothing new
// waits out the rest of it first.

/** The least time between two reads that brought nothing new. */
export const QUIET_MS = 1000;

/** Whether `signal` stopped a walk: read through a call, since a walk
 * asks again after every wait and a property read would be taken as
 * known from the first one. */
export function halted(signal: AbortSignal | null): boolean {
  return signal?.aborted ?? false;
}

/** A pause of `ms`, cut short by `signal`. */
export function pause(ms: number, signal: AbortSignal | null): Promise<void> {
  return new Promise((resolve) => {
    const timer = setTimeout(done, ms);
    signal?.addEventListener("abort", done, { once: true });
    function done() {
      clearTimeout(timer);
      signal?.removeEventListener("abort", done);
      resolve();
    }
  });
}

/** After a read asked at `asked` (`performance.now()`) that brought
 * nothing new: wait out the rest of `QUIET_MS`. */
export function quiet(
  asked: number,
  signal: AbortSignal | null,
): Promise<void> {
  const left = QUIET_MS - (performance.now() - asked);
  return left > 0 ? pause(left, signal) : Promise.resolve();
}
