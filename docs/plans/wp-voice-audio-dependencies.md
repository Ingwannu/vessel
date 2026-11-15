# Desktop audio dependency inventory (ADR 0080)

Recorded 2026-11-15 from the exact crates.io archives and a separate
metadata-only resolver for Linux x86_64 and Windows x86_64 MSVC. No library
or executable was run for this inventory. This is source and archive size,
not linked size, end-device support or measured echo-cancellation quality.
The actual native lockfile diff must be checked against this candidate
closure when manifests resolve.

## Decision Log

- Purpose and intent: one native device owner for dictation/conversation,
  with playback referenced by acoustic echo cancellation.
- Existing assets and constraints: the app has no audio engine. std cannot
  open ALSA/WASAPI or provide AEC; the existing GPUI stack does neither.
  The node's provider/credential ownership stays separate.
- Alternatives: browser-engine audio adds another UI/runtime and permission
  origin; helper-process capture does not share a reverse-stream reference;
  handwritten OS/DSP FFI would violate the unsafe-code rule.
- Choice: adopt exact CPAL 0.16.0 and Sonora 0.3.0 in link on Linux/Windows
  only. Default CPAL ALSA/WASAPI backends, no ASIO/JACK/priority options;
  Sonora library only, no example, FFI or optional dependencies.
- Why these: CPAL's inspected safe callback API and narrow backend set can
  be pinned without bringing unreviewed newer native-backend changes;
  Sonora supplies a safe Rust WebRTC audio-processing pipeline instead of
  adding our own C++ bindings or sample-rate conversion implementation.
- Advantages, costs and effects: one bounded device thread, explicit
  unsupported-format failures, added native source/build dependencies and
  DSP CPU cost. Linux requires ALSA development files from the build image;
  an installed pkg-config ALSA 1.2.11 was verified in development. No system
  package or permission change was made. Linked size and Windows hardware
  behaviour remain to be measured before release readiness is claimed.

## New packages relative to the app base lock

Already-present dependencies (including bytemuck, derive_more, libc,
bitflags, cfg-if, pkg-config and the platform support crates) are not
recounted here. Optional/example dependencies are excluded.

| Package | Declared license | Archive bytes | Unpacked source bytes | Targets |
| --- | --- | ---: | ---: | --- |
| alsa 0.9.1 | Apache-2.0/MIT | 72824 | 442440 | linux |
| alsa-sys 0.3.1 | MIT | 32748 | 283749 | linux |
| cpal 0.16.0 | Apache-2.0 | 104465 | 506821 | linux, windows |
| dasp_sample 0.11.0 | MIT OR Apache-2.0 | 11617 | 63873 | linux, windows |
| sonora 0.3.0 | BSD-3-Clause | 111649 | 565551 | linux, windows |
| sonora-aec3 0.3.0 | BSD-3-Clause | 130120 | 660828 | linux, windows |
| sonora-agc2 0.3.0 | BSD-3-Clause | 7516103 | 10892185 | linux, windows |
| sonora-common-audio 0.3.0 | BSD-3-Clause | 19846 | 86239 | linux, windows |
| sonora-fft 0.3.0 | BSD-3-Clause | 37338 | 201165 | linux, windows |
| sonora-ns 0.3.0 | BSD-3-Clause | 27607 | 118698 | linux, windows |
| sonora-simd 0.3.0 | BSD-3-Clause | 13828 | 92580 | linux, windows |
| unicode-xid 0.2.6 | MIT OR Apache-2.0 | 15744 | 76872 | linux, windows |
| windows 0.54.0 | MIT OR Apache-2.0 | 11096265 | 146499830 | windows |
| windows-core 0.54.0 | MIT OR Apache-2.0 | 44015 | 246877 | windows |

CPAL: <https://github.com/RustAudio/cpal>.
Sonora: <https://github.com/dignifiedquire/sonora>.
The exact cargo archive checksums remain in the generated native lockfile.
Redistribution notices are kept under `spikes/native/link/licenses/audio/`.

Local sherpa-onnx/ONNX Runtime/model distribution is deliberately separate:
no inference crate or model is selected or approved by this inventory.
The inspected candidate native archives did not include notices, and the
newer inference API's phonemization differs from the reference's older
Kokoro packaging. Those require a further reviewed ADR amendment.


## Actual lockfile and initial format floor

The native lock adds 34 package/version entries: the compiled-target
closure above plus inactive Android/Apple/other-target support entries
resolved by CPAL's own manifest (including jni/ndk, coreaudio/objc2 and
older windows-sys/targets). The link's CPAL/Sonora dependencies themselves
remain conditional on Linux/Windows; this does not enable a device backend
on mobile, browser or macOS. Those inactive target entries are not a claim
of platform support or shipped native binaries.

The initial DSP wrapper accepts mono/stereo and rates divisible by 100 Hz
within 8–192 kHz, so every 10 ms block has an exact integer frame count.
Other formats, including 22050 Hz, are explicit refusals until a fractional
frame adapter exists; they are not rounded into a drifting playback rate.
This is a stated incomplete-format difference from Paseo's broader PCM
resampler. Input and output devices must select a supported configuration;
no optional model is advertised compatible without that conversion path.
A separate unfiltered rate converter feeds the stable device-rate reverse
stream, so cue/TTS rate changes do not reinitialize the voice AEC.
