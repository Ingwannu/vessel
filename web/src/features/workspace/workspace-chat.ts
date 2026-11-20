// Code references go to the existing draft store. The host reveals its
// composer after the action, because a workspace shows one pane at a time.

import { createContext } from "react";

export const WorkspaceChat = createContext<(() => void) | null>(null);
