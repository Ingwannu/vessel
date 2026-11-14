// Whether an element is within a screen of its scroller's view: one
// `IntersectionObserver` per scroller for every element that asks, not
// one per element (a long timeline draws dozens of rows with images).

type Near = () => void;

interface Watch {
  observer: IntersectionObserver;
  callbacks: Map<Element, Near>;
}

const watches = new Map<Element | null, Watch>();

/** Call `near` once, when `element` comes within a screen of `root`'s
 * view (`null`: the window's); the returned function stops watching. */
export function whenNear(
  root: Element | null,
  element: Element,
  near: Near,
): () => void {
  let watch = watches.get(root);
  if (watch === undefined) {
    const callbacks = new Map<Element, Near>();
    const observer = new IntersectionObserver(
      (seen) => {
        for (const entry of seen) {
          const callback = callbacks.get(entry.target);
          if (entry.isIntersecting && callback !== undefined) {
            callbacks.delete(entry.target);
            observer.unobserve(entry.target);
            callback();
          }
        }
      },
      { root, rootMargin: "100% 0px" },
    );
    watch = { observer, callbacks };
    watches.set(root, watch);
  }
  const current = watch;
  current.callbacks.set(element, near);
  current.observer.observe(element);
  return () => {
    current.callbacks.delete(element);
    current.observer.unobserve(element);
    if (current.callbacks.size === 0) {
      current.observer.disconnect();
      watches.delete(root);
    }
  };
}
