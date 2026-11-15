// Toasts (DESIGN.md §6.10): below the header, clear of the lower composer
// and dock. Success leaves after 4 s; errors are dismissed.

import { Store, useStore } from "../data/store";
import { IconButton } from "./kit";
import { useToastClearance } from "./use-toast-clearance";

interface Toast {
  id: number;
  tone: "ok" | "error";
  text: string;
}

const toasts = new Store<readonly Toast[]>([]);
let next = 1;

export function toast(text: string, tone: "ok" | "error" = "ok"): void {
  const id = next;
  next += 1;
  toasts.update((list) => [...list, { id, tone, text }]);
  if (tone === "ok") {
    setTimeout(() => {
      dismiss(id);
    }, 4000);
  }
}

function dismiss(id: number): void {
  toasts.update((list) => list.filter((item) => item.id !== id));
}

export function Toasts() {
  const list = useStore(toasts, (value) => value);
  const host = useToastClearance(list.length > 0);
  return (
    <div ref={host} className="toast-stack">
      {list.map((item) => (
        <div
          key={item.id}
          className={item.tone === "error" ? "toast error" : "toast"}
          role={item.tone === "error" ? "alert" : "status"}
        >
          <span>{item.text}</span>
          <IconButton
            label="Dismiss"
            icon="close"
            size="xs"
            onClick={() => {
              dismiss(item.id);
            }}
          />
        </div>
      ))}
    </div>
  );
}
