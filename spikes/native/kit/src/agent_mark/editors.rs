//! Editor marks for Open in editor (TRACKER 467, ADR 0123), embedded from
//! the design folder's files (`docs/design/editor-marks/`, by path, never
//! copied): Simple Icons 16.38.0 path data (CC0-1.0), each filled in its
//! brand colour where that colour reads on the scheme's menu surface (3:1
//! or more) and in the scheme's black or white otherwise; and VS Code's
//! icon, Microsoft's own file, unchanged. Sources, hashes, the colours
//! chosen and the terms are in that folder's README. The colours are the
//! artwork's, not the interface's, so they stay in the files and out of
//! the kit's tokens.

/// Cursor's cube on light surfaces.
pub(super) const CURSOR_LIGHT: &str =
    include_str!("../../../../../docs/design/editor-marks/cursor-light.svg");
/// Cursor's cube on dark surfaces.
pub(super) const CURSOR_DARK: &str =
    include_str!("../../../../../docs/design/editor-marks/cursor-dark.svg");
/// Zed's mark on light surfaces.
pub(super) const ZED_LIGHT: &str =
    include_str!("../../../../../docs/design/editor-marks/zed-light.svg");
/// Zed's mark on dark surfaces.
pub(super) const ZED_DARK: &str =
    include_str!("../../../../../docs/design/editor-marks/zed-dark.svg");
/// Android Studio's mark on light surfaces.
pub(super) const ANDROID_STUDIO_LIGHT: &str =
    include_str!("../../../../../docs/design/editor-marks/android-studio-light.svg");
/// Android Studio's mark on dark surfaces.
pub(super) const ANDROID_STUDIO_DARK: &str =
    include_str!("../../../../../docs/design/editor-marks/android-studio-dark.svg");
/// Xcode's mark, the same on both.
pub(super) const XCODE: &str = include_str!("../../../../../docs/design/editor-marks/xcode.svg");
/// `WebStorm`'s mark on light surfaces.
pub(super) const WEBSTORM_LIGHT: &str =
    include_str!("../../../../../docs/design/editor-marks/webstorm-light.svg");
/// `WebStorm`'s mark on dark surfaces.
pub(super) const WEBSTORM_DARK: &str =
    include_str!("../../../../../docs/design/editor-marks/webstorm-dark.svg");
/// Microsoft's Visual Studio Code icon (`vscode.svg` of
/// `visual-studio-code-icons.zip`, 2021-06-21), byte for byte; the same in
/// both schemes (Microsoft: not to be modified or redrawn).
pub(super) const VSCODE: &str = include_str!("../../../../../docs/design/editor-marks/vscode.svg");
