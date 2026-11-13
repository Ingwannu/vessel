// The owner's mark, read from docs/design/brand/vessel-mark.svg by the
// build (vite-mark.ts): its view box and paths, drawn by the sidebar.
declare module "virtual:vessel-mark" {
  export const mark: {
    readonly viewBox: string;
    readonly paths: readonly {
      readonly d: string;
      readonly evenOdd: boolean;
    }[];
  };
}
