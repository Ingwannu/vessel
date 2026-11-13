// A branch name checked as it is typed, with git's own rules
// (`check-ref-format --branch`, the contract's `BranchName`), so a field
// says what is wrong before anything is sent. The node checks again; this
// only saves a round trip and says it in words.

/** The longest name the node takes, in bytes. */
const MAX_BYTES = 255;

/** Why `name` is not a branch name git takes (`null`: it is). */
export function branchNameProblem(name: string): string | null {
  if (name === "") {
    return "A branch needs a name.";
  }
  if (new TextEncoder().encode(name).length > MAX_BYTES) {
    return "That name is too long.";
  }
  if (name === "@" || name === "HEAD") {
    return `“${name}” is not a branch name git takes.`;
  }
  if (name.startsWith("-")) {
    return "A branch name cannot start with a dash.";
  }
  for (const char of name) {
    const code = char.codePointAt(0) ?? 0;
    if (code <= 0x20 || code === 0x7f) {
      return "A branch name cannot hold spaces or control characters.";
    }
    if ("~^:?*[\\".includes(char)) {
      return `A branch name cannot hold “${char}”.`;
    }
  }
  if (name.includes("..") || name.includes("@{")) {
    return `A branch name cannot hold “${name.includes("..") ? ".." : "@{"}”.`;
  }
  if (name.startsWith("/") || name.endsWith("/") || name.includes("//")) {
    return "A branch name cannot start or end with “/”, or hold two in a row.";
  }
  if (name.endsWith(".")) {
    return "A branch name cannot end with a dot.";
  }
  for (const part of name.split("/")) {
    if (part.startsWith(".")) {
      return "No part of a branch name can start with a dot.";
    }
    if (part.endsWith(".lock")) {
      return "No part of a branch name can end with “.lock”.";
    }
  }
  return null;
}
