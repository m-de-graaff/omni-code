//! External code formatter runner.

use std::path::Path;
use std::process::Command;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum FormatterError {
    #[error("formatter command failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("formatter exited with code {code}: {stderr}")]
    NonZeroExit { code: i32, stderr: String },
    #[error("no formatter configured for language")]
    NotConfigured,
}

/// Run an external formatter on the given content.
///
/// The `command` string is split on whitespace. `{file}` is replaced
/// with the file path. Content is piped to stdin, formatted output
/// is read from stdout.
///
/// # Errors
/// Returns an error if the command fails or exits non-zero.
pub fn format_buffer(
    content: &str,
    command: &str,
    file_path: Option<&Path>,
) -> Result<String, FormatterError> {
    let parts: Vec<String> = command
        .split_whitespace()
        .map(|s| {
            if s == "{file}" {
                file_path
                    .and_then(|p| p.to_str())
                    .unwrap_or("")
                    .to_string()
            } else {
                s.to_string()
            }
        })
        .collect();

    if parts.is_empty() {
        return Err(FormatterError::NotConfigured);
    }

    let mut cmd = Command::new(&parts[0]);
    for arg in &parts[1..] {
        cmd.arg(arg);
    }

    cmd.stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn()?;

    // Write content to stdin
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin.write_all(content.as_bytes())?;
    }

    let output = child.wait_with_output()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        Err(FormatterError::NonZeroExit {
            code: output.status.code().unwrap_or(-1),
            stderr,
        })
    }
}
