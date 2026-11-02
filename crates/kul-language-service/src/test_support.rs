//! Fixtures shared by the feature modules' inline tests.

use kul_core::CheckResult;
use kul_core::ast::InputFile;

/// Name of the file [`check_one`] checks.
pub(crate) const FILE: &str = "t.kul";

/// Check a project of `(name, source)` files.
pub(crate) fn check_project(files: &[(&str, &str)]) -> CheckResult {
    let inputs: Vec<InputFile> = files
        .iter()
        .map(|(name, source)| InputFile::new(*name, *source))
        .collect();
    kul_core::check(&inputs)
}

/// Check a one-file project whose file is [`FILE`].
pub(crate) fn check_one(source: &str) -> CheckResult {
    check_project(&[(FILE, source)])
}

/// Byte offset of the first occurrence of `pat` in `source`.
pub(crate) fn idx(source: &str, pat: &str) -> usize {
    source.find(pat).expect("pattern in source")
}

/// Split a fixture holding one `<CURSOR>` marker into `(source, offset)`.
pub(crate) fn cursor_fixture(s: &str) -> (String, usize) {
    let offset = s.find("<CURSOR>").expect("fixture must contain <CURSOR>");
    let source = format!("{}{}", &s[..offset], &s[offset + "<CURSOR>".len()..]);
    (source, offset)
}
