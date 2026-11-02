//! Shared fixtures for the `Workspace` tests.

// reason: each test binary compiles this module and uses a different subset of it.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use kul_wasm::Workspace;

pub fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .join("examples")
}

/// `(name, text)` of every `.kul` file of one example project, by name.
pub fn example_files(project: &str) -> Vec<(String, String)> {
    let dir = examples_dir().join(project);
    let mut files: Vec<(String, String)> = std::fs::read_dir(&dir)
        .unwrap_or_else(|err| panic!("read {}: {err}", dir.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("kul"))
        .map(|p| {
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            let text = std::fs::read_to_string(&p).expect("read .kul file");
            (name, text)
        })
        .collect();
    files.sort();
    files
}

/// Every example project's directory name.
pub fn example_projects() -> Vec<String> {
    let mut projects: Vec<String> = std::fs::read_dir(examples_dir())
        .expect("read examples")
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|p| !example_files(p).is_empty())
        .collect();
    projects.sort();
    projects
}

/// A workspace holding `files`, each at version 1.
pub fn workspace(files: &[(&str, &str)]) -> Workspace {
    let mut ws = Workspace::new();
    for (name, text) in files {
        ws.set_file((*name).to_owned(), (*text).to_owned(), 1);
    }
    ws
}

/// A workspace holding one example project.
pub fn example_workspace(project: &str) -> Workspace {
    let mut ws = Workspace::new();
    for (name, text) in example_files(project) {
        ws.set_file(name, text, 1);
    }
    ws
}
