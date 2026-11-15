//! Inventory follows normal declarations on every target. Cargo parses each
//! local workspace without resolving external packages; filtering by the host
//! would silently exempt a Windows-only or browser-only runtime dependency.
//! Development/build declarations never initiate traversal; a name used in
//! both roles remains required. Optional normal declarations are conservative:
//! a shipping feature can enable them without changing this inventory test.
//! Native members may also declare external dependencies without workspace
//! inheritance. Those must be required too; otherwise an inline target-only
//! dependency silently escapes the workspace-manifest order comparison.

use super::{BUNDLED, Role};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(transparent)]
struct PackageId(String);
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Kind {
    Dev,
    Build,
}
#[derive(Debug, Clone, Deserialize)]
struct Dependency {
    name: String,
    kind: Option<Kind>,
    path: Option<PathBuf>,
}
#[derive(Debug, Clone, Deserialize)]
struct Package {
    id: PackageId,
    manifest_path: PathBuf,
    dependencies: Vec<Dependency>,
}
#[derive(Debug, Deserialize)]
struct Metadata {
    workspace_members: Vec<PackageId>,
    packages: Vec<Package>,
}

fn runtime_names(metadata: Metadata, mut load: impl FnMut(&Path) -> Metadata) -> BTreeSet<String> {
    let mut pending: Vec<_> = metadata
        .packages
        .iter()
        .filter(|p| metadata.workspace_members.contains(&p.id))
        .map(|p| p.manifest_path.clone())
        .collect();
    let mut packages: BTreeMap<_, _> = metadata
        .packages
        .into_iter()
        .map(|p| (p.manifest_path.clone(), p))
        .collect();
    let mut visited = BTreeSet::new();
    let mut names = BTreeSet::new();
    while let Some(manifest) = pending.pop() {
        if !visited.insert(manifest.clone()) {
            continue;
        }
        if !packages.contains_key(&manifest) {
            for package in load(&manifest).packages {
                let _previous = packages.insert(package.manifest_path.clone(), package);
            }
        }
        for dependency in &packages[&manifest].dependencies {
            if dependency.kind.is_some() {
                continue;
            }
            let _new = names.insert(dependency.name.clone());
            if let Some(path) = &dependency.path {
                pending.push(path.join("Cargo.toml"));
            }
        }
    }
    names
}

fn query(manifest: &Path) -> Metadata {
    let output = std::process::Command::new(env!("CARGO"))
        .args([
            "metadata",
            "--format-version=1",
            "--no-deps",
            "--offline",
            "--locked",
            "--manifest-path",
        ])
        .arg(manifest)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "Cargo declaration query failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

// Preserve the original registry-order comparison; only runtime membership
// changes. Cargo above owns aliases, target syntax and workspace inheritance.
fn dependencies(manifest: &str, runtime: &BTreeSet<String>) -> Vec<String> {
    let mut inside = false;
    let mut names = Vec::new();
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            inside = line == "[workspace.dependencies]";
            continue;
        }
        if !inside || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let name = value
            .split_once("package = \"")
            .and_then(|(_, rest)| rest.split_once('"'))
            .map_or(key, |(package, _)| package);
        if !name.starts_with("vessel-") && runtime.contains(name) {
            names.push(name.to_owned());
        }
    }
    names
}

fn direct_runtime(metadata: &Metadata) -> BTreeSet<String> {
    metadata
        .packages
        .iter()
        .filter(|package| metadata.workspace_members.contains(&package.id))
        .flat_map(|package| &package.dependencies)
        .filter(|dependency| dependency.kind.is_none() && !dependency.name.starts_with("vessel-"))
        .map(|dependency| dependency.name.clone())
        .collect()
}

fn inventory_names(
    manifest: &str,
    runtime: &BTreeSet<String>,
    direct: &BTreeSet<String>,
) -> Vec<String> {
    let mut names = dependencies(manifest, runtime);
    let shared: BTreeSet<_> = names.iter().cloned().collect();
    // Keep the shared manifest's existing order. Inline member declarations
    // follow alphabetically, so metadata package traversal cannot reorder them.
    names.extend(direct.difference(&shared).cloned());
    names
}

#[test]
fn name_every_dependency_in_the_manifests_order() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../Cargo.toml")
        .canonicalize()
        .unwrap();
    let metadata = query(&manifest);
    let direct = direct_runtime(&metadata);
    let required = runtime_names(metadata, query);
    let listed: Vec<_> = BUNDLED
        .iter()
        .filter(|entry| !entry.face)
        .map(|entry| entry.name)
        .collect();
    assert_eq!(
        listed,
        inventory_names(include_str!("../../../Cargo.toml"), &required, &direct),
        "a runtime dependency is missing from BUNDLED, or out of order"
    );
}

#[test]
fn translation_libraries_and_uuid_describe_their_actual_roles() {
    assert_eq!(
        BUNDLED
            .iter()
            .filter(|entry| entry.role == Role::Translations)
            .map(|entry| entry.name)
            .collect::<Vec<_>>(),
        [
            "fluent-bundle",
            "fluent-syntax",
            "fluent-pseudo",
            "intl-memoizer",
            "unic-langid"
        ]
    );
    assert_eq!(
        BUNDLED
            .iter()
            .find(|entry| entry.name == "uuid")
            .map(|entry| entry.role),
        Some(Role::Identifiers)
    );
}

fn fixture(edges: &[(&str, Option<Kind>)]) -> Metadata {
    Metadata {
        workspace_members: vec![PackageId("app".to_owned())],
        packages: vec![Package {
            id: PackageId("app".to_owned()),
            manifest_path: PathBuf::from("app/Cargo.toml"),
            dependencies: edges
                .iter()
                .map(|(name, kind)| Dependency {
                    name: (*name).to_owned(),
                    kind: *kind,
                    path: None,
                })
                .collect(),
        }],
    }
}

fn required(edges: &[(&str, Option<Kind>)]) -> BTreeSet<String> {
    runtime_names(fixture(edges), |_| panic!("no path declaration in fixture"))
}

#[test]
fn development_and_build_only_dependencies_are_not_required() {
    assert_eq!(
        dependencies(
            "[workspace.dependencies]\nnormal = \"1\"\ndev = \"1\"\nbuild = \"1\"",
            &required(&[
                ("normal", None),
                ("dev", Some(Kind::Dev)),
                ("build", Some(Kind::Build))
            ])
        ),
        ["normal"]
    );
}

#[test]
fn missing_runtime_still_fails_and_both_kinds_are_required() {
    let runtime = required(&[("normal", None), ("both", Some(Kind::Dev)), ("both", None)]);
    let names = dependencies(
        "[workspace.dependencies]\nnormal = \"1\"\nboth = \"1\"",
        &runtime,
    );
    assert_eq!(names, ["normal", "both"]);
    assert_ne!(vec!["normal"], names);
}

#[test]
fn inline_normal_declarations_cannot_escape_and_dev_only_stays_excluded() {
    let metadata = fixture(&[
        ("shared", None),
        ("inline", None),
        ("dev-only", Some(Kind::Dev)),
        ("both", Some(Kind::Dev)),
        ("both", None),
    ]);
    let direct = direct_runtime(&metadata);
    let runtime = runtime_names(metadata, |_| panic!("no path declaration in fixture"));
    let manifest = "[workspace.dependencies]\nshared = \"1\"";
    // The former comparison accepts just shared; the strengthened gate
    // additionally requires each inline normal declaration, including both.
    assert_eq!(dependencies(manifest, &runtime), ["shared"]);
    assert_eq!(
        inventory_names(manifest, &runtime, &direct),
        ["shared", "both", "inline"]
    );
    assert_ne!(vec!["shared"], inventory_names(manifest, &runtime, &direct));
}

#[test]
fn another_targets_inline_runtime_declaration_is_required() {
    let metadata: Metadata = serde_json::from_str(r#"{
        "workspace_members":["app"],"packages":[{"id":"app","manifest_path":"app/Cargo.toml",
        "dependencies":[{"name":"inline-browser","kind":null,"target":"cfg(target_family = \"wasm\")","path":null}]}]}
    "#).unwrap();
    let direct = direct_runtime(&metadata);
    let runtime = runtime_names(metadata, |_| panic!("no path declaration in fixture"));
    assert_eq!(
        inventory_names("[workspace.dependencies]", &runtime, &direct),
        ["inline-browser"]
    );
}

#[test]
fn another_targets_runtime_and_transitive_path_alias_are_required() {
    // --no-deps retains this declaration even when Cargo cannot compile its
    // target here. The platform field intentionally has no filtering role.
    let root: Metadata = serde_json::from_str(
        r#"{
        "workspace_members":["app"],"packages":[{"id":"app","manifest_path":"app/Cargo.toml",
        "dependencies":[{"name":"foreign","kind":null,"target":"cfg(windows)","path":"foreign"},
        {"name":"browser","kind":null,"target":"cfg(target_family = \"wasm\")","path":null}]}]}
    "#,
    )
    .unwrap();
    let runtime = runtime_names(root, |manifest| {
        assert_eq!(manifest, Path::new("foreign/Cargo.toml"));
        let mut data = fixture(&[("published-name", None)]);
        data.packages[0].manifest_path = manifest.to_owned();
        data
    });
    assert_eq!(
        dependencies(
            "[workspace.dependencies]\nbrowser = \"1\"\nalias = { package = \"published-name\", version = \"1\" }",
            &runtime
        ),
        ["browser", "published-name"]
    );
}

#[test]
fn list_every_face_the_kit_bundles() {
    let fonts = include_str!("../../../kit/src/fonts.rs");
    for (face, file) in [
        ("Geist", "Geist-Regular"),
        ("Geist Mono", "GeistMono-Regular"),
        ("Pretendard", "Pretendard-Regular"),
    ] {
        assert!(fonts.contains(file), "{file} is no longer bundled");
        assert!(
            BUNDLED.iter().any(|entry| entry.face && entry.name == face),
            "{face} is bundled and not listed"
        );
    }
}
