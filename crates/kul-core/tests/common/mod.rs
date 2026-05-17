//! Shared fixture helpers for the `kul-core` integration tests.
//!
//! Each `tests/*.rs` file is compiled as its own test binary. The
//! conventional Rust shape for code shared across them is a sibling
//! module under `tests/common/`; integration tests pull it in with
//! `mod common;`. The helpers here cover the single-in-memory-source
//! pattern that `validator.rs`, `export.rs`, and `format.rs` all want;
//! the multi-file directory-loading fixture in `multi_file.rs` has only
//! the one consumer and stays local to that file.

use kul_core::CheckResult;
use kul_core::ast::InputFile;
use kul_core::manifest::Manifest;

/// Run `kul_core::check_with_manifest` on a single in-memory source
/// named `test.kul` against the default manifest. The shape every
/// per-rule and per-feature integration test wants when it doesn't
/// care about the manifest content or the input filename. The empty
/// `manifest_yaml` argument is correct here — `check_with_manifest`
/// only consumes those bytes when rendering manifest-anchored
/// diagnostics, and an in-memory single-input test never triggers
/// one.
pub fn check_one(source: &str) -> CheckResult {
    let inputs = vec![InputFile::new("test.kul", source)];
    kul_core::check_with_manifest("kul.yml", "", &Manifest::default(), &inputs)
}
