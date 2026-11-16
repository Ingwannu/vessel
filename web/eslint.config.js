// Lint rules for the web UI: TypeScript's strictest type-checked set,
// no `any` anywhere, and no raw HTML (ADR 0017: session content is
// untrusted; React escapes text and Markdown renders with raw HTML off).
import js from "@eslint/js";
import { defineConfig } from "eslint/config";
import reactHooks from "eslint-plugin-react-hooks";
import globals from "globals";
import tseslint from "typescript-eslint";

/** Base UI is reached through the kit only (ADR 0021). */
const BASE_UI = {
  regex: "^@base-ui/",
  message:
    "Controls come from the kit (src/ui/kit), not from Base UI (ADR 0021).",
};

/** The one helper that brands a handle without checking its shape
 * (src/testing/ids.ts) is for tests and their fixtures; the page's
 * handles are branded by src/bridge/ids.ts alone. */
const TESTING = {
  regex: "^(\\.{1,2}/)+testing(/|$)",
  message:
    "src/testing is for tests and their fixtures; the page brands a handle with src/bridge/ids.ts.",
};

/** Tests and the fixtures they share. */
const TESTS = ["**/*.test.ts", "**/*.test.tsx", "src/data/fixtures/**"];

/** `no-restricted-imports` for the files `files`: none of `below` (the
 * layers it may not reach, by their first path segment under src/), and
 * outside tests nothing of src/testing. A block replaces the rule's
 * options of an earlier one, so each repeats the Base UI pattern, except
 * the kit's, which is where Base UI lives. */
function layer(files, below) {
  const segments = below.join("|");
  const kit = files === "src/ui/kit/**";
  const patterns = [
    ...(kit ? [] : [BASE_UI]),
    {
      regex: `^(\\.{1,2}/)+(ui/)?(${segments})(/|$)`,
      message: `${files.replace("/**", "")} may not import ${below.join(", ")} (one direction: wire <- bridge <- data <- features <- routes; the kit imports none of them).`,
    },
  ];
  return [
    {
      files: [files],
      rules: { "no-restricted-imports": ["error", { patterns }] },
    },
    {
      files: [files],
      ignores: TESTS,
      rules: {
        "no-restricted-imports": [
          "error",
          { patterns: [...patterns, TESTING] },
        ],
      },
    },
  ];
}

export default defineConfig(
  {
    // A rule is not switched off in a comment: a file that needs an
    // exception gets it here, with its reason.
    linterOptions: {
      noInlineConfig: true,
      reportUnusedDisableDirectives: "error",
    },
  },
  {
    ignores: [
      "dist/",
      "artifacts/",
      "test-results/",
      "playwright-report/",
      // Generated from the Rust types; checked by `tsc` and CI's drift check.
      "src/wire/proto.ts",
      "src/wire/bridge.ts",
    ],
  },
  js.configs.recommended,
  tseslint.configs.strictTypeChecked,
  tseslint.configs.stylisticTypeChecked,
  {
    languageOptions: {
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
      globals: globals.browser,
    },
  },
  reactHooks.configs.flat["recommended-latest"],
  {
    rules: {
      "@typescript-eslint/no-explicit-any": "error",
      "@typescript-eslint/switch-exhaustiveness-check": "error",
      "@typescript-eslint/restrict-template-expressions": [
        "error",
        { allowNumber: true },
      ],
      "no-restricted-syntax": [
        "error",
        {
          selector: "JSXAttribute[name.name='dangerouslySetInnerHTML']",
          message:
            "Session content is untrusted: render text, or Markdown with raw HTML off.",
        },
      ],
      // Types held as strictly as the compiler can (the owner's word, in
      // TypeScript as in Rust): no assertion but `as const`, a value
      // from the DOM or the wire is narrowed by a function that checks it;
      // no `!`; conditions on booleans and nullable objects only, so a
      // string or a number is compared explicitly.
      "@typescript-eslint/consistent-type-assertions": [
        "error",
        { assertionStyle: "never" },
      ],
      "@typescript-eslint/no-unsafe-type-assertion": "error",
      "@typescript-eslint/no-non-null-assertion": "error",
      "@typescript-eslint/strict-boolean-expressions": [
        "error",
        {
          allowString: false,
          allowNumber: false,
          allowNullableObject: true,
          allowNullableBoolean: false,
          allowNullableString: false,
          allowNullableNumber: false,
          allowNullableEnum: false,
          allowAny: false,
        },
      ],
      "@typescript-eslint/prefer-readonly": "error",
      "@typescript-eslint/no-unnecessary-condition": "error",
      "@typescript-eslint/no-unnecessary-type-parameters": "error",
      "@typescript-eslint/method-signature-style": "error",
      "@typescript-eslint/ban-ts-comment": [
        "error",
        {
          "ts-expect-error": "allow-with-description",
          "ts-ignore": true,
          "ts-nocheck": true,
        },
      ],
    },
  },
  // Logic modules state their boundary types; components (.tsx) are
  // typed by their props.
  {
    files: ["**/*.ts"],
    rules: {
      "@typescript-eslint/explicit-module-boundary-types": "error",
    },
  },
  // The page's layers, one direction (docs/plans/backend-m10.md,
  // "Structure"): wire <- bridge <- data <- features <- routes. The kit
  // imports none of them, and nothing but the kit imports Base UI.
  {
    files: ["src/**/*.ts", "src/**/*.tsx"],
    rules: { "no-restricted-imports": ["error", { patterns: [BASE_UI] }] },
  },
  {
    files: ["src/**/*.ts", "src/**/*.tsx"],
    ignores: TESTS,
    rules: {
      "no-restricted-imports": ["error", { patterns: [BASE_UI, TESTING] }],
    },
  },
  ...layer("src/wire/**", [
    "bridge",
    "data",
    "features",
    "routes",
    "ui",
    "context",
    "app",
  ]),
  ...layer("src/bridge/**", [
    "data",
    "features",
    "routes",
    "ui",
    "context",
    "app",
  ]),
  // The one shared presentation value must remain a dependency-free leaf,
  // rather than making data import the kit or the kit import data.
  ...layer("src/quota-percent.ts", [
    "wire",
    "bridge",
    "data",
    "features",
    "routes",
    "ui",
    "context",
    "app",
  ]),
  ...layer("src/data/**", ["features", "routes", "ui", "context", "app"]),
  ...layer("src/features/**", ["routes", "bridge", "app"]),
  ...layer("src/ui/kit/**", [
    "wire",
    "bridge",
    "data",
    "features",
    "routes",
    "context",
    "app",
    "shell",
    "parts",
    "toasts",
    "styles",
  ]),
  // xterm.js keeps the one hook this file needs on its private core; the
  // assertion there is the boundary the plan allows (see the file).
  {
    files: ["src/data/terminal-input.ts"],
    rules: {
      "@typescript-eslint/consistent-type-assertions": "off",
      "@typescript-eslint/no-unsafe-type-assertion": "off",
    },
  },
  {
    files: ["**/*.js", "**/*.mjs"],
    extends: [tseslint.configs.disableTypeChecked],
    languageOptions: { globals: globals.node },
  },
);
