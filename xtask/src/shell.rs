//! Running child processes with useful failure messages.

use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;

use thiserror::Error;

/// A task failure.
#[derive(Debug, Error)]
pub enum TaskError {
    /// A child process could not be started.
    #[error("could not start `{program}`: {reason}")]
    Spawn {
        /// Program that failed to start.
        program: String,
        /// OS message.
        reason: String,
    },
    /// A child process exited with a failure status.
    #[error("`{command}` failed with {status}")]
    Status {
        /// The full command line.
        command: String,
        /// Exit status text.
        status: String,
    },
    /// A path the task needs is missing.
    #[error("{what} not found at {path}")]
    Missing {
        /// What was being looked for.
        what: String,
        /// Where it was expected.
        path: String,
    },
    /// A filesystem operation failed.
    #[error("could not {action} {path}: {reason}")]
    Io {
        /// The attempted action.
        action: String,
        /// Path involved.
        path: String,
        /// OS message.
        reason: String,
    },
    /// A required tool is not installed.
    #[error("{tool} is required: {hint}")]
    MissingTool {
        /// Tool name.
        tool: String,
        /// How to install it.
        hint: String,
    },
}

/// Result alias for tasks.
pub type Result<T> = std::result::Result<T, TaskError>;

/// Run a command in `directory`, streaming its output.
pub fn run<I, S>(program: &str, args: I, directory: &Path) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let args: Vec<String> = args
        .into_iter()
        .map(|arg| arg.as_ref().to_string_lossy().into_owned())
        .collect();
    let printed = format!("{program} {}", args.join(" "));
    println!("  $ {printed}");

    let status = Command::new(program)
        .args(&args)
        .current_dir(directory)
        .status()
        .map_err(|err| TaskError::Spawn {
            program: program.to_string(),
            reason: err.to_string(),
        })?;

    if !status.success() {
        return Err(TaskError::Status {
            command: printed,
            status: status.to_string(),
        });
    }
    Ok(())
}

/// Run a command and capture its stdout.
pub fn capture<I, S>(program: &str, args: I, directory: &Path) -> Result<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new(program)
        .args(args)
        .current_dir(directory)
        .output()
        .map_err(|err| TaskError::Spawn {
            program: program.to_string(),
            reason: err.to_string(),
        })?;
    if !output.status.success() {
        return Err(TaskError::Status {
            command: program.to_string(),
            status: output.status.to_string(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
