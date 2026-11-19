//! Paseo's built-in file panels in the Files tab (F-F38, F-F40;
//! docs/plans/wp-file-viewers.md): which panel a file opens in
//! (`view::workspace::viewers`), the Preview|Source side of a Markdown
//! file, the image panel's bytes and zoom, and the images a Markdown
//! preview shows, read through the node.
//!
//! [Decision Log]
//! - Purpose: the page's viewers on the app. Markdown gets a Preview
//!   beside its Source, an image is drawn and zoomable, and both follow
//!   the file on disk.
//! - Constraint: answers arrive without a window. The kit's Markdown
//!   resolves an image through a `Send + Sync` resolver that cannot reach
//!   the shell. HTML has no web engine here (ruling 2).
//! - Alternatives: reading Markdown images in the resolver (it has no
//!   client); scanning the Markdown for images apart from the renderer
//!   (two parsers that could disagree).
//! - Choice: the renderer's resolver names each image it meets in a shared
//!   list and returns what is held for it. After the frame, the shell
//!   reads the named ones with `file.read_bytes`, as it reads the image
//!   panel's file, and hands the decoded images to the same shared map.
//! - Cost: an image shows one frame after its bytes arrive. A workspace
//!   image that fails to read stays empty (its words are not drawn).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use gpui::{Image, ImageFormat, ImageSource, SharedUri};
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::workspace::{Ask, FileAsk, ImageBytes, ReadImage};
use vessel_app_view::composer::viewer::Viewport;
use vessel_app_view::workspace::file_panels::DownloadFailure;
use vessel_app_view::workspace::markdown_paths::{self, Target};
use vessel_app_view::workspace::viewers::{FilePanel, ImageKind, Side, image_kind, panel_of};
use vessel_proto::workspace::{FileVersion, ReadFile, RelPath};

use crate::shell::Shell;

/// How the file shown is drawn.
pub(super) struct Look {
    /// Paseo's panel for it.
    pub(super) panel: FilePanel,
    /// The side shown, for a file with a Preview.
    pub(super) side: Side,
    /// The image panel, on a machine that reads files' bytes.
    pub(super) image: Option<ImagePanel>,
    /// The images a Markdown preview names.
    pub(super) pictures: Pictures,
}

impl Look {
    /// The look of `path`, opened at a line or not; `bytes`: the machine
    /// has `file.read_bytes`.
    pub(super) fn of(path: &RelPath, at_line: bool, bytes: bool) -> Self {
        let panel = panel_of(path.as_str());
        Self {
            panel,
            side: Side::opening(at_line),
            image: (bytes && matches!(panel, FilePanel::Image(_))).then(ImagePanel::default),
            pictures: Pictures::default(),
        }
    }

    /// Whether the Preview is shown.
    pub(super) fn previewing(&self) -> bool {
        self.panel.has_preview() && self.side == Side::Preview
    }
}

/// The image panel's state.
#[derive(Default)]
pub(super) struct ImagePanel {
    /// The bytes as drawn.
    pub(super) state: ImageState,
    /// The version whose bytes were asked, so a re-read of the same
    /// version asks nothing.
    asked: Option<FileVersion>,
    /// The zoom, fitted when the image and the panel's size are known.
    pub(super) viewport: Option<Viewport>,
    /// The pointer is over the image (Paseo's toolbar shows then).
    pub(super) hovered: bool,
    /// The last point of a drag.
    pub(super) dragged: Option<vessel_app_view::composer::viewer::Position>,
    /// The panel's bounds as last laid out (the zoom's canvas).
    pub(super) canvas: std::rc::Rc<
        std::cell::Cell<Option<vessel_app_kit::ui::Bounds<vessel_app_kit::ui::Pixels>>>,
    >,
}

/// An image's bytes as drawn.
#[derive(Default)]
pub(super) enum ImageState {
    /// Not read yet.
    #[default]
    Loading,
    /// Why it was not read.
    Failed(String),
    /// Decoded by GPUI as it is drawn.
    Ready(Arc<Image>),
}

/// The images a Markdown preview names, shared with the renderer's
/// resolver.
#[derive(Clone, Default)]
pub(super) struct Pictures {
    /// Named by the renderer, not asked yet.
    wanted: Arc<Mutex<Vec<String>>>,
    /// Asked or held, by the name the document gives.
    held: Arc<Mutex<HashMap<String, Option<Arc<Image>>>>>,
    /// The document's names for each file asked (two spellings of one
    /// file are read once).
    names: Arc<Mutex<HashMap<RelPath, Vec<String>>>>,
}

impl Pictures {
    /// The resolver the renderer calls for each image it meets.
    pub(super) fn resolver(&self) -> impl Fn(&SharedUri) -> ImageSource + Send + Sync + 'static {
        let pictures = self.clone();
        move |uri| pictures.source(uri.as_ref())
    }

    fn source(&self, name: &str) -> ImageSource {
        let held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(Some(image)) = held.get(name) {
            return ImageSource::Image(Arc::clone(image));
        }
        if !held.contains_key(name) {
            self.wanted
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(name.to_owned());
        }
        // Nothing to draw yet (GPUI's "still loading"), never a path on
        // this computer.
        ImageSource::Custom(Arc::new(|_, _| None))
    }

    /// The names met since the last call, each once.
    fn drain(&self) -> Vec<String> {
        let wanted =
            std::mem::take(&mut *self.wanted.lock().unwrap_or_else(PoisonError::into_inner));
        let mut held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        wanted
            .into_iter()
            .filter(|name| held.insert(name.clone(), None).is_none())
            .collect()
    }

    /// The document names `name`, which is `path`: whether `path` is new
    /// (to be read).
    fn name(&self, name: String, path: &RelPath) -> bool {
        let mut names = self.names.lock().unwrap_or_else(PoisonError::into_inner);
        let known = names.contains_key(path);
        names.entry(path.clone()).or_default().push(name);
        !known
    }

    /// `path`'s bytes arrived: every name for it shows them.
    fn keep(&self, path: &RelPath, image: &Arc<Image>) {
        let names = self
            .names
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(path)
            .cloned()
            .unwrap_or_default();
        let mut held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        for name in names {
            let _before = held.insert(name, Some(Arc::clone(image)));
        }
    }
}

/// GPUI's decoder for `kind`.
const fn format(kind: ImageKind) -> ImageFormat {
    match kind {
        ImageKind::Png => ImageFormat::Png,
        ImageKind::Jpeg => ImageFormat::Jpeg,
        ImageKind::Gif => ImageFormat::Gif,
        ImageKind::Webp => ImageFormat::Webp,
        ImageKind::Svg => ImageFormat::Svg,
    }
}

impl Shell {
    /// A read of `ask` answered at `version`: the image panel reads the
    /// bytes of a version it has not asked for (opened, or changed on
    /// disk).
    pub(in crate::areas::workspace) fn code_image_seen(
        &mut self,
        ask: &ReadFile,
        version: Option<FileVersion>,
    ) {
        let panel = &mut self.areas.workspace.code;
        let Some((machine, root)) = panel
            .root()
            .map(|(machine, root)| (machine.to_owned(), *root))
        else {
            return;
        };
        if root != ask.root {
            return;
        }
        let Some(image) = panel
            .files
            .viewer
            .as_mut()
            .filter(|viewer| viewer.path == ask.path)
            .and_then(|viewer| viewer.look.image.as_mut())
        else {
            return;
        };
        if version.is_some() && image.asked == version {
            return;
        }
        image.asked = version;
        let read = ReadImage {
            root,
            path: ask.path.clone(),
        };
        self.ask(&machine, Ask::File(FileAsk::Image(read)));
    }

    /// An image's bytes arrived: the panel's file, or one its Markdown
    /// preview shows.
    pub(in crate::areas::workspace) fn code_image_read(
        &mut self,
        ask: &ReadImage,
        result: Result<ImageBytes, DownloadFailure>,
        cx: &mut Context<'_, Self>,
    ) {
        let words = self.words();
        let panel = &mut self.areas.workspace.code;
        if panel.root().map(|(_, root)| root) != Some(&ask.root) {
            return;
        }
        let Some(viewer) = panel.files.viewer.as_mut() else {
            return;
        };
        let Some(kind) = image_kind(ask.path.as_str()) else {
            return;
        };
        let decoded =
            result.map(|read| Arc::new(Image::from_bytes(format(kind), read.bytes.to_vec())));
        if viewer.path == ask.path {
            if let Some(image) = viewer.look.image.as_mut() {
                image.state = match decoded {
                    Ok(image) => ImageState::Ready(image),
                    Err(failure) => ImageState::Failed(failure_words(words, failure)),
                };
                // A new image is measured and fitted again, as Paseo's is.
                image.viewport = None;
            }
        } else if let Ok(image) = decoded {
            viewer.look.pictures.keep(&ask.path, &image);
        }
        cx.notify();
    }

    /// After a frame of the Markdown preview: read the images it named
    /// that are files of the root.
    pub(super) fn preview_pictures(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) {
        let panel = &self.areas.workspace.code;
        let Some((machine, root)) = panel
            .root()
            .map(|(machine, root)| (machine.to_owned(), *root))
        else {
            return;
        };
        let folder = panel.cwd().map(|cwd| cwd.to_string_lossy().into_owned());
        let Some(viewer) = panel.files.viewer.as_ref() else {
            return;
        };
        let from = viewer.path.clone();
        let pictures = viewer.look.pictures.clone();
        let mut asked = false;
        for name in pictures.drain() {
            let Target::File(path) = markdown_paths::target(&name, &from, folder.as_deref()) else {
                continue;
            };
            if image_kind(path.as_str()).is_none() || !pictures.name(name, &path) {
                continue;
            }
            self.ask(
                &machine,
                Ask::File(FileAsk::Image(ReadImage { root, path })),
            );
            asked = true;
        }
        if asked {
            cx.notify();
        }
    }

    /// Preview or Source, from the head's control.
    pub(super) fn code_side(&mut self, side: Side, cx: &mut Context<'_, Self>) {
        if let Some(viewer) = self.areas.workspace.code.files.viewer.as_mut()
            && viewer.look.side != side
        {
            viewer.look.side = side;
            cx.notify();
        }
    }
}

/// What the image panel says when the bytes were not read.
fn failure_words(
    words: &'static dyn vessel_app_view::text::workspace::WorkspaceWords,
    failure: DownloadFailure,
) -> String {
    let labels = words.labels();
    match failure {
        DownloadFailure::Call(problem) => words.problem(&problem),
        DownloadFailure::Inconsistent => labels.download_changed.to_owned(),
        DownloadFailure::Local(why) => why,
    }
}
