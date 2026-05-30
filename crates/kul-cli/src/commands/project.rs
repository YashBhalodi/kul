//! Shared CLI plumbing for resolving the current Kul project from CWD.

use std::process::ExitCode;

use kul_core::CheckResult;
use kul_loader::{LoadedProject, ProjectLoadError, load};

/// Errors from the CLI's project-discovery phase. Use [`Self::report`]
/// to print the message and obtain the exit code.
pub enum ProjectRunError {
    NoProjectRoot,
    Load(ProjectLoadError),
    CwdUnavailable(std::io::Error),
}

impl ProjectRunError {
    pub fn report(self) -> ExitCode {
        match self {
            // Wording is the user-facing contract — kept verbatim for grep-ability.
            ProjectRunError::NoProjectRoot => {
                eprintln!("kul: not a Kul project root: no kul.yml in current directory");
            }
            ProjectRunError::Load(err) => {
                eprintln!("kul: {err}");
            }
            ProjectRunError::CwdUnavailable(err) => {
                eprintln!("kul: failed to read current working directory: {err}");
            }
        }
        ExitCode::from(1)
    }
}

/// Resolve CWD and load it as a Kul project.
pub fn load_cwd_project() -> Result<LoadedProject, ProjectRunError> {
    let cwd = std::env::current_dir().map_err(ProjectRunError::CwdUnavailable)?;
    match load(&cwd) {
        Ok(p) => Ok(p),
        Err(ProjectLoadError::ManifestNotFound { .. }) => Err(ProjectRunError::NoProjectRoot),
        Err(other) => Err(ProjectRunError::Load(other)),
    }
}

/// Load the CWD project and run the check pipeline. Load failures are
/// reported to stderr; on success returns both the loaded inputs and
/// the [`CheckResult`].
pub fn load_and_check() -> Result<(LoadedProject, CheckResult), ExitCode> {
    let project = load_cwd_project().map_err(ProjectRunError::report)?;
    let result = kul_core::check(
        project.manifest_name.clone(),
        &project.manifest_yaml,
        &project.inputs,
    );
    Ok((project, result))
}
