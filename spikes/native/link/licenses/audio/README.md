# Desktop audio notices

These source notices accompany the selected desktop device/DSP dependency
closure. They do not cover local inference runtimes, downloaded speech
models or a claim of platform audio readiness. Archive/source byte inventory
and the reasons for the dependencies are in `docs/plans/wp-voice-audio-dependencies.md`.

Most notices come from the exact crate archives. `dasp_sample` 0.11.0 omitted
its license files, so its official repository commit
`97c3bb9b2363c0b46ac1633858bf1054fd02a980` supplies them. `sonora-aec3`
0.3.0 shares workspace commit `1632d47c1e77d0857f9145bd175134b348541e11`
with `sonora` 0.3.0; its omitted notice is supplied by that workspace's
published root-library LICENSE, including the WebRTC and packaging authors.
A release that ships these dependencies must include these notices.

| Dependency | Declared license | Notice files |
| --- | --- | --- |
| alsa 0.9.1 | Apache-2.0/MIT | [alsa-0.9.1-LICENSE-APACHE.txt](alsa-0.9.1-LICENSE-APACHE.txt), [alsa-0.9.1-LICENSE-MIT.txt](alsa-0.9.1-LICENSE-MIT.txt) |
| alsa-sys 0.3.1 | MIT | [alsa-sys-0.3.1-LICENSE.txt](alsa-sys-0.3.1-LICENSE.txt) |
| cpal 0.16.0 | Apache-2.0 | [cpal-0.16.0-LICENSE.txt](cpal-0.16.0-LICENSE.txt) |
| dasp_sample 0.11.0 | MIT OR Apache-2.0 | [dasp_sample-0.11.0-LICENSE-MIT.txt](dasp_sample-0.11.0-LICENSE-MIT.txt), [dasp_sample-0.11.0-LICENSE-APACHE.txt](dasp_sample-0.11.0-LICENSE-APACHE.txt) |
| sonora 0.3.0 | BSD-3-Clause | [sonora-0.3.0-LICENSE.txt](sonora-0.3.0-LICENSE.txt) |
| sonora-aec3 0.3.0 | BSD-3-Clause | [sonora-aec3-0.3.0-LICENSE.txt](sonora-aec3-0.3.0-LICENSE.txt) |
| sonora-agc2 0.3.0 | BSD-3-Clause | [sonora-agc2-0.3.0-LICENSE.txt](sonora-agc2-0.3.0-LICENSE.txt) |
| sonora-common-audio 0.3.0 | BSD-3-Clause | [sonora-common-audio-0.3.0-LICENSE.txt](sonora-common-audio-0.3.0-LICENSE.txt) |
| sonora-fft 0.3.0 | BSD-3-Clause | [sonora-fft-0.3.0-LICENSE.txt](sonora-fft-0.3.0-LICENSE.txt) |
| sonora-ns 0.3.0 | BSD-3-Clause | [sonora-ns-0.3.0-LICENSE.txt](sonora-ns-0.3.0-LICENSE.txt) |
| sonora-simd 0.3.0 | BSD-3-Clause | [sonora-simd-0.3.0-LICENSE.txt](sonora-simd-0.3.0-LICENSE.txt) |
| unicode-xid 0.2.6 | MIT OR Apache-2.0 | [unicode-xid-0.2.6-COPYRIGHT.txt](unicode-xid-0.2.6-COPYRIGHT.txt), [unicode-xid-0.2.6-LICENSE-APACHE.txt](unicode-xid-0.2.6-LICENSE-APACHE.txt), [unicode-xid-0.2.6-LICENSE-MIT.txt](unicode-xid-0.2.6-LICENSE-MIT.txt) |
| windows 0.54.0 | MIT OR Apache-2.0 | [windows-0.54.0-license-apache-2.0.txt](windows-0.54.0-license-apache-2.0.txt), [windows-0.54.0-license-mit.txt](windows-0.54.0-license-mit.txt) |
| windows-core 0.54.0 | MIT OR Apache-2.0 | [windows-core-0.54.0-license-apache-2.0.txt](windows-core-0.54.0-license-apache-2.0.txt), [windows-core-0.54.0-license-mit.txt](windows-core-0.54.0-license-mit.txt) |
