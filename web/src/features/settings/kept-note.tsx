// Where the bridge keeps what Settings changes (ADR 0028), said only when
// it is not a file: then a choice lasts until vessel ui stops, and the
// page says so rather than pretend.

import { useStore } from "../../data/store";
import { keptNote, prefs } from "../../data/prefs";
import { Banner } from "../../ui/kit";

export function KeptNote() {
  const kept = useStore(prefs, (state) => state.kept);
  const note = keptNote(kept);
  return note === null ? null : <Banner>{note}</Banner>;
}
