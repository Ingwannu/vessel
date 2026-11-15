# ADR 0099: A release is compiled without secrets, and the feed's address is written into the built app

Status: decided by the coordinator, 2026-11-15 (milestone 12, track R),
option (b) of three. Built on `backend/m12-release-adr0094`. No label is
built the old way after 2026-11-15. The first label after this lands is the
first built this way. The release key, the feed's address and its token do
not change, and published apps keep updating as before.

## Purpose and intent

The operator ruled on 2026-11-15 that no compiler runs on the machine that
holds the release secrets. That machine is the development machine: it
holds the feed's token and the signing key. Every compile, test, gate and
capture build goes to the build VM, and the build VM keeps no secret. Until
then a label was compiled on the development machine with the feed's
address compiled into the app (`option_env!("VESSEL_UPDATE_FEED")`). The
address carries the feed's token. A label now has to be compiled on the
build VM, where the token may not go, and still ship an app that knows its
feed.

## Existing implementation and constraints

- `vessel-app-updater` read four settings with `option_env!`: the label,
  the feed's address, the release key's public half and the channel. Its
  `build.rs` refused values that do not parse.
- `spikes/release/build.sh` compiled the app for Windows (cargo-xwin, fxc
  under wine) and for Linux, and made the zip, the installer, the tarball,
  the gzip copies and the sums. It signed with the feed tool through
  `cargo run`.
- Constraints:
  - The address and the token are never printed, logged or committed.
  - Only track R writes the dist directory.
  - A published label is never rebuilt.
  - The repository is public and names no host (ADR 0014).
  - Apps already published (0.0.0.0.3 to 0.0.0.0.20) must keep updating,
    so the feed, the manifest and the key stay as they are.

## Alternatives considered

1. **(a) Labels stay on the development machine, as the one exception.**
   Refused by the operator: no compiler on the machine with the secrets.
2. **(c) Put the token on the build VM.** Refused: the build VM keeps no
   secret. It runs every track's code, and a gate run there may read any
   file its account can.
3. **Read the address at run time from a file beside the app.** That means
   a second file to install, update and protect from other users. An app
   that loses the file loses its feed silently, and apps already
   published read no such file.
4. **(b) Compile with a fixed placeholder, then write the address into
   those bytes of the built executable on the machine that signs.**
   Chosen.

## Decision

- **The slot.** `updater/src/config.rs` holds a static
  `[u8; FEED_SLOT_LEN]`, 512 bytes. While nothing is written in, it holds
  a marker: `VESSEL-UPDATE-FEED-SLOT/ADR-0099/UNPATCHED/` and then `#` to
  the end. The marker has no NUL byte.
  - A written slot is the address, then NUL to the end, with at least one
    NUL. That allows addresses up to 511 bytes; the real one is 83.
  - At run time the slot is read through `std::hint::black_box(&FEED_SLOT)`.
    The compiler therefore cannot assume the slot still holds the marker it
    compiled, and cannot fold the reading into a constant.
  - A slot with no NUL is unwritten.
  - A slot with text after the padding, no text, or bytes that are not
    UTF-8 is refused.
  - The written address goes through the app's own `FeedUrl::parse`.
  - A build with a label and a release key compiled in but an unwritten
    slot reports `FeedNotWritten` and never updates.
- **The old variable is refused.** The updater's `build.rs` refuses
  `VESSEL_UPDATE_FEED` outright, and so does `build.sh`. The token can
  then not reach a compiler by habit.
- **The release key's public half is committed** as
  `spikes/release/update-key.pub`. It is public by design: it is in every
  shipped app and verifies only. The build VM gets nothing from outside to
  compile a release, and the commit alone decides what a build trusts.
  The package step checks that the signature on the sums verifies with
  this key before anything is published.
- **`build.sh` runs as two steps:**
  - `--compile-only DIR`, on the build VM.
    - Refuses to run if `VESSEL_UPDATE_FEED_FILE` or
      `VESSEL_UPDATE_SECRET` is set.
    - Compiles the app with the key, the label and the channel, and builds
      the feed tool.
    - Splits the Linux debug information off and keeps the Windows `.pdb`.
    - Makes the icons and gathers the licences.
    - Checks that the slot occurs exactly once in each app.
    - Writes `compile.txt` (label, targets, commit, `package_node`,
      channel, key, `links_updater`, slot size, `rustc`) and `SHA256SUMS`.
    - DIR is written beside itself and renamed into place at the end, so
      it exists only whole.
  - `--package-from DIR`, on the signing machine, with no compiler.
    - Checks DIR against its sums.
    - Checks that the label and targets match the request and that the
      commit is this checkout's `HEAD`, with `LICENSE.md` and
      `spikes/release` unchanged from it.
    - Proves the slot on the Linux build: see "Checks and tests".
    - Writes the address into copies of the executables. The compile
      step's files are never changed.
    - Checks the Windows imports and resources.
    - Sets the modes the published sets have always had: directories
      0775, files 0664, programs 0775, and the Linux update file 0755.
      DIR's files arrive with whatever mode the transfer gave them (the
      remote gate exports 0500).
    - Packages, gzips, verifies every shipped app, and runs the shipped
      Linux app against the real feed.
    - Signs the sums and checks them with the compiled-in key.
    - Moves the set into place, and with `--publish` signs and checks the
      manifest. It signs and publishes with `DIR/tools/vessel-app-feed`,
      the feed tool compiled from the same commit.
  - Without either flag, both steps run here in turn: the same code with
    the hand-off in a scratch directory.
- **The patch tool** is `spikes/release/tools/patch-feed.py`. It is
  Python, so it is not compilation, and it uses the standard library only.
  - `check FILE...`: the unwritten slot occurs exactly once in each file.
  - `write --address-file F IN OUT` checks before and after writing:
    - Before writing:
      - The address is checked as the app checks it: `https://` (or
        `http://` to this machine), no query, fragment or credentials, at
        most 511 bytes, printable ASCII.
      - The whole 512-byte marker occurs exactly once in IN.
      - OUT does not exist.
    - It writes a temporary file beside OUT with IN's mode, syncs it, and
      reads it back.
    - Before the rename:
      - The size is unchanged.
      - Every byte outside the slot equals IN's.
      - The slot is the address and NUL padding.
      - The address and padding occur exactly once, and no marker is left.
    - IN is never changed.
  - `verify --address-file F FILE...`: the written slot occurs exactly
    once and no marker is left.
  - The tool never prints the address and never puts it in an error.
    Exceptions it did not foresee are reported by kind only, so their text
    cannot quote the address. Its messages name offsets, lengths and
    SHA-256 sums.

## Checks and tests

- `tools/test_patch_feed.py`, 13 unittest cases, all through the command
  line:
  - The address lands in the slot and nothing else moves; the mode is
    kept.
  - No slot or two slots is refused, and nothing is written.
  - A header without its fill is not a slot.
  - An existing target is refused.
  - 511 bytes fit and 512 do not.
  - Addresses the app refuses are refused.
  - Test feeds on this machine are allowed.
  - CRLF is handled.
  - `check` and `verify` work, including after a byte past the padding is
    changed.
  - Writing twice is refused.
  - A missing address file gives no traceback.
  - The tool's constants equal `config.rs`'s, which the test reads from
    the source.
  - Every run's output is searched for the address and its token.
- `updater/src/config.rs` unit tests:
  - The unwritten slot is the marker and holds no NUL.
  - An unwritten slot means no feed.
  - Written slots give their address, the longest included.
  - A slot written twice, an empty slot and non-UTF-8 bytes are refused.
  - A written address goes through `FeedUrl::parse`.
- `tools/feed-slot-proof.sh` runs in every package step:
  - **`test`**, before the real address is written anywhere. It patches a
    copy of the compiled Linux app with the address of a feed served on
    127.0.0.1 for that run only, runs the copy under its own Xvfb, and
    passes when the copy asks that feed for `<channel>.json`.
  - **`real`**, after packaging. It runs a copy of the shipped Linux app
    and passes when the app's log says the real feed's signed manifest
    was read and nothing is newer. It also fails if the app's log holds
    the address or any long part of its path.
  - Both modes run from a private scratch directory and stop only the
    processes they started.
- **Dry run on 2026-11-15:** `--package-from` on a stand-in compile output.
  - The stand-ins: a Windows exe with no feed, with the marker added, and
    a script that reads its own slot. A throwaway key and a loopback feed
    were used.
  - The full step passed: both proofs, write, verify, signed sums.
  - 13 refusal cases each stopped with their own message and placed
    nothing:
    - a changed file
    - another label
    - another commit
    - no address file
    - the old variable
    - two slots
    - an app that never asks its feed
    - a signing key the app does not carry (it stops at the signature
      check, before the set is placed)
    - an address the app refuses
    - the compile step given the secret
    - the compile step given the address
    - an existing compile directory
    - both steps at once
- **The first real label** is gated as every label is (the coordinator's
  smoke test). The self-update from the label before it shows on a real
  install that the patched app updates.

## Why this and not the alternatives

The operator's two rules leave (b) as the only option that keeps one app
binary. It needs no new file on the owner's machine and no change to the
feed. The token stays on one machine, and is written only into the
executables that ship, as before. The build VM compiles every release from
the commit alone.

## Advantages, disadvantages and impact

- **Advantages:**
  - The token is never on the build VM, in a compiler's environment, in a
    log, or in the compile step's output.
  - The signing machine runs no compiler.
  - The release key the app trusts is fixed by the commit, and the
    package step refuses to sign with any other.
  - The feed tool that signs comes from the same commit.
- **Disadvantages:**
  - `black_box` is documented as best effort. The `test` proof is what
    guards it, and it covers the Linux build only. The Windows app stops
    under wine at its first text. Both builds come from the same source
    and the same compiler, and the first self-update of a Windows install
    after a label shows it there.
  - The executables that ship differ from the compiled ones by the 512
    slot bytes. The `.pdb` and `.debug` still match, because neither
    holds the slot's contents.
  - A label takes the round trip to the build VM, and its compile output
    is about 0.5 GB.
- **Impact:**
  - The build VM needs the Windows cross tools. Track R installed them on
    2026-11-15 (wine, llvm, lld, clang, imagemagick, cargo-xwin, the xwin
    cache, fxc). They are recorded with the VM's provisioning notes.
  - The remote gate's release compile mode, owned by the build-tools
    track, runs `--compile-only` for a commit and brings DIR back to a new
    folder.
  - Apps already published are unaffected.
