//! The project's diagnostics, each anchored at a named file.
//!
//! Related spans keep their own file, so a duplicate id declared in a sibling
//! file (R01) or a reference resolving across files points at the right one
//! (ADR-0015).

use kul_core::CheckResult;
use kul_core::diagnostic::{Diagnostic as CoreDiagnostic, Severity as CoreSeverity};
use serde::Serialize;
#[cfg(feature = "tsify")]
use tsify::Tsify;

use crate::{Location, location};

/// How serious a [`Diagnostic`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    /// The project is invalid: export and render refuse it.
    Error,
    /// Suspicious but valid.
    Warning,
    /// Information only.
    Note,
}

impl From<CoreSeverity> for Severity {
    fn from(severity: CoreSeverity) -> Self {
        match severity {
            CoreSeverity::Error => Severity::Error,
            CoreSeverity::Warning => Severity::Warning,
            CoreSeverity::Note => Severity::Note,
        }
    }
}

/// One problem the parser, resolver or validator found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify), tsify(into_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    /// Stable code: `KUL-Lxx` (lexer), `KUL-Pxx` (parser), `KUL-Rxx`
    /// (validator rule), `KUL-Vxx` (render).
    pub code: String,
    /// How serious it is.
    pub severity: Severity,
    /// The message, written for the author.
    pub message: String,
    /// Where it is. Absent for a problem with the project as a whole.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
    /// Other places that explain it (a duplicate's first declaration).
    pub related: Vec<Related>,
}

/// A secondary place a [`Diagnostic`] points at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub struct Related {
    /// What this place is (`first declared here as a person`).
    pub message: String,
    /// The place.
    pub location: Location,
}

/// Every diagnostic of the project, in the order the check produced them.
#[must_use]
pub fn diagnostics(check: &CheckResult) -> Vec<Diagnostic> {
    check
        .diagnostics
        .iter()
        .map(|d| diagnostic(check, d))
        .collect()
}

/// One core diagnostic with its spans resolved to named files.
pub(crate) fn diagnostic(check: &CheckResult, d: &CoreDiagnostic) -> Diagnostic {
    Diagnostic {
        code: d.code.to_owned(),
        severity: d.severity.into(),
        message: d.message.clone(),
        location: d.primary.and_then(|primary| location(check, primary)),
        related: d
            .related
            .iter()
            .filter_map(|r| {
                Some(Related {
                    message: r.label.clone(),
                    location: location(check, r.span)?,
                })
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Span;
    use crate::test_support::{check_one, check_project};

    #[test]
    fn clean_project_has_no_diagnostics() {
        let check = check_one("person alice name:\"A\" gender:female\n");
        assert!(diagnostics(&check).is_empty());
    }

    #[test]
    fn carries_code_severity_message_and_location() {
        let check = check_one("person alice gender:female\n");
        let all = diagnostics(&check);
        let r03 = all.iter().find(|d| d.code == "KUL-R03").expect("R03");
        assert_eq!(r03.severity, Severity::Error);
        assert!(r03.message.contains("name"));
        let location = r03.location.as_ref().expect("anchored");
        assert_eq!(location.file, "t.kul");
        assert_eq!(location.span, Span { start: 7, end: 12 });
    }

    #[test]
    fn each_diagnostic_names_its_own_file() {
        let check = check_project(&[
            ("a.kul", "person alice name:\"A\" gender:female\n"),
            ("b.kul", "person bob gender:male\n"),
        ]);
        let files: Vec<String> = diagnostics(&check)
            .into_iter()
            .filter_map(|d| d.location.map(|l| l.file))
            .collect();
        assert_eq!(files, vec!["b.kul".to_owned()]);
    }

    /// Cross-file related info (ADR-0015): a duplicate in `second.kul`
    /// points back at the first declaration in `first.kul`.
    #[test]
    fn related_info_in_sibling_file_names_the_sibling() {
        let check = check_project(&[
            ("first.kul", "person alice name:\"A\" gender:female\n"),
            ("second.kul", "person alice name:\"A2\" gender:male\n"),
        ]);
        let all = diagnostics(&check);
        let r01 = all.iter().find(|d| d.code == "KUL-R01").expect("R01");
        assert_eq!(r01.location.as_ref().unwrap().file, "second.kul");
        assert_eq!(r01.related.len(), 1);
        assert_eq!(r01.related[0].location.file, "first.kul");
        assert_eq!(r01.related[0].location.span, Span { start: 7, end: 12 });
    }

    #[test]
    fn snapshot_multi_error_fixture() {
        let check = check_project(&[
            (
                "a.kul",
                "person alice name:\"A\"\nperson alice name:\"B\" gender:male\n",
            ),
            (
                "b.kul",
                "marriage m alice ghost start:2000 end:1990\nperson kid name:\"K\" gender:other\n  birth nope\n",
            ),
        ]);
        insta::assert_json_snapshot!(diagnostics(&check));
    }
}
