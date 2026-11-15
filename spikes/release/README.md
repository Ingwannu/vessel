# Releases, installers and updates (milestone 12, track R)

A workspace of its own: the crates the native app (`spikes/native`) links
for its updates and its log, the release tools, and the patched GPUI crates
(`patches/`, not members; `patches/README.md`). What the track built and
why is in `docs/plans/backend-m12-release.md`.

## Crates, and what may depend on what

| Crate | Owns | Must not | Depends on |
| --- | --- | --- | --- |
| `hello` (`vessel-release-hello`) | the smallest GPUI window, a target's first proof | be linked into the app | - |
| `updater` (`vessel-app-updater`) | the app's updates: the signed feed read, a build fetched, verified and swapped in | use GPUI (the app links it on every desktop) | - |
| `feed` (`vessel-app-feed`) | the release key, the signed feed and the sums, for whoever releases | be linked into the app | `vessel-app-updater` |
| `applog` (`vessel-app-log`) | the app's log: stderr and a rotated file in the platform's place for logs | use GPUI | - |

The table is checked: `node ../../scripts/check-boundaries.mjs --workspace .`
compares the last column with every crate's normal and build dependencies
on `vessel-*` crates, in both directions, and holds the table below; it
builds nothing (`cargo metadata --no-deps`).

### External crates with one owner

| Crate | Who may depend on it | Why |
| --- | --- | --- |
| `minisign` | only `vessel-app-feed` | Signing stays with the release tool; the app only verifies. |
| `minisign-verify` | only `vessel-app-updater` | One place checks a feed's signature. |
| `gpui-pre` | only `vessel-release-hello` | The updater and the log stay free of the UI layer the app may replace. |
| `gpui-pre-platform` | only `vessel-release-hello` | As above. |
| `tracing-subscriber` | only `vessel-app-log` | Where the app's log goes is decided in one crate. |

## Gate

Run from `spikes/release` with its pinned toolchain, the whole workspace:

```bash
cd spikes/release
cargo fmt --all --check
node ../../scripts/check-rust-size.mjs --workspace .     # file budget
node ../../scripts/check-boundaries.mjs --workspace .    # the tables above
cargo clippy --all-targets -- -D warnings
cargo test
```

The file budget is the main workspace's (`docs/QUALITY.md`, M3): a file of
product code over 500 lines fails, and this workspace has none
(2026-11-14: the largest is `updater/src/manifest.rs`, 449 lines), so it
keeps no list of exceptions.
