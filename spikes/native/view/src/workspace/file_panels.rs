//! Paseo's file-panels/resolve.ts: one pure choice rule for tab chrome,
//! panel picker and preferences. Plugin identity stays generic so this
//! owner consumes W16's canonical address without a parallel registry.

use std::collections::BTreeMap;
use vessel_proto::workspace::RelPath;

/// A plugin catalogue cannot cause an unbounded picker or matching pass.
pub const MAX_FILE_PANELS: usize = 128;

/// Classification comes from a read/stat, never an extension alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    /// Complete UTF-8 source.
    Text,
    /// Decodable supported image bytes.
    Image,
    /// Opaque bytes with no ordinary source view.
    Binary,
}

/// Built-in renderer identity, separate from the file's tab/buffer identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BuiltinPanel {
    /// Markdown owns its Source/Preview switch.
    Markdown,
    /// HTML owns its Source/Preview switch.
    Html,
    /// Ordinary image preview (SVG is a separate bounded renderer below).
    Image,
    /// Pure Rust PDF viewer, offered only after its renderer is registered.
    Pdf,
    /// Pure Rust SVG viewer, offered only after its renderer is registered.
    Svg,
    /// Text fallback; suppressed beside a built-in rendering panel.
    Source,
}

/// The plugin parameter is W16's validated contribution address in the app.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum PanelIdentity<P> {
    /// A renderer owned by this package.
    Builtin(BuiltinPanel),
    /// A plugin-owned contribution, including its installation generation.
    Plugin(P),
}

/// Borrowed projections of the producer's validated declaration types.
/// Matching is literal: filenames are case-sensitive, extensions fold case.
#[derive(Debug, Clone, Copy)]
pub enum FileSelector<'a> {
    /// Every file of the candidate's allowed kind.
    All,
    /// Literal extension and filename claims, never wildcard matching.
    Names {
        /// Extensions without a leading dot.
        extensions: &'a [&'a str],
        /// Exact basenames, including dotfiles.
        filenames: &'a [&'a str],
    },
}

/// One renderer already registered by its owner; resolution never launches it.
#[derive(Debug, Clone)]
pub struct PanelCandidate<'a, P> {
    /// Stable renderer/contribution identity.
    pub identity: PanelIdentity<P>,
    /// Which names it claims.
    pub selector: FileSelector<'a>,
    /// Built-ins constrain kind; plugins may claim any kind with `None`.
    pub kind: Option<FileKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum KeyValue {
    Extension(String),
    Filename(String),
}

/// An association key derived only from a validated path and matching claims.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SelectorKey(KeyValue);

/// Which association namespace this key belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectorKind {
    /// All files with this lowercased extension.
    Extension,
    /// This exact case-sensitive basename.
    Filename,
}

impl SelectorKey {
    /// Discriminates exact filename associations from extension associations.
    #[must_use]
    pub const fn kind(&self) -> SelectorKind {
        match self.0 {
            KeyValue::Extension(_) => SelectorKind::Extension,
            KeyValue::Filename(_) => SelectorKind::Filename,
        }
    }
    /// The filename or folded extension, for settings presentation.
    #[must_use]
    pub fn value(&self) -> &str {
        match &self.0 {
            KeyValue::Extension(value) | KeyValue::Filename(value) => value,
        }
    }
}

/// Resolved choices borrow the catalogue; no second renderer instance is made.
pub struct PanelResolution<'a, P> {
    /// The explicit tab choice, valid association, or first candidate.
    pub selected: Option<&'a PanelCandidate<'a, P>>,
    /// Offered choices in producer order, excluding an inapplicable Source floor.
    pub candidates: Vec<&'a PanelCandidate<'a, P>>,
    /// The one key used by the picker and device preference store.
    pub selector: SelectorKey,
    /// A remembered renderer only when that renderer is currently a candidate.
    pub associated: Option<&'a PanelCandidate<'a, P>>,
}

impl<P> PanelResolution<'_, P> {
    /// Built-in-only choices never force a modal picker.
    #[must_use]
    pub fn asks_picker(&self) -> bool {
        self.candidates.len() > 1
            && self
                .candidates
                .iter()
                .any(|candidate| matches!(candidate.identity, PanelIdentity::Plugin(_)))
    }
}

/// Refuse excessive catalogue work before allocating picker state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("at most {MAX_FILE_PANELS} file panels may be resolved")]
pub struct PanelCatalogueFull;

fn names(path: &RelPath) -> (&str, String) {
    let name = path.file_name().unwrap_or_default();
    let extension = name
        .rsplit_once('.')
        .map_or("", |(_, extension)| extension)
        .to_lowercase();
    (name, extension)
}

fn matches(selector: FileSelector<'_>, name: &str, extension: &str) -> bool {
    match selector {
        FileSelector::All => true,
        FileSelector::Names {
            extensions,
            filenames,
        } => {
            filenames.contains(&name)
                || extensions
                    .iter()
                    .any(|candidate| candidate.to_lowercase() == extension)
        }
    }
}

/// Resolve exactly one registered catalogue against real source classification.
///
/// # Errors
/// [`PanelCatalogueFull`] before processing an oversized plugin catalogue.
pub fn resolve<'a, P: Eq>(
    path: &RelPath,
    kind: FileKind,
    catalogue: &'a [PanelCandidate<'a, P>],
    associations: &BTreeMap<SelectorKey, PanelIdentity<P>>,
    choice: Option<&PanelIdentity<P>>,
) -> Result<PanelResolution<'a, P>, PanelCatalogueFull> {
    if catalogue.len() > MAX_FILE_PANELS {
        return Err(PanelCatalogueFull);
    }
    let (name, extension) = names(path);
    let mut candidates: Vec<_> = catalogue
        .iter()
        .filter(|candidate| {
            candidate.kind.is_none_or(|allowed| allowed == kind)
                && matches(candidate.selector, name, &extension)
        })
        .collect();
    let rendering = candidates.iter().any(|candidate| matches!(candidate.identity, PanelIdentity::Builtin(panel) if panel != BuiltinPanel::Source));
    if rendering {
        candidates
            .retain(|candidate| candidate.identity != PanelIdentity::Builtin(BuiltinPanel::Source));
    }
    let exact_name = candidates.iter().any(|candidate| matches!(candidate.selector, FileSelector::Names { filenames, .. } if filenames.contains(&name)));
    let selector = SelectorKey(if exact_name {
        KeyValue::Filename(name.to_owned())
    } else {
        KeyValue::Extension(extension)
    });
    let remembered = associations.get(&selector);
    let associated = candidates
        .iter()
        .copied()
        .find(|candidate| Some(&candidate.identity) == remembered);
    let selected = candidates
        .iter()
        .copied()
        .find(|candidate| Some(&candidate.identity) == choice)
        .or(associated)
        .or_else(|| candidates.first().copied());
    Ok(PanelResolution {
        selected,
        candidates,
        selector,
        associated,
    })
}

#[cfg(test)]
mod tests;
