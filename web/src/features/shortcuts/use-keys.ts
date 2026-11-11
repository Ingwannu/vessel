// An action's first key, as the platform writes it, for the hints beside
// the controls that do the same (the sidebar's search, New session): they
// follow a rebinding instead of saying the default.

import { prefs } from "../../data/prefs";
import { bindingWords } from "../../data/shortcuts/keys";
import { currentPlatform } from "../../data/shortcuts/platform";
import { bindingsOf, type ShortcutAction } from "../../data/shortcuts/registry";
import { useStore } from "../../data/store";

/** `action`'s first key in words ("⌘K", "Ctrl+K"); `null` when unset. */
export function useShortcutWords(action: ShortcutAction): string | null {
  const overrides = useStore(prefs, (state) => state.values.shortcuts);
  const [first] = bindingsOf(action, overrides);
  return first === undefined ? null : bindingWords(first, currentPlatform());
}
