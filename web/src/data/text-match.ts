// Ranked text matching, Paseo's `search/text-match.ts` (pinned 0.15.4)
// without its typo tier: a match is a tier plus the offset it was found
// at, lower being better on both.
//
// [Decision Log]
// - Purpose: Add project's directory rows keep the machine's recent
//   project roots that match the field as Paseo's do
//   (`working-directory-suggestions.ts` over `scorePathMatch`).
// - Constraint: the page had no matcher; the native app's is
//   `view::matcher` (the same reference).
// - Choice: Paseo's exact tiers and its in-word subsequence, and
//   `scorePathMatch`'s compact retry; its opt-in typo tier is left out, as
//   no caller here turns it on.
// - Cost: a caller that wants typo tolerance adds that tier from the same
//   reference.

/** A match: its tier (lower is better), where it starts, and for a
 * subsequence how far it spreads. */
export interface MatchScore {
  tier: number;
  offset: number;
  spread?: number;
}

const TIER_EXACT = 0;
const TIER_WHOLE_WORD = 1;
const TIER_PREFIX = 2;
const TIER_WORD_START = 3;
const TIER_SUBSTRING = 4;
const TIER_SUBSEQUENCE = 5;

const WORD = /[a-z0-9]/u;
const WORD_ANY_CASE = /[a-z0-9]/iu;
const SPACE = /\s/u;

function isWordBoundary(character: string): boolean {
  return character === "" || !WORD.test(character);
}

function scoreSubstring(query: string, text: string): MatchScore | null {
  let best: MatchScore | null = null;
  let from = 0;
  while (from <= text.length - query.length) {
    const found = text.indexOf(query, from);
    if (found === -1) break;
    const startsAtBoundary =
      found === 0 || isWordBoundary(text.charAt(found - 1));
    const endsAtBoundary = isWordBoundary(text.charAt(found + query.length));
    let tier: number;
    if (startsAtBoundary && endsAtBoundary) {
      tier = TIER_WHOLE_WORD;
    } else if (found === 0) {
      tier = TIER_PREFIX;
    } else if (startsAtBoundary) {
      tier = TIER_WORD_START;
    } else {
      tier = TIER_SUBSTRING;
    }
    if (
      best === null ||
      tier < best.tier ||
      (tier === best.tier && found < best.offset)
    ) {
      best = { tier, offset: found };
    }
    from = found + 1;
  }
  return best;
}

/** The query's characters in order within one whitespace-free word. */
function scoreSubsequence(query: string, text: string): MatchScore | null {
  let matched = 0;
  let first = -1;
  let last = -1;
  for (
    let index = 0;
    index < text.length && matched < query.length;
    index += 1
  ) {
    const character = text.charAt(index);
    if (SPACE.test(character)) {
      matched = 0;
      first = -1;
      last = -1;
      continue;
    }
    if (character !== query.charAt(matched)) continue;
    if (first === -1) first = index;
    last = index;
    matched += 1;
  }
  if (matched !== query.length || first === -1) return null;
  return { tier: TIER_SUBSEQUENCE, offset: first, spread: last - first + 1 };
}

/** How well `query` matches `text`, ignoring case; `null` for no match. */
export function scoreMatch(query: string, text: string): MatchScore | null {
  if (query === "") return { tier: TIER_EXACT, offset: 0 };
  const q = query.toLowerCase();
  const t = text.toLowerCase();
  if (t === q) return { tier: TIER_EXACT, offset: 0 };
  return scoreSubstring(q, t) ?? scoreSubsequence(q, t);
}

interface CompactText {
  value: string;
  offsets: number[];
}

/** Letters and digits only, lower-cased, with each one's place in `value`. */
function compactText(value: string): CompactText {
  let compact = "";
  const offsets: number[] = [];
  for (let index = 0; index < value.length; index += 1) {
    const character = value.charAt(index);
    if (!WORD_ANY_CASE.test(character)) continue;
    compact += character.toLowerCase();
    offsets.push(index);
  }
  return { value: compact, offsets };
}

/** Match a query against a whole displayed path, separators included;
 * else both without their separators (`blank page` finds `blankpage/`). */
export function scorePathMatch(query: string, path: string): MatchScore | null {
  const direct = scoreMatch(query, path);
  if (direct !== null) return direct;
  const compactQuery = compactText(query);
  const compactPath = compactText(path);
  if (compactQuery.value === "" || compactPath.value === "") return null;
  const compact = scoreMatch(compactQuery.value, compactPath.value);
  if (compact === null) return null;
  return {
    ...compact,
    offset: compactPath.offsets[compact.offset] ?? compact.offset,
  };
}
