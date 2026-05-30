//! Tests for the project loader. Fixtures live in
//! `tests/fixtures/<scenario>/`. Error rendering is snapshot-tested
//! through `Display`. Both [`load`] (strict) and [`discover`]
//! (lenient) are exercised against the shared discovery rule.

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
    // Strip absolute path prefix and normalize separators so the
    // snapshot stays portable across checkouts and across Windows/Unix.
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
    assert!(project.manifest_yaml.contains("kul: \"0.1\""));
}

#[test]
fn subdirectories_are_ignored() {
    // Fixture has a `notes/scratch.kul` that must not appear.
    let project = load(&fixture("with-subdirs")).expect("load");
    assert_eq!(input_names(&project), ["main.kul"]);
}

#[test]
fn non_kul_files_are_ignored() {
    let project = load(&fixture("with-non-kul-files")).expect("load");
    assert_eq!(input_names(&project), ["main.kul"]);
}

#[test]
fn directory_named_with_kul_extension_is_skipped() {
    // `notes.kul/` must be ignored (subdirectory rule) rather than
    // tripping a read error. Built at test-time to avoid checking a
    // directory into the fixture set.
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
    // Must run as non-root; root would bypass the permission check.
    std::fs::set_permissions(&bad, std::fs::Permissions::from_mode(0o000)).unwrap();

    let result = load(&root);

    // Restore readability so cargo can clean up even if the assertion fails.
    let _ = std::fs::set_permissions(&bad, std::fs::Permissions::from_mode(0o644));

    let err = result.expect_err("expected error");
    assert!(
        matches!(err, ProjectLoadError::InputReadFailed { .. }),
        "expected InputReadFailed, got {err:?}",
    );
}

fn file_names(project: &DiscoveredProject) -> Vec<&str> {
    project.files.iter().map(|f| f.name.as_str()).collect()
}

#[test]
fn discover_happy_path_matches_load() {
    let strict = load(&fixture("happy-path")).expect("load");
    let lenient = discover(&fixture("happy-path"));

    assert_eq!(file_names(&lenient), ["01-founders.kul", "02-children.kul"]);
    let strict_names: Vec<&str> = strict.inputs.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(file_names(&lenient), strict_names);
    assert_eq!(lenient.manifest_yaml, strict.manifest_yaml);
}

#[test]
fn discover_tolerates_missing_manifest() {
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
    // The LSP turns `path` into a `file://` URL — the loader must
    // hand back a real, absolute path that round-trips.
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
    // Pointing discover at a non-directory exercises the `read_dir`
    // failure path: lenient projection swallows it and returns empty.
    let not_a_directory = fixture("happy-path").join("01-founders.kul");
    assert!(not_a_directory.exists(), "fixture sanity check");
    let project = discover(&not_a_directory);
    assert!(project.files.is_empty());
    assert_eq!(project.manifest_yaml, "");
}

#[cfg(unix)]
#[test]
fn discover_skips_unreadable_kul_file() {
    // Counterpart to `unreadable_kul_file_surfaces_input_read_error`:
    // strict errors, lenient silently drops the file.
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

    // Restore readability so cargo can clean up even if the assertion fails.
    let _ = std::fs::set_permissions(&bad, std::fs::Permissions::from_mode(0o644));

    assert_eq!(file_names(&project), ["good.kul"]);
}
