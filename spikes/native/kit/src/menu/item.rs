//! Item data and paint: a pending label and spinner are one typed state.
use crate::{assets::Glyph, popup::Surfaces};
use gpui::{Hsla, SharedString};

/// A menu row's semantic tone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// An ordinary action.
    Plain,
    /// A destructive action, separated from ordinary ones.
    Danger,
}

/// The caller's reference glyph size; the leading slot remains sixteen points.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LeadingSize {
    /// Schedule menu glyphs, fourteen points (the existing default).
    #[default]
    Small,
    /// Git action glyphs, sixteen points.
    Regular,
}
impl LeadingSize {
    pub(super) const fn points(self) -> f32 {
        match self {
            Self::Small => 14.0,
            Self::Regular => 16.0,
        }
    }
}

/// Idle presentation, independent of input authority or feedback status.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Presentation {
    /// Ordinary source text/opacity.
    #[default]
    Normal,
    /// A quiet but selectable action; the caller may explain availability.
    Muted,
}

/// Pending and acknowledged success carry complete translated labels and refuse input.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ItemStatus {
    /// Ready unless explicitly disabled.
    #[default]
    Idle,
    /// Spinner replaces the leading glyph and these words replace the label.
    Pending(SharedString),
    /// Acknowledged success, with complete caller-localized words.
    Success(SharedString),
}

/// Whether activation dismisses the surface before invoking its caller.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Dismissal {
    /// Legacy/default behaviour, including actions that open another surface.
    #[default]
    Close,
    /// Direct writes keep pending/result feedback in the same open menu.
    KeepOpen,
}

/// One menu row. Existing constructors preserve their old defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuItem {
    /// Idle words, provided by the caller's language table.
    pub label: SharedString,
    /// Ordinary or destructive colour.
    pub tone: Tone,
    /// Quiet idle presentation; disabled/feedback states take precedence.
    pub presentation: Presentation,
    /// Visible but not chosen.
    pub disabled: bool,
    /// A full-width separator above this item.
    pub starts_group: bool,
    /// A kit glyph inside the unchanged 16-point leading slot.
    pub leading: Option<Glyph>,
    /// Ordinary glyph size only; pending and success retain their own size.
    pub leading_size: LeadingSize,
    /// Typed feedback; pending/success stay non-selectable even when disabled is false.
    pub status: ItemStatus,
    /// Default Close; direct writes opt into staying on the same surface.
    pub dismissal: Dismissal,
    /// The page this row leads to (a submenu, [`crate::submenu`]): choosing
    /// it opens the page, never closes the menu and never reaches the caller.
    pub opens: Option<crate::submenu::PageId>,
    /// The current value of that page's decision, muted before the chevron
    /// (Paseo's `MenuSubTrigger` `value`: "Grouping  Project ›").
    pub value: Option<SharedString>,
}
impl MenuItem {
    /// An ordinary item, with no leading glyph or pending state.
    #[must_use]
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            tone: Tone::Plain,
            presentation: Presentation::Normal,
            disabled: false,
            starts_group: false,
            leading: None,
            leading_size: LeadingSize::Small,
            status: ItemStatus::Idle,
            dismissal: Dismissal::Close,
            opens: None,
            value: None,
        }
    }
    /// A row that opens `page` beside the menu (Paseo's `MenuSubTrigger`).
    #[must_use]
    pub fn sub(label: impl Into<SharedString>, page: impl Into<crate::submenu::PageId>) -> Self {
        Self {
            opens: Some(page.into()),
            ..Self::new(label)
        }
    }
    /// The decision's current value, shown before the chevron.
    #[must_use]
    pub fn with_value(self, value: impl Into<SharedString>) -> Self {
        Self {
            value: Some(value.into()),
            ..self
        }
    }
    /// A destructive item starting a group.
    #[must_use]
    pub fn danger(label: impl Into<SharedString>) -> Self {
        Self {
            tone: Tone::Danger,
            starts_group: true,
            ..Self::new(label)
        }
    }
    /// Keep this item's other properties and start a group.
    #[must_use]
    pub fn grouped(self) -> Self {
        Self {
            starts_group: true,
            ..self
        }
    }
    /// Set the leading glyph; pending temporarily replaces it with the spinner.
    #[must_use]
    pub fn with_leading(self, glyph: Glyph) -> Self {
        self.with_leading_at(glyph, LeadingSize::Small)
    }
    /// Opt into the source's glyph size without scaling the row or leading slot.
    #[must_use]
    pub fn with_leading_at(self, glyph: Glyph, leading_size: LeadingSize) -> Self {
        Self {
            leading: Some(glyph),
            leading_size,
            ..self
        }
    }
    /// Quiet idle text/opacity without disabling pointer or keyboard input.
    #[must_use]
    pub fn with_presentation(self, presentation: Presentation) -> Self {
        Self {
            presentation,
            ..self
        }
    }
    /// Keep all presentation/state and choose the activation policy.
    #[must_use]
    pub fn with_dismissal(self, dismissal: Dismissal) -> Self {
        Self { dismissal, ..self }
    }
    /// An in-progress action, with complete caller-localized words.
    #[must_use]
    pub fn pending(self, label: impl Into<SharedString>) -> Self {
        Self {
            status: ItemStatus::Pending(label.into()),
            ..self
        }
    }
    /// An acknowledged result; timing/reset stays in the caller's view model.
    #[must_use]
    pub fn success(self, label: impl Into<SharedString>) -> Self {
        Self {
            status: ItemStatus::Success(label.into()),
            ..self
        }
    }
    /// Both pointer and keyboard paths use this guard, including Enter on a stale highlight.
    #[must_use]
    pub fn selectable(&self) -> bool {
        !self.disabled && matches!(self.status, ItemStatus::Idle)
    }
    pub(super) fn words(&self) -> &SharedString {
        match &self.status {
            ItemStatus::Idle => &self.label,
            ItemStatus::Pending(label) | ItemStatus::Success(label) => label,
        }
    }
    pub(super) fn leading(&self) -> Leading {
        match &self.status {
            ItemStatus::Pending(_) => Leading::Spinner,
            ItemStatus::Success(_) => Leading::SuccessCheck,
            ItemStatus::Idle => self.leading.map_or(Leading::None, Leading::Glyph),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Leading {
    None,
    Glyph(Glyph),
    Spinner,
    SuccessCheck,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Paint {
    pub ink: Hsla,
    pub glyph: Hsla,
    pub fill: Hsla,
    pub opacity: f32,
}
pub(super) fn paint(item: &MenuItem, highlighted: bool, surfaces: &Surfaces) -> Paint {
    let (ink, glyph) = match item.tone {
        Tone::Plain => (surfaces.foreground, surfaces.foreground_muted),
        Tone::Danger => (surfaces.destructive, surfaces.destructive),
    };
    let (ink, glyph) = match &item.status {
        ItemStatus::Idle => (ink, glyph),
        ItemStatus::Pending(_) => (ink, surfaces.foreground_muted),
        ItemStatus::Success(_) => {
            let green = gpui::rgb(crate::theme::palette::GREEN_500).into();
            (green, green)
        }
    };
    let selectable = item.selectable();
    let (ink, opacity) = match (selectable, item.presentation) {
        (false, Presentation::Normal | Presentation::Muted) => (ink, 0.5),
        (true, Presentation::Normal) => (ink, 1.0),
        (true, Presentation::Muted) => (surfaces.foreground_muted, 0.72),
    };
    Paint {
        ink,
        glyph,
        fill: if highlighted && selectable {
            surfaces.surface2
        } else {
            gpui::transparent_black()
        },
        opacity,
    }
}
