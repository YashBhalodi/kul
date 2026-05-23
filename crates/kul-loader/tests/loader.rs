//! Tests for the project loader.
//!
//! Fixtures live in `tests/fixtures/<scenario>/` and exercise the
//! loader's filesystem-level behaviour: happy path, missing manifest,
//! empty project (no `.kul` files), subdirectories ignored,
//! non-`.kul` files ignored, and an unreadable `.kul` file. Error
//! variants are snapshot-tested through their `Display` impl so the
//! human-facing rendering stays stable.
//!
//! Both entry points are exercised here: [`load`] (strict, CLI-shaped)
//! and [`discover`] (lenient, LSP-shaped). The shared discovery rule
//! lives once in the loader; the tests cover that both projections of
//! it agree on what counts as a `.kul` file and what the iteration
//! order is.

use std::path::PathBuf;

use kul_loader::{DiscoveredProject, LoadedProject, ProjectLoadError, discover, load};

fn fixture(scenario: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(scenario)
}

fn input_names(project: &LoadedProject) -> Vec<&str> {
    project.inputs.iter().map(|i| i.name.as_str()).collect()
}

#[test]
fn happy_path_loads_manifest_and_all_kul_files_in_order() {
    let project = load(&fixture("happy-path")).expect("load");
    assert!(project.manifest_yaml.contains("kul: \"0.1\""));
    assert_eq!(
        input_names(&project),
        ["01-founders.kul", "02-children.kul"]
    );
    // The first file's bytes match the fixture; sanity-checks the
    // loader actually read the source rather than handing back empty
    // strings.
    assert!(project.inputs[0].source.contains("person alice"));
    assert!(project.inputs[1].source.contains("person carol"));
}

#[test]
fn manifest_path_label_uses_kul_yml_under_root() {
    let project = load(&fixture("happy-path")).expect("load");
    assert!(
        project.manifest_name.ends_with("kul.yml"),
        "manifest_name should reference kul.yml; got {}",
        project.manifest_name,
    );
}

#[test]
fn missing_manifest_returns_typed_error() {
    let err = load(&fixture("missing-manifest")).expect_err("expected error");
    assert!(
        matches!(err, ProjectLoadError::ManifestNotFound { .. }),
        "expected ManifestNotFound, got {err:?}",
    );
}

#[test]
fn missing_manifest_rendering_is_stable() {
    let err = load(&fixture("missing-manifest")).expect_err("expected error");
    // Strip the absolute path prefix so the snapshot stays portable
    // across checkout locations, then normalize separators so the
    // snapshot matches on Windows too (`PathBuf::Display` renders
    // `\` on Windows, `/` on Unix).
    let rendered = err
        .to_string()
        .replace(env!("CARGO_MANIFEST_DIR"), "<crate-root>")
        .replace('\\', "/");
    insta::assert_snapshot!(rendered);
}

#[test]
fn empty_project_loads_with_no_inputs() {
    let project = load(&fixture("empty-project")).expect("load");
    assert!(project.inputs.is_empty());
    // KUL-M06 is `kul_core::check`'s job — the loader stays silent
    // on the empty case and the diagnostic flows through the normal
    // check pipeline.
    assert!(project.manifest_yaml.contains("kul: \"0.1\""));
}

#[test]
fn subdirectories_are_ignored() {
    let project = load(&fixture("with-subdirs")).expect("load");
    // The fixture has a `notes/scratch.kul` that must not appear.
    assert_eq!(input_names(&project), ["main.kul"]);
}

#[test]
fn non_kul_files_are_ignored() {
    let project = load(&fixture("with-non-kul-files")).expect("load");
    assert_eq!(input_names(&project), ["main.kul"]);
}

#[test]
fn directory_named_with_kul_extension_is_skipped() {
    // A directory like `notes.kul/` should be ignored (subdirectories
    // rule) rather than tripping a read error. We build this at
    // test-time to keep the on-disk fixture set simple.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("target")
        .join("kul-loader-tests")
        .join("dir-with-kul-extension");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("kul.yml"), "kul: \"0.1\"\n").unwrap();
    std::fs::write(
        root.join("main.kul"),
        "person alice name:\"Alice\" gender:female born:1950\n",
    )
    .unwrap();
    std::fs::create_dir(root.join("backups.kul")).unwrap();
    let project = load(&root).expect("load");
    assert_eq!(input_names(&project), ["main.kul"]);
}

#[cfg(unix)]
#[test]
fn unreadable_kul_file_surfaces_input_read_error() {
    use std::os::unix::fs::PermissionsExt;

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("target")
        .join("kul-loader-tests")
        .join("unreadable-input");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("kul.yml"), "kul: \"0.1\"\n").unwrap();
    let bad = root.join("bad.kul");
    std::fs::write(
        &bad,
        "person alice name:\"Alice\" gender:female born:1950\n",
    )
    .unwrap();
    // 0o000 — the test must run as a non-root user to be effective;
    // root would bypass the permission check entirely.
    std::fs::set_permissions(&bad, std::fs::Permissions::from_mode(0o000)).unwrap();

    let result = load(&root);

    // Restore readability so cargo can clean up the file even if the
    // assertion below fails.
    let _ = std::fs::set_permissions(&bad, std::fs::Permissions::from_mode(0o644));

    let err = result.expect_err("expected error");
    assert!(
        matches!(err, ProjectLoadError::InputReadFailed { .. }),
        "expected InputReadFailed, got {err:?}",
    );
}

// --- discover() — the lenient, LSP-shaped projection ----------------

fn file_names(project: &DiscoveredProject) -> Vec<&str> {
    project.files.iter().map(|f| f.name.as_str()).collect()
}

#[test]
fn discover_happy_path_matches_load() {
    // Both projections agree on which files belong to the project and
    // in what order — the shared enumerator is exercised through both
    // entry points.
    let strict = load(&fixture("happy-path")).expect("load");
    let lenient = discover(&fixture("happy-path"));

    assert_eq!(file_names(&lenient), ["01-founders.kul", "02-children.kul"]);
    let strict_names: Vec<&str> = strict.inputs.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(file_names(&lenient), strict_names);
    assert_eq!(lenient.manifest_yaml, strict.manifest_yaml);
}

#[test]
fn discover_tolerates_missing_manifest() {
    // The LSP keeps an editor session usable even when the manifest is
    // absent. The lenient projection returns empty YAML but still
    // populates `manifest_name` with the expected path label.
    let project = discover(&fixture("missing-manifest"));
    assert_eq!(project.manifest_yaml, "");
    assert!(
        project.manifest_name.ends_with("kul.yml"),
        "manifest_name should be the expected sibling kul.yml path; got {}",
        project.manifest_name,
    );
}

#[test]
fn discover_path_field_is_absolute_and_readable() {
    // The LSP turns `path` into a `file://` URL — verify the loader
    // hands back a real path that round-trips, not a relative or
    // mangled one.
    let project = discover(&fixture("happy-path"));
    for f in &project.files {
        assert!(
            f.path.is_absolute(),
            "path should be absolute: {:?}",
            f.path
        );
        assert!(f.path.exists(), "path should exist on disk: {:?}", f.path);
        assert!(
            f.path.ends_with(&f.name),
            "path should end with bare name; path={:?} name={}",
            f.path,
            f.name,
        );
    }
}

#[test]
fn discover_skips_subdirs_like_load() {
    // The flat-directory rule (ADR-0015) must hold for the lenient path
    // too — a nested `.kul` under a subdirectory is invisible to both.
    let project = discover(&fixture("with-subdirs"));
    assert_eq!(file_names(&project), ["main.kul"]);
}

#[test]
fn discover_skips_non_kul_like_load() {
    let project = discover(&fixture("with-non-kul-files"));
    assert_eq!(file_names(&project), ["main.kul"]);
}

#[test]
fn discover_unreadable_directory_yields_empty_files() {
    // Pointing discover at a path that isn't a directory at all is the
    // "unreadable directory" path: read_dir fails, the lenient
    // projection swallows the error and returns an empty file list.
    // The manifest_yaml stays empty too.
    let not_a_directory = fixture("happy-path").join("01-founders.kul");
    assert!(not_a_directory.exists(), "fixture sanity check");
    let project = discover(&not_a_directory);
    assert!(project.files.is_empty());
    assert_eq!(project.manifest_yaml, "");
}

#[cfg(unix)]
#[test]
fn discover_skips_unreadable_kul_file() {
    // The strict loader errors on this case; the lenient projection
    // silently drops the file. Mirror the layout of
    // `unreadable_kul_file_surfaces_input_read_error` so the contrast
    // between the two postures is obvious.
    use std::os::unix::fs::PermissionsExt;

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("target")
        .join("kul-loader-tests")
        .join("discover-unreadable-input");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("kul.yml"), "kul: \"0.1\"\n").unwrap();
    std::fs::write(
        root.join("good.kul"),
        "person alice name:\"Alice\" gender:female born:1950\n",
    )
    .unwrap();
    let bad = root.join("bad.kul");
    std::fs::write(&bad, "person bob name:\"Bob\" gender:male born:1950\n").unwrap();
    std::fs::set_permissions(&bad, std::fs::Permissions::from_mode(0o000)).unwrap();

    let project = discover(&root);

    // Restore readability so cargo can clean up the file even if the
    // assertion below fails.
    let _ = std::fs::set_permissions(&bad, std::fs::Permissions::from_mode(0o644));

    assert_eq!(file_names(&project), ["good.kul"]);
}
