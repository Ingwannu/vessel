// A draft's async completion may navigate only while its screen remains.
// Initial focus is optional: a late-loaded page must leave an open modal
// alone, without postponing its draft lifetime until that modal closes.
import { type RefObject, useEffect, useRef } from "react";

import { focusPage } from "../../ui/page-focus";

export function useDraftPresence(
  text: RefObject<HTMLTextAreaElement | null>,
): RefObject<boolean> {
  const here = useRef(true);
  useEffect(() => {
    here.current = true;
    focusPage(text.current);
    return () => {
      here.current = false;
    };
  }, [text]);
  return here;
}
