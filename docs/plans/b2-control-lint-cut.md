# B2 control lint cut

[Decision Log]
- Purpose: remove the four reported B2-owned strict lint findings without importing another producer's source.
- Choice: add semicolons to the input mouse-down, fullscreen close mouse-down and fullscreen submit-click unit callbacks; mark the existing SessionResidents::settle documentation reference as code.
- Impact: no control, state, callback signature, admission, assertion or IO behavior changes. No allowance, root/Cargo/protocol change or image-codec fixture is included.

This leaf is prepared on the exact clean common f6d78c454 snapshot. It contains only the three owned Rust files and this why-document. The source must be checked in the one common consumer union; this mechanical cut alone supplies no combined strict/runtime or package pass.
