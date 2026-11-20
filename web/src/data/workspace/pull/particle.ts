// Korean particles chosen from a value, for the pane's Korean sentences
// that name a forge's host ("github.com이 …"): the native localization's
// `PARTICLE` function (view/src/text/localization/grammar.rs), which the
// Fluent catalog calls as `{ $host }{ PARTICLE($host, "subject") }`.
//
// The particle follows the value, not the label in today's screenshot.
// Latin text uses the Korean names of its final letter (L/M/N/R end in a
// consonant); a word's pronunciation is never guessed. Hangul uses its
// real final consonant, digits their Sino-Korean readings (0 영, 1 일, …).
// Other scripts and an empty value take the open form; a final ㄹ is
// special only for the direction particle.

/** The grammatical role a particle plays. */
export type ParticleRole =
  "object" | "subject" | "topic" | "with" | "direction";

/** How the value's last sound ends. */
type Ending = "open" | "consonant" | "rieul";

/** Each role's form after each ending. */
const FORMS: Readonly<Record<ParticleRole, Readonly<Record<Ending, string>>>> =
  {
    object: { open: "를", consonant: "을", rieul: "을" },
    subject: { open: "가", consonant: "이", rieul: "이" },
    topic: { open: "는", consonant: "은", rieul: "은" },
    with: { open: "와", consonant: "과", rieul: "과" },
    direction: { open: "로", consonant: "으로", rieul: "로" },
  };

/** The value's last character that is alphanumeric as Rust's
 * `char::is_alphanumeric` reads it (the Alphabetic property, or a number:
 * Nd, Nl, No); punctuation and spaces after it are skipped. The `u` flag
 * makes a character a code point, as a Rust `char` is. */
const LAST_ALPHANUMERIC =
  /[\p{Alphabetic}\p{Nd}\p{Nl}\p{No}](?=[^\p{Alphabetic}\p{Nd}\p{Nl}\p{No}]*$)/u;

/** Letters and digits whose Korean name ends in ㄹ (일, 칠, 팔, 엘, 알). */
const RIEUL: ReadonlySet<string> = new Set(["1", "7", "8", "l", "L", "r", "R"]);

/** Letters and digits whose Korean name ends in another consonant
 * (영, 삼, 육, 엠, 엔). */
const CONSONANT: ReadonlySet<string> = new Set([
  "0",
  "3",
  "6",
  "m",
  "M",
  "n",
  "N",
]);

/** The first Hangul syllable, and how many there are after it. */
const SYLLABLES = 0xac00;
const SYLLABLE_SPAN = 11171;

function ending(value: string): Ending {
  const last = LAST_ALPHANUMERIC.exec(value)?.[0];
  if (last === undefined) {
    return "open";
  }
  const point = last.codePointAt(0) ?? 0;
  const syllable = point - SYLLABLES;
  if (syllable >= 0 && syllable <= SYLLABLE_SPAN) {
    // A syllable's final consonant is its index modulo 28: none, or ㄹ (8).
    const final = syllable % 28;
    if (final === 0) {
      return "open";
    }
    return final === 8 ? "rieul" : "consonant";
  }
  // A decomposed Hangul value ends in a final jamo, not a syllable.
  if (point === 0x11af) {
    return "rieul";
  }
  if (point >= 0x11a8 && point <= 0x11ff) {
    return "consonant";
  }
  // A compatibility jamo alone: ㄹ, or another of ㄱ…ㅎ.
  if (RIEUL.has(last) || point === 0x3139) {
    return "rieul";
  }
  if (CONSONANT.has(last) || (point >= 0x3131 && point <= 0x314e)) {
    return "consonant";
  }
  return "open";
}

/** The particle for `role` after `value`. */
export function particle(value: string, role: ParticleRole): string {
  return FORMS[role][ending(value)];
}
