//! Editor marks for Open in editor (TRACKER 467, ADR 0123), embedded from
//! the design folder's files (`docs/design/editor-marks/`, by path, never
//! copied). One mark per editor, the same on light and dark surfaces, as
//! Paseo ships one icon per editor: Simple Icons 16.38.0 path data
//! (CC0-1.0) on an app tile where Paseo draws a tile (Zed, Cursor, Android
//! Studio), `WebStorm`'s square glyph, Xcode's glyph, and VS Code's icon,
//! Microsoft's own file, unchanged (Microsoft allows no background). The
//! sources, hashes, colours and terms are in that folder's README. The
//! colours are the artwork's, not the interface's, so they stay in the
//! files and out of the kit's tokens.

/// Cursor's cube on its tile.
pub(super) const CURSOR: &str = include_str!("../../../../../docs/design/editor-marks/cursor.svg");
/// Zed's mark on its tile.
pub(super) const ZED: &str = include_str!("../../../../../docs/design/editor-marks/zed.svg");
/// Android Studio's mark on its tile.
pub(super) const ANDROID_STUDIO: &str =
    include_str!("../../../../../docs/design/editor-marks/android-studio.svg");
/// Xcode's mark.
pub(super) const XCODE: &str = include_str!("../../../../../docs/design/editor-marks/xcode.svg");
/// `WebStorm`'s square mark.
pub(super) const WEBSTORM: &str =
    include_str!("../../../../../docs/design/editor-marks/webstorm.svg");
/// Microsoft's Visual Studio Code icon (`vscode.svg` of
/// `visual-studio-code-icons.zip`, 2021-06-21), byte for byte (Microsoft:
/// not to be modified, redrawn or given a background).
pub(super) const VSCODE: &str = include_str!("../../../../../docs/design/editor-marks/vscode.svg");
