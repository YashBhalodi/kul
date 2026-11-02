//! The project a subcommand runs on: every `*.kul` file directly inside one
//! directory.
//!
//! Not recursive: subdirectories and every other file are ignored. A
//! directory with no `.kul` files is an empty project, which is a valid,
//! empty family. File order is `kul_core::check`'s to set, so this module
//! only reads bytes.

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use kul_core::CheckResult;
use kul_core::ast::InputFile;

/// The `[DIR]` argument every project subcommand takes.
#[derive(clap::Args, Debug, Clone)]
pub struct ProjectArg {
    /// Project directory: every `*.kul` file directly inside it is one
    /// project. Defaults to the current directory.
    #[arg(value_name = "DIR", default_value = ".")]
    pub dir: PathBuf,
}

/// A filesystem failure while reading a project.
#[derive(Debug)]
pub enum LoadError {
    DirectoryRead {
        path: PathBuf,
        source: std::io::Error,
    },
    FileRead {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::DirectoryRead { path, source } => {
                write!(
                    f,
                    "failed to read project directory {}: {source}",
                    path.display()
                )
            }
            LoadError::FileRead { path, source } => {
                write!(
                    f,
                    "failed to read project file {}: {source}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for LoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LoadError::DirectoryRead { source, .. } | LoadError::FileRead { source, .. } => {
                Some(source)
            }
        }
    }
}

/// Read every `*.kul` file directly inside `dir`, named by its bare file
/// name so diagnostics read the same wherever the project lives.
///
/// # Errors
///
/// [`LoadError`] when `dir` cannot be listed or a `.kul` file in it cannot
/// be read as UTF-8 text.
pub fn load(dir: &Path) -> Result<Vec<InputFile>, LoadError> {
    let directory_error = |source| LoadError::DirectoryRead {
        path: dir.to_path_buf(),
        source,
    };
    let mut inputs = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(directory_error)? {
        let entry = entry.map_err(directory_error)?;
        // A directory named `*.kul` is skipped, but a `.kul` entry that
        // cannot be inspected is read anyway so its failure surfaces.
        if entry.file_type().is_ok_and(|t| t.is_dir()) {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("kul") {
            continue;
        }
        let source = std::fs::read_to_string(&path).map_err(|source| LoadError::FileRead {
            path: path.clone(),
            source,
        })?;
        let name = entry.file_name().to_string_lossy().into_owned();
        inputs.push(InputFile::new(name, source));
    }
    Ok(inputs)
}

/// Read the project in `dir` and run the check pipeline. A read failure is
/// reported on stderr and becomes exit code 1.
pub fn load_and_check(dir: &Path) -> Result<CheckResult, ExitCode> {
    match load(dir) {
        Ok(inputs) => Ok(kul_core::check(&inputs)),
        Err(err) => {
            eprintln!("kul: {err}");
            Err(ExitCode::from(1))
        }
    }
}
