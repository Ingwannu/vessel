// [Decision Log]
// Background effects and key recorders must yield before a modal's first
// focus frame, as Paseo's overlay-first shortcut routing does. Reuse the
// kit's public role/open state rather than a second overlay registry or
// Escape listener, which could disagree with Base UI's nested/IME rules.
// Current kit dialogs, palette and sheets all expose this state; aria-modal
// alone is not present on every Base UI popup.
const OPEN_MODAL =
  '[role="dialog"][data-open], [role="alertdialog"][data-open]';

export function pageOwnsFocus(page: Document): boolean {
  return page.querySelector(OPEN_MODAL) === null;
}

/** Focus initiated by the page, never by an overlay's own controls. */
export function focusPage(target: HTMLElement | null): boolean {
  if (target === null || !pageOwnsFocus(target.ownerDocument)) return false;
  target.focus();
  return target.ownerDocument.activeElement === target;
}
