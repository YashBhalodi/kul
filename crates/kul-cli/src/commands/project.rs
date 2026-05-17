//! Shared CLI plumbing for resolving the current Kul project.
//!
//! All three subcommands (`validate`, `format`, `export`) operate on
//! the project rooted at CWD. This module folds the three steps every
//! subcommand needs — get CWD, load the project, render any
//! filesystem-level error — into one entry point so the subcommand
//! bodies stay focused on their per-command rendering work.

use std::process::ExitCode;

use kul_core::CheckResult;
use kul_loader::{LoadedProject, ProjectLoadError, load};

/// Errors the CLI's project-discovery phase surfaces. Each variant has
/// a `report` method that prints the message and returns the right
/// `ExitCode` so subcommand bodies can `return err.report()` on the
/// failure path.
pub enum ProjectRunError {
    NoProjectRoot,
    Load(ProjectLoadError),
    CwdUnavailable(std::io::Error),
}

impl ProjectRunError {
    pub fn report(self) -> ExitCode {
        match self {
            // The "not a project root" message is the load-bearing
            // user-facing error from issue #83: the user invoked
            // `kul validate` (etc.) from a directory that doesn't
            // hold a `kul.yml`. The wording matches the acceptance
            // criterion verbatim so a future doc / message audit can
            // grep for one string.
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

/// Resolve CWD and load it as a Kul project. The returned
/// [`LoadedProject`] is ready to feed into `kul_core::check`.
pub fn load_cwd_project() -> Result<LoadedProject, ProjectRunError> {
    let cwd = std::env::current_dir().map_err(ProjectRunError::CwdUnavailable)?;
    match load(&cwd) {
        Ok(p) => Ok(p),
        Err(ProjectLoadError::ManifestNotFound { .. }) => Err(ProjectRunError::NoProjectRoot),
        Err(other) => Err(ProjectRunError::Load(other)),
    }
}

/// Load the CWD project and run the full check pipeline over it. The
/// shape every subcommand wants: surface load failures as the CLI's
/// canonical messages (via [`ProjectRunError::report`]), and on success
/// hand back both the loaded inputs and the resulting [`CheckResult`]
/// so renderers can read whichever they need. The returned
/// [`LoadedProject`] retains all fields — `manifest_name` is cloned for
/// the pipeline call so callers (e.g. `kul format`) can still iterate
/// `project.inputs` after.
pub fn load_and_check() -> Result<(LoadedProject, CheckResult), ExitCode> {
    let project = load_cwd_project().map_err(ProjectRunError::report)?;
    let result = kul_core::check(
        project.manifest_name.clone(),
        &project.manifest_yaml,
        &project.inputs,
    );
    Ok((project, result))
}
