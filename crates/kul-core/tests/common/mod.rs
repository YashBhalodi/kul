//! Shared fixture helpers for the `kul-core` integration tests.

use kul_core::CheckResult;
use kul_core::ast::InputFile;
use kul_core::manifest::Manifest;

/// Check a single in-memory source named `test.kul` against the default
/// manifest. The empty `manifest_yaml` is fine — it is only read when
/// rendering manifest-anchored diagnostics.
pub fn check_one(source: &str) -> CheckResult {
    let inputs = vec![InputFile::new("test.kul", source)];
    kul_core::check_with_manifest("kul.yml", "", &Manifest::default(), &inputs)
}
