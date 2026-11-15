//! The kit's glyphs drawn: one SVG per glyph, inline (split from
//! `assets.rs`, which keeps the inventory and the paths, at the file
//! budget, docs/QUALITY.md M3).
//!
//! A glyph added to [`Glyph`] gets its drawing here: the match below has
//! no catch-all, so a glyph without a drawing does not compile.

use super::{Glyph, MARK};

/// A line glyph on Lucide's grid (24 units, a stroke of 1.5, round ends),
/// as Paseo's icons are drawn. Original kit paths use this directly; the
/// four attributed Lucide paths below retain their licence in SVG metadata.
macro_rules! line_glyph {
    ($body:expr) => {
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="black" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">"#,
            $body,
            "</svg>"
        )
    };
}

// Retain the upstream notice in each distributed vector, not only the source
// checkout. CDATA preserves the licence text without interpreting it as SVG.
macro_rules! lucide_glyph {
    ($body:literal) => {
        line_glyph!(concat!(
            "<metadata><![CDATA[",
            include_str!("../../licenses/lucide.txt"),
            "]]></metadata>",
            $body,
        ))
    };
}

impl Glyph {
    /// Its drawing, as the asset source serves it.
    #[expect(clippy::too_many_lines, reason = "one arm per glyph, each its drawing")]
    pub(super) const fn svg(self) -> &'static str {
        match self {
            Self::Mark => MARK,
            Self::Minimize => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M0 5.5h10" stroke="black" stroke-width="1"/></svg>"#
            }
            Self::Maximize => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect x="0.5" y="0.5" width="9" height="9" fill="none" stroke="black" stroke-width="1"/></svg>"#
            }
            Self::Restore => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect x="0.5" y="2.5" width="7" height="7" fill="none" stroke="black" stroke-width="1"/><path d="M2.5 2.5V0.5h7v7h-2" fill="none" stroke="black" stroke-width="1"/></svg>"#
            }
            Self::Close => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M0.5 0.5l9 9M9.5 0.5l-9 9" stroke="black" stroke-width="1"/></svg>"#
            }
            Self::Update => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12"><path d="M6 1.5v6.5M3.2 5.4L6 8.2l2.8-2.8M1.5 10.5h9" fill="none" stroke="black" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/></svg>"#
            }
            Self::Menu => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12"><path d="M1.5 3h9M1.5 6h9M1.5 9h9" fill="none" stroke="black" stroke-width="1.2" stroke-linecap="round"/></svg>"#
            }
            Self::ChevronDown => line_glyph!(r#"<path d="M6 9l6 6 6-6"/>"#),
            Self::ChevronUp => line_glyph!(r#"<path d="M6 15l6-6 6 6"/>"#),
            Self::ChevronRight => line_glyph!(r#"<path d="M9 6l6 6-6 6"/>"#),
            Self::ChevronLeft => line_glyph!(r#"<path d="M15 6l-6 6 6 6"/>"#),
            Self::Check => line_glyph!(r#"<path d="M20 6L9 17l-5-5"/>"#),
            Self::Search => {
                line_glyph!(r#"<circle cx="11" cy="11" r="7"/><path d="M20 20l-3.6-3.6"/>"#)
            }
            Self::ArrowUp => line_glyph!(r#"<path d="M12 19V5M5 12l7-7 7 7"/>"#),
            // AppShell's LucideProvider supplies1.5; ExternalLink does not
            // override it. Keep the canonical line glyph's inherited stroke.
            Self::ArrowUpRight => line_glyph!(r#"<path d="M7 17 17 7M7 7h10v10"/>"#),
            Self::Stop => line_glyph!(r#"<rect x="6" y="6" width="12" height="12" rx="1.5"/>"#),
            Self::Shield => {
                line_glyph!(r#"<path d="M12 3l7 3v5.5c0 4.6-3 8.1-7 9.5-4-1.4-7-4.9-7-9.5V6z"/>"#)
            }
            // The shield's outline with its mark inside, centred on the
            // shield's optical middle (12, 11.5).
            Self::ShieldPlus => {
                line_glyph!(
                    r#"<path d="M12 3l7 3v5.5c0 4.6-3 8.1-7 9.5-4-1.4-7-4.9-7-9.5V6z"/><path d="M12 8.5v6M9 11.5h6"/>"#
                )
            }
            Self::ShieldCheck => {
                line_glyph!(
                    r#"<path d="M12 3l7 3v5.5c0 4.6-3 8.1-7 9.5-4-1.4-7-4.9-7-9.5V6z"/><path d="M9 11.5l2 2 4-4"/>"#
                )
            }
            Self::ShieldOff => {
                line_glyph!(
                    r#"<path d="M12 3l7 3v5.5c0 4.6-3 8.1-7 9.5-4-1.4-7-4.9-7-9.5V6z"/><path d="M4 4l16 16"/>"#
                )
            }
            Self::ShieldEllipsis => {
                line_glyph!(
                    r#"<path d="M12 3l7 3v5.5c0 4.6-3 8.1-7 9.5-4-1.4-7-4.9-7-9.5V6z"/><path d="M8.5 11.5h.01M12 11.5h.01M15.5 11.5h.01"/>"#
                )
            }
            Self::ShieldAlert => {
                line_glyph!(
                    r#"<path d="M12 3l7 3v5.5c0 4.6-3 8.1-7 9.5-4-1.4-7-4.9-7-9.5V6z"/><path d="M12 8v4M12 15h.01"/>"#
                )
            }
            Self::Brain => line_glyph!(
                r#"<path d="M12 5.5a3 3 0 0 0-5.6-1.2A3 3 0 0 0 4 8.5a3.2 3.2 0 0 0 .8 5.8A3 3 0 0 0 9 19a3 3 0 0 0 3-1.5z"/><path d="M12 5.5a3 3 0 0 1 5.6-1.2A3 3 0 0 1 20 8.5a3.2 3.2 0 0 1-.8 5.8A3 3 0 0 1 15 19a3 3 0 0 1-3-1.5"/>"#
            ),
            Self::Bot => line_glyph!(
                r#"<rect x="4.5" y="8" width="15" height="11" rx="2.5"/><path d="M12 8V4.5M9.5 12.5v1.5M14.5 12.5v1.5"/>"#
            ),
            Self::Alert => {
                line_glyph!(r#"<path d="M12 4l9 16H3z"/><path d="M12 10v4M12 17v.01"/>"#)
            }
            Self::CircleCheck => {
                line_glyph!(r#"<circle cx="12" cy="12" r="9"/><path d="M7.5 12l3 3 6-6"/>"#)
            }
            Self::CircleX => {
                line_glyph!(r#"<circle cx="12" cy="12" r="9"/><path d="M9 9l6 6M15 9l-6 6"/>"#)
            }
            Self::Circle => line_glyph!(r#"<circle cx="12" cy="12" r="9"/>"#),
            Self::CircleDot => {
                line_glyph!(r#"<circle cx="12" cy="12" r="9"/><circle cx="12" cy="12" r="1"/>"#)
            }
            Self::SquareCheck => line_glyph!(
                r#"<rect x="3.5" y="3.5" width="17" height="17" rx="2.5"/><path d="M8.5 12l2.5 2.5 4.5-5"/>"#
            ),
            // Own calendar/clock composition; the macro supplies the same
            // stroke, caps and joins as the neighbouring kit line drawings.
            Self::CalendarClock => line_glyph!(
                r#"<path d="M10 20H6a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2v3M8 3v4M16 3v4M4 10h16"/><circle cx="17" cy="17" r="4.5"/><path d="M17 14.5v2.75l1.75 1"/>"#
            ),
            Self::Folder => line_glyph!(
                r#"<path d="M3 7.5A2.5 2.5 0 0 1 5.5 5H9l2 2h7.5A2.5 2.5 0 0 1 21 9.5v7a2.5 2.5 0 0 1-2.5 2.5h-13A2.5 2.5 0 0 1 3 16.5z"/>"#
            ),
            // The kit's folder left open at its lower right for a branch:
            // two commits, the trunk and the branch joining it.
            Self::FolderGit => line_glyph!(
                r#"<path d="M10 19H5.5A2.5 2.5 0 0 1 3 16.5v-9A2.5 2.5 0 0 1 5.5 5H9l2 2h7.5A2.5 2.5 0 0 1 21 9.5V12"/><circle cx="14" cy="13" r="1.75"/><circle cx="19.25" cy="18.25" r="1.75"/><path d="M14 14.75V21M17.5 18.25a3.5 3.5 0 0 1-3.5-3.5"/>"#
            ),
            // Three isometric boxes, one resting on two; every edge stays
            // inside 2.25 to 21.75 so a 16-point glyph keeps a clear rim.
            Self::Boxes => line_glyph!(
                r#"<path d="M12 3l4 2.25v4.5L12 12l-4-2.25v-4.5zM8 12l4 2.25v4.5L8 21l-4-2.25v-4.5zM16 12l4 2.25v4.5L16 21l-4-2.25v-4.5z"/><path d="M8 5.25l4 2.25 4-2.25M12 7.5V12M4 14.25l4 2.25 4-2.25M8 16.5V21M12 14.25l4 2.25 4-2.25M16 16.5V21"/>"#
            ),
            Self::Machine => line_glyph!(
                r#"<rect x="3" y="4" width="18" height="12" rx="2"/><path d="M8 20h8M12 16v4"/>"#
            ),
            Self::More => line_glyph!(
                r#"<circle cx="5" cy="12" r="1"/><circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/>"#
            ),
            Self::Sliders => line_glyph!(
                r#"<path d="M19.5 7H10M14 17H4.5"/><circle cx="7" cy="7" r="2.75"/><circle cx="17" cy="17" r="2.75"/>"#
            ),
            Self::Gear => line_glyph!(
                r#"<circle cx="12" cy="12" r="3"/><path d="M12 2.75v2.5M12 18.75v2.5M2.75 12h2.5M18.75 12h2.5M5.46 5.46l1.77 1.77M16.77 16.77l1.77 1.77M5.46 18.54l1.77-1.77M16.77 7.23l1.77-1.77"/><circle cx="12" cy="12" r="6.25"/>"#
            ),
            Self::Pencil => line_glyph!(
                r#"<path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L8 18l-4 1 1-4z"/><path d="M14.5 5.5l3 3"/>"#
            ),
            Self::Remove => line_glyph!(r#"<path d="M18 6L6 18M6 6l12 12"/>"#),
            Self::Steer => {
                line_glyph!(r#"<path d="M15 14l5-5-5-5"/><path d="M4 20v-7a4 4 0 0 1 4-4h12"/>"#)
            }
            Self::Grip => line_glyph!(
                r#"<circle cx="9" cy="6" r="1"/><circle cx="15" cy="6" r="1"/><circle cx="9" cy="12" r="1"/><circle cx="15" cy="12" r="1"/><circle cx="9" cy="18" r="1"/><circle cx="15" cy="18" r="1"/>"#
            ),
            Self::Copy => line_glyph!(
                r#"<rect x="8" y="8" width="13" height="13" rx="2"/><path d="M16 8V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h3"/>"#
            ),
            Self::Palette => line_glyph!(
                r#"<path d="M12 3a9 9 0 1 0 0 18c1.1 0 2-.9 2-2 0-.5-.2-1-.5-1.3-.3-.4-.5-.8-.5-1.3 0-1.1.9-2 2-2h2.4a4.6 4.6 0 0 0 4.6-4.6C21 5.9 17 3 12 3z"/><circle cx="7.5" cy="11" r="1"/><circle cx="10" cy="7" r="1"/><circle cx="15" cy="7.5" r="1"/>"#
            ),
            // Original tubing/earpiece drawing in the kit's 24/1.5 line style.
            Self::Stethoscope => line_glyph!(
                r#"<path d="M5 3.75v5a4.25 4.25 0 0 0 8.5 0v-5M3.5 4.5h3M12 4.5h3M9.25 13v3a4.625 4.625 0 0 0 9.25 0v-2.25"/><circle cx="18.5" cy="11.5" r="2.25"/>"#
            ),
            Self::Keyboard => line_glyph!(
                r#"<rect x="2.5" y="6" width="19" height="12" rx="2"/><path d="M6.5 10h.01M10 10h.01M14 10h.01M17.5 10h.01M8 14h8"/>"#
            ),
            Self::Bell => line_glyph!(
                r#"<path d="M6 16v-5a6 6 0 0 1 12 0v5l1.5 2h-15z"/><path d="M10 20.5a2 2 0 0 0 4 0"/>"#
            ),
            Self::Sparkles => line_glyph!(
                r#"<path d="M12 3l1.8 5.2L19 10l-5.2 1.8L12 17l-1.8-5.2L5 10l5.2-1.8z"/><path d="M19 15v4M17 17h4"/>"#
            ),
            Self::Server => line_glyph!(
                r#"<rect x="3" y="4" width="18" height="7" rx="2"/><rect x="3" y="13" width="18" height="7" rx="2"/><path d="M7 7.5h.01M7 16.5h.01"/>"#
            ),
            Self::Smartphone => line_glyph!(
                r#"<rect x="6.5" y="2.5" width="11" height="19" rx="2.5"/><path d="M11 18h2"/>"#
            ),
            Self::Refresh => line_glyph!(
                r#"<path d="M20 11a8 8 0 0 0-14.5-4.5L4 8M4 4v4h4"/><path d="M4 13a8 8 0 0 0 14.5 4.5L20 16M20 20v-4h-4"/>"#
            ),
            Self::Info => {
                line_glyph!(r#"<circle cx="12" cy="12" r="9"/><path d="M12 11v5M12 8v.01"/>"#)
            }
            Self::ArrowLeft => line_glyph!(r#"<path d="M19 12H5M12 19l-7-7 7-7"/>"#),
            Self::Plus => line_glyph!(r#"<path d="M12 5v14M5 12h14"/>"#),
            Self::Branch => line_glyph!(
                r#"<path d="M6 3v12"/><circle cx="18" cy="6" r="3"/><circle cx="6" cy="18" r="3"/><path d="M18 9a9 9 0 0 1-9 9"/>"#
            ),
            Self::Lightning => {
                line_glyph!(r#"<path d="M13.25 2.75 4.75 13.5h6.75l-1 7.75 8.5-10.75h-6.75z"/>"#)
            }
            Self::LightningFilled => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="black" stroke="black" stroke-width="1.5" stroke-linejoin="round">"#,
                r#"<path d="M13.25 2.75 4.75 13.5h6.75l-1 7.75 8.5-10.75h-6.75z"/>"#,
                "</svg>"
            ),
            Self::Reset => line_glyph!(
                r#"<path d="M4.75 12a7.25 7.25 0 1 0 2.12-5.13L4.75 9"/><path d="M4.75 4.75V9H9"/>"#
            ),
            Self::MoreVertical => line_glyph!(
                r#"<circle cx="12" cy="5" r="1"/><circle cx="12" cy="12" r="1"/><circle cx="12" cy="19" r="1"/>"#
            ),
            Self::Pause => line_glyph!(
                r#"<rect x="6.5" y="4.5" width="3.5" height="15" rx="0.5"/><rect x="14" y="4.5" width="3.5" height="15" rx="0.5"/>"#
            ),
            Self::Play => line_glyph!(r#"<path d="M7.5 4.5 19.5 12 7.5 19.5z"/>"#),
            Self::RotateCw => {
                line_glyph!(r#"<path d="M20 10a8.25 8.25 0 1 0-1.1 6.5M20 4.5V10h-5.5"/>"#)
            }
            Self::ActivityIndicator => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32" fill="none" stroke="currentColor" stroke-width="4"><circle cx="16" cy="16" r="14" opacity="0.2"/><path d="M30 16A14 14 0 0 1 17.984442561589 29.858643069210"/></svg>"#
            }
            Self::ActivityTrack => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32" fill="none" stroke="currentColor" stroke-width="4"><circle cx="16" cy="16" r="14"/></svg>"#
            }
            Self::ActivityArc => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32" fill="none" stroke="currentColor" stroke-width="4"><path d="M30 16A14 14 0 0 1 17.984442561589 29.858643069210"/></svg>"#
            }
            Self::Spinner => {
                line_glyph!(r#"<path d="M12 3.75A8.25 8.25 0 1 1 3.75 12" stroke-width="2"/>"#)
            }
            Self::File => line_glyph!(
                r#"<path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/>"#
            ),
            Self::FileText => line_glyph!(
                r#"<path d="M6 22a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h8a2.4 2.4 0 0 1 1.704.706l3.588 3.588A2.4 2.4 0 0 1 20 8v12a2 2 0 0 1-2 2z"/><path d="M14 2v5a1 1 0 0 0 1 1h5"/><path d="M10 9H8"/><path d="M16 13H8"/><path d="M16 17H8"/>"#
            ),
            Self::PanelRight => line_glyph!(
                r#"<rect width="18" height="18" x="3" y="3" rx="2"/><path d="M15 3v18"/>"#
            ),
            Self::PanelLeft => line_glyph!(
                r#"<rect width="18" height="18" x="3" y="3" rx="2"/><path d="M9 3v18"/>"#
            ),
            // This is Paseo's compact View geometry, not a scaled 24-grid
            // line glyph: round filled bars keep the painted footprint exact.
            Self::MenuCompact => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 12" fill="black"><rect x="0" y="0" width="16" height="1.5" rx="0.75"/><rect x="0" y="5.25" width="16" height="1.5" rx="0.75"/><rect x="0" y="10.5" width="8" height="1.5" rx="0.75"/></svg>"#
            }
            Self::PullRequest => line_glyph!(
                r#"<circle cx="18" cy="18" r="3"/><circle cx="6" cy="6" r="3"/><path d="M13 6h3a2 2 0 0 1 2 2v7M6 9v12"/>"#
            ),
            Self::PullRequestDraft => line_glyph!(
                r#"<circle cx="18" cy="18" r="3"/><circle cx="6" cy="6" r="3"/><path d="M18 6V5M18 11v-1M6 9v12"/>"#
            ),
            Self::PullRequestClosed => line_glyph!(
                r#"<circle cx="6" cy="6" r="3"/><path d="M6 9v12M21 3l-6 6M21 9l-6-6M18 11.5V15"/><circle cx="18" cy="18" r="3"/>"#
            ),
            Self::GitMerge => line_glyph!(
                r#"<circle cx="18" cy="18" r="3"/><circle cx="6" cy="6" r="3"/><path d="M6 21V9a9 9 0 0 0 9 9"/>"#
            ),
            // The page's own drawings (`web/src/ui/kit/icon.css`, on a
            // 16 grid at a stroke of 1), at one and a half times.
            Self::Terminal => line_glyph!(r#"<path d="m6 7.5 4.5 4.5-4.5 4.5M12 16.5h6"/>"#),
            Self::Message => line_glyph!(r#"<path d="M4.5 6.75h15v10.5H9l-4.5 3v-13.5z"/>"#),
            Self::Archive => line_glyph!(
                r#"<rect x="3" y="4.5" width="18" height="5.25" rx="1.5"/><path d="M4.8 9.75v7.5A2.25 2.25 0 0 0 7.05 19.5h9.9a2.25 2.25 0 0 0 2.25-2.25v-7.5"/><path d="M9.75 13.95h4.5"/>"#
            ),
            Self::Trash => line_glyph!(
                r#"<path d="M3.75 6h16.5M18 6v13.5a2.25 2.25 0 0 1-2.25 2.25h-7.5A2.25 2.25 0 0 1 6 19.5V6M9 6V4.125A1.875 1.875 0 0 1 10.875 2.25h2.25A1.875 1.875 0 0 1 15 4.125V6"/>"#
            ),
            Self::Trash2 => line_glyph!(
                r#"<path d="M3.75 6h16.5M18 6v13.5a2.25 2.25 0 0 1-2.25 2.25h-7.5A2.25 2.25 0 0 1 6 19.5V6M9 6V4.125A1.875 1.875 0 0 1 10.875 2.25h2.25A1.875 1.875 0 0 1 15 4.125V6M9.75 9.5v8M14.25 9.5v8"/>"#
            ),
            Self::ArchiveRestore => line_glyph!(
                r#"<rect x="3" y="4.5" width="18" height="5.25" rx="1.5"/><path d="M4.8 9.75v7.5A2.25 2.25 0 0 0 7.05 19.5h9.9a2.25 2.25 0 0 0 2.25-2.25v-7.5M12 17.25v-5.5M8.75 15 12 11.75 15.25 15"/>"#
            ),
            Self::Minus => line_glyph!(r#"<path d="M5.25 12h13.5"/>"#),
            Self::GitBranchPlus => line_glyph!(
                r#"<path d="M6 3v12"/><circle cx="18" cy="6" r="3"/><circle cx="6" cy="18" r="3"/><path d="M18 9a9 9 0 0 1-9 9M15 18h6M18 15v6"/>"#
            ),
            // Reuse the git track's original drawing, rather than a second Commit.
            Self::Commit => {
                line_glyph!(r#"<circle cx="12" cy="12" r="4"/><path d="M3 12h5M16 12h5"/>"#)
            }
            Self::Upload => line_glyph!(
                r#"<path d="M12 16.5V3.75M7.5 8.25 12 3.75 16.5 8.25M3.75 15.75v3a1.5 1.5 0 0 0 1.5 1.5h13.5a1.5 1.5 0 0 0 1.5-1.5v-3"/>"#
            ),
            Self::Download => line_glyph!(
                r#"<path d="M12 3.75v12M7.5 11.25 12 15.75 16.5 11.25M3.75 15.75v3a1.5 1.5 0 0 0 1.5 1.5h13.5a1.5 1.5 0 0 0 1.5-1.5v-3"/>"#
            ),
            Self::Undo2 => {
                line_glyph!(r#"<path d="M9 5.5 4.5 10 9 14.5M5 10h8a6 6 0 0 1 6 6v3"/>"#)
            }
            // Lucide's `split`, Paseo's fork mark.
            Self::Split => line_glyph!(
                r#"<path d="M16 3h5v5M8 3H3v5M12 21v-8.3a4 4 0 0 0-1.17-2.88L3.5 3.5M15 9l5.5-5.5"/>"#
            ),
            Self::Pin => line_glyph!(
                r#"<path d="M8.25 3.75h7.5"/><path d="M9.75 3.75v6L6 13.5v2.25h12V13.5l-3.75-3.75v-6"/><path d="M12 15.75V21"/>"#
            ),
            Self::Board => line_glyph!(
                r#"<rect x="3" y="3.75" width="5.25" height="16.5" rx="1.5"/><rect x="9.375" y="3.75" width="5.25" height="10.5" rx="1.5"/><rect x="15.75" y="3.75" width="5.25" height="13.5" rx="1.5"/>"#
            ),
            Self::Sun => line_glyph!(
                r#"<circle cx="12" cy="12" r="4.125"/><path d="M12 2.25v1.875M12 19.875v1.875M2.25 12h1.875M19.875 12h1.875M5.1 5.1l1.35 1.35M17.55 17.55l1.35 1.35M5.1 18.9l1.35-1.35M17.55 6.45l1.35-1.35"/>"#
            ),
            Self::Moon => line_glyph!(
                r#"<path d="M20.25 14.4A8.25 8.25 0 0 1 9.6 3.75a8.25 8.25 0 1 0 10.65 10.65Z"/>"#
            ),
            Self::Monitor => line_glyph!(
                r#"<rect x="3" y="4.125" width="18" height="12.75" rx="2.25"/><path d="M8.25 21h7.5M12 16.875V21"/>"#
            ),
            Self::Gauge => line_glyph!(
                r#"<path d="M4.75 18.5a9 9 0 1 1 14.5 0"/><path d="m12 13.5 4.5-4.5"/>"#
            ),
            Self::PackagePlus => line_glyph!(
                r#"<path d="M12 3.25 3.5 7.75 12 12.25l8.5-4.5L12 3.25M3.5 7.75v9L12 21.25l3-1.6M12 12.25v9M20.5 7.75v4.5M7.75 5.5l8.5 4.5M19 14.75v6.5M15.75 18h6.5"/>"#
            ),
            // Lucide source, adapted only to the shared stroke. Immutable
            // revision and ISC notice: kit/licenses/lucide.txt.
            Self::Wrench => lucide_glyph!(
                r#"<path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.106-3.105c.32-.322.863-.22.983.218a6 6 0 0 1-8.259 7.057l-7.91 7.91a1 1 0 0 1-2.999-3l7.91-7.91a6 6 0 0 1 7.057-8.259c.438.12.54.662.219.984z"/>"#
            ),
            Self::Eye => lucide_glyph!(
                r#"<path d="M2.062 12.348a1 1 0 0 1 0-.696 10.75 10.75 0 0 1 19.876 0 1 1 0 0 1 0 .696 10.75 10.75 0 0 1-19.876 0"/><circle cx="12" cy="12" r="3"/>"#
            ),
            Self::MicVocal => lucide_glyph!(
                r#"<path d="m11 7.601-5.994 8.19a1 1 0 0 0 .1 1.298l.817.818a1 1 0 0 0 1.314.087L15.09 12"/><path d="M16.5 21.174C15.5 20.5 14.372 20 13 20c-2.058 0-3.928 2.356-6 2-2.072-.356-2.775-3.369-1.5-4.5"/><circle cx="16" cy="7" r="5"/>"#
            ),
            Self::Scissors => lucide_glyph!(
                r#"<circle cx="6" cy="6" r="3"/><path d="M8.12 8.12 12 12"/><path d="M20 4 8.12 15.88"/><circle cx="6" cy="18" r="3"/><path d="M14.8 14.8 20 20"/>"#
            ),
        }
    }
}
