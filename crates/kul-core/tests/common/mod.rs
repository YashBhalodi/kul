//! Shared fixture helpers for the `kul-core` integration tests.

use kul_core::CheckResult;
use kul_core::ast::InputFile;

/// Check a one-file project: a single in-memory source named `test.kul`.
pub fn check_one(source: &str) -> CheckResult {
    let inputs = vec![InputFile::new("test.kul", source)];
    kul_core::check(&inputs)
}
