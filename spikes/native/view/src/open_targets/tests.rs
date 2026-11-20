//! The targets' rules against Paseo's, with a fake computer: nothing is
//! read from this one and nothing is started.

use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

use super::guard::{Refused, plain_for_tests};
use super::launch::Unavailable;
use super::*;

/// A computer made of a few paths.
#[derive(Default)]
struct Fake {
    platform: Option<Platform>,
    env: Env,
    exists: HashSet<PathBuf>,
    entries: HashMap<PathBuf, Vec<Entry>>,
    /// Links: a path and where it really is.
    links: HashMap<PathBuf, PathBuf>,
}

impl Fake {
    fn on(platform: Platform, path: &str) -> Self {
        Self {
            platform: Some(platform),
            env: Env {
                path: Some(path.to_owned()),
                home: Some("/home/me".to_owned()),
                local_app_data: None,
                program_files: None,
            },
            ..Self::default()
        }
    }

    fn with(mut self, paths: &[&str]) -> Self {
        self.exists.extend(paths.iter().map(PathBuf::from));
        self
    }
}

impl Probe for Fake {
    fn platform(&self) -> Platform {
        self.platform.unwrap_or(Platform::Other)
    }

    fn env(&self) -> &Env {
        &self.env
    }

    fn exists(&self, path: &Path) -> bool {
        self.exists.contains(path)
    }

    fn is_file(&self, path: &Path) -> bool {
        self.exists.contains(path) && !self.entries.contains_key(path)
    }

    fn entries(&self, dir: &Path) -> io::Result<Vec<Entry>> {
        self.entries
            .get(dir)
            .cloned()
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
    }

    fn canonical(&self, path: &Path) -> io::Result<PathBuf> {
        if let Some(real) = self.links.get(path) {
            return Ok(real.clone());
        }
        if self.exists.contains(path) {
            return Ok(path.to_path_buf());
        }
        Err(io::Error::from(io::ErrorKind::NotFound))
    }
}

fn dir(name: &str) -> Entry {
    Entry {
        name: name.to_owned(),
        kind: EntryKind::Directory,
    }
}

fn file(name: &str) -> Entry {
    Entry {
        name: name.to_owned(),
        kind: EntryKind::File,
    }
}

fn os(args: &[&str]) -> Vec<OsString> {
    args.iter().map(OsString::from).collect()
}

fn checked_at(file: Option<&str>, line: Option<u32>, column: Option<u32>) -> Checked {
    Checked {
        folder: PathBuf::from("/w"),
        file: file.map(PathBuf::from),
        line,
        column,
    }
}

#[test]
fn the_targets_are_paseo_s_twenty_five_in_its_order_with_its_ids() {
    let ids: Vec<&str> = TargetId::ALL.iter().map(|target| target.id()).collect();
    assert_eq!(
        ids,
        [
            "cursor",
            "trae",
            "kiro",
            "vscode",
            "vscode-insiders",
            "vscodium",
            "zed",
            "antigravity",
            "android-studio",
            "xcode",
            "intellij-idea",
            "aqua",
            "clion",
            "datagrip",
            "dataspell",
            "goland",
            "phpstorm",
            "pycharm",
            "rider",
            "rubymine",
            "rustrover",
            "webstorm",
            "finder",
            "explorer",
            "file-manager",
        ]
    );
    for target in TargetId::ALL {
        assert_eq!(TargetId::from_id(target.id()), Some(target));
    }
    assert_eq!(TargetId::from_id("notepad"), None);
    assert_eq!(TargetId::FileManager.label(), "Files");
    assert_eq!(TargetId::Explorer.kind(), Kind::FileManager);
    assert_eq!(TargetId::Antigravity.icon(), Icon::Terminal);
    assert_eq!(TargetId::Finder.icon(), Icon::Folder);
    assert_eq!(TargetId::VsCode.icon(), Icon::Mark(EditorMark::VsCode));
}

#[test]
fn only_what_is_on_path_or_installed_is_listed_with_the_platform_s_file_manager() {
    let linux = Fake::on(Platform::Other, "/opt/bin::/usr/bin")
        .with(&["/usr/bin/code", "/opt/bin/zeditor"]);
    assert_eq!(
        list(&linux, Some(Path::new("/w"))),
        [TargetId::VsCode, TargetId::Zed, TargetId::FileManager]
    );
    // Windows: `.cmd` forms on PATH, and the install paths.
    let mut windows =
        Fake::on(Platform::Windows, r"C:\bin;C:\tools").with(&[r"C:\tools/codium.cmd"]);
    windows.env.local_app_data = Some(r"C:\Local".to_owned());
    let _new = windows.exists.insert(
        ["C:\\Local", "Programs", "cursor", "Cursor.exe"]
            .iter()
            .collect(),
    );
    assert_eq!(
        list(&windows, None),
        [TargetId::Cursor, TargetId::VsCodium, TargetId::Explorer]
    );
    // macOS: an app bundle without a command, and Finder.
    let mac = Fake::on(Platform::MacOs, "/usr/bin")
        .with(&["/home/me/Applications/Visual Studio Code.app"]);
    assert_eq!(list(&mac, None), [TargetId::VsCode, TargetId::Finder]);
}

#[test]
fn xcode_is_listed_only_for_a_workspace_it_opens_and_takes_the_folder_s_namesake() {
    let mut mac = Fake::on(Platform::MacOs, "").with(&["/Applications/Xcode.app"]);
    assert_eq!(list(&mac, Some(Path::new("/w"))), [TargetId::Finder]);
    let _old = mac.entries.insert(
        PathBuf::from("/Users/me/App"),
        vec![
            dir("Other.xcodeproj"),
            dir("App.xcodeproj"),
            file("Package.swift"),
        ],
    );
    let app = Path::new("/Users/me/App");
    assert_eq!(list(&mac, Some(app)), [TargetId::Xcode, TargetId::Finder]);
    assert_eq!(list(&mac, None), [TargetId::Finder]);
    assert_eq!(
        listing::xcode_document(app, &mac).ok().flatten(),
        Some(app.join("App.xcodeproj"))
    );
    let _old = mac.entries.insert(
        app.to_path_buf(),
        vec![
            dir("B.xcodeproj"),
            dir("Z.xcworkspace"),
            dir("A.xcworkspace"),
        ],
    );
    assert_eq!(
        listing::xcode_document(app, &mac).ok().flatten(),
        Some(app.join("A.xcworkspace"))
    );
    let _old = mac
        .entries
        .insert(app.to_path_buf(), vec![file("Package.swift")]);
    let opened = plan(
        TargetId::Xcode,
        &checked_at(Some("/w/a.swift"), Some(3), None),
        &{
            let mut at = Fake::on(Platform::MacOs, "").with(&["/Applications/Xcode.app"]);
            let _old = at
                .entries
                .insert(PathBuf::from("/w"), vec![file("Package.swift")]);
            at
        },
    );
    assert_eq!(
        opened,
        Ok(Plan::Start {
            program: PathBuf::from("/usr/bin/open"),
            args: os(&[
                "-a",
                "/Applications/Xcode.app",
                "/w/Package.swift",
                "/w/a.swift"
            ]),
        })
    );
    assert_eq!(
        plan(TargetId::Xcode, &checked_at(None, None, None), &mac),
        Err(Unavailable::NoXcodeDocument(PathBuf::from("/w")))
    );
}

#[test]
fn each_family_gets_paseo_s_arguments() {
    let fake = Fake::on(Platform::Other, "/bin").with(&[
        "/bin/code",
        "/bin/kiro",
        "/bin/zed",
        "/bin/studio",
        "/bin/idea",
        "/bin/webstorm64",
    ]);
    let started = |target: TargetId, request: &Checked| match plan(target, request, &fake) {
        Ok(Plan::Start { args, .. }) => args,
        other => panic!("{other:?}"),
    };
    let folder = checked_at(None, None, None);
    let at_file = checked_at(Some("/w/src/a.rs"), None, None);
    let at_line = checked_at(Some("/w/src/a.rs"), Some(12), Some(4));
    let line_only = checked_at(Some("/w/src/a.rs"), Some(12), None);
    assert_eq!(started(TargetId::VsCode, &folder), os(&["/w"]));
    assert_eq!(
        started(TargetId::VsCode, &at_file),
        os(&["/w", "/w/src/a.rs"])
    );
    assert_eq!(
        started(TargetId::VsCode, &at_line),
        os(&["/w", "--goto", "/w/src/a.rs:12:4"])
    );
    assert_eq!(
        started(TargetId::VsCode, &line_only),
        os(&["/w", "--goto", "/w/src/a.rs:12"])
    );
    assert_eq!(started(TargetId::Kiro, &folder), os(&["ide", "/w"]));
    assert_eq!(
        started(TargetId::Kiro, &at_line),
        os(&["ide", "/w", "--goto", "/w/src/a.rs:12:4"])
    );
    assert_eq!(started(TargetId::Zed, &at_file), os(&["/w", "/w/src/a.rs"]));
    assert_eq!(
        started(TargetId::Zed, &at_line),
        os(&["/w", "/w/src/a.rs:12:4"])
    );
    assert_eq!(
        started(TargetId::AndroidStudio, &at_line),
        os(&["/w", "--line", "12", "--column", "4", "/w/src/a.rs"])
    );
    assert_eq!(
        started(TargetId::IntellijIdea, &at_line),
        os(&["--line", "12", "--column", "4", "/w", "/w/src/a.rs"])
    );
    assert_eq!(started(TargetId::WebStorm, &folder), os(&["/w"]));
    assert_eq!(
        started(TargetId::WebStorm, &at_file),
        os(&["/w", "/w/src/a.rs"])
    );
    assert_eq!(
        plan(TargetId::Cursor, &folder, &fake),
        Err(Unavailable::NotInstalled(TargetId::Cursor))
    );
}

#[test]
fn a_mac_app_without_a_command_opens_through_open_and_the_file_manager_reveals_or_opens() {
    let mac = Fake::on(Platform::MacOs, "/usr/bin").with(&["/Applications/Zed.app"]);
    assert_eq!(
        plan(
            TargetId::Zed,
            &checked_at(Some("/w/a.rs"), Some(2), None),
            &mac
        ),
        Ok(Plan::Start {
            program: PathBuf::from("/usr/bin/open"),
            args: os(&["-a", "/Applications/Zed.app", "/w", "/w/a.rs"]),
        })
    );
    assert_eq!(
        plan(
            TargetId::Finder,
            &checked_at(Some("/w/a.rs"), None, None),
            &mac
        ),
        Ok(Plan::Reveal(PathBuf::from("/w/a.rs")))
    );
    assert_eq!(
        plan(TargetId::FileManager, &checked_at(None, None, None), &mac),
        Ok(Plan::OpenFolder(PathBuf::from("/w")))
    );
}

#[test]
fn only_existing_absolute_paths_under_the_root_pass_and_links_out_of_it_are_refused() {
    let mut fake =
        Fake::on(Platform::Other, "").with(&["/w", "/w/src", "/w/src/a.rs", "/etc/passwd"]);
    let _old = fake
        .links
        .insert(PathBuf::from("/w/out"), PathBuf::from("/etc/passwd"));
    let request = |folder: &str, file: Option<&str>| Request {
        root: PathBuf::from("/w"),
        folder: PathBuf::from(folder),
        file: file.map(PathBuf::from),
        line: Some(0),
        column: None,
    };
    assert_eq!(
        checked(&request("/w/src", Some("/w/src/a.rs")), &fake),
        Ok(Checked {
            folder: PathBuf::from("/w/src"),
            file: Some(PathBuf::from("/w/src/a.rs")),
            line: None,
            column: None,
        })
    );
    assert_eq!(
        checked(&request("w/src", None), &fake),
        Err(Refused::NotAbsolute(PathBuf::from("w/src")))
    );
    assert_eq!(
        checked(&request("/w", Some("/w/gone.rs")), &fake),
        Err(Refused::Missing(PathBuf::from("/w/gone.rs")))
    );
    assert_eq!(
        checked(&request("/w", Some("/w/out")), &fake),
        Err(Refused::Outside(PathBuf::from("/w/out")))
    );
    assert_eq!(
        checked(&request("/etc", None), &fake),
        Err(Refused::Missing(PathBuf::from("/etc")))
    );
}

#[test]
fn a_windows_drive_path_loses_its_verbatim_prefix_and_nothing_else_does() {
    assert_eq!(
        plain_for_tests(r"\\?\C:\w\a.rs"),
        PathBuf::from(r"C:\w\a.rs")
    );
    assert_eq!(
        plain_for_tests(r"\\?\UNC\server\share"),
        PathBuf::from(r"\\?\UNC\server\share")
    );
    assert_eq!(plain_for_tests("/w/a.rs"), PathBuf::from("/w/a.rs"));
}

#[test]
fn the_preferred_target_is_the_stored_one_while_listed_else_the_first() {
    let listed = [TargetId::Zed, TargetId::FileManager];
    assert_eq!(
        preferred(&listed, Some(TargetId::FileManager)),
        Some(TargetId::FileManager)
    );
    assert_eq!(
        preferred(&listed, Some(TargetId::Cursor)),
        Some(TargetId::Zed)
    );
    assert_eq!(preferred(&listed, None), Some(TargetId::Zed));
    assert_eq!(preferred(&[], Some(TargetId::Zed)), None);
}

#[test]
fn relative_path_folders_and_folders_named_like_a_command_are_not_commands() {
    let mut fake = Fake::on(Platform::Other, "bin:/usr/bin").with(&["bin/zed", "/usr/bin/code"]);
    let _old = fake
        .entries
        .insert(PathBuf::from("/usr/bin/code"), Vec::new());
    assert_eq!(list(&fake, None), [TargetId::FileManager]);
    let windows =
        Fake::on(Platform::Windows, r"tools;C:\bin").with(&[r"tools/zed.exe", r"C:\bin/code.cmd"]);
    assert_eq!(list(&windows, None), [TargetId::VsCode, TargetId::Explorer]);
}
