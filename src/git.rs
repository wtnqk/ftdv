use crate::cli::OperationMode;
use anyhow::{Context, Result, anyhow};
use std::path::Path;
use std::process::Command;

/// Git command executor for getting diff data
pub struct GitExecutor;

impl GitExecutor {
    pub fn new() -> Self {
        Self
    }

    /// Check if we're in a git repository
    pub fn is_git_repo() -> bool {
        Command::new("git")
            .args(["rev-parse", "--git-dir"])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Get diff output based on operation mode
    pub fn get_diff(&self, mode: &OperationMode) -> Result<String> {
        match mode {
            OperationMode::GitWorkingDirectory => self.execute_git_diff(&["diff"]),
            OperationMode::GitCached => self.execute_git_diff(&["diff", "--cached"]),
            OperationMode::GitDiff { target } => self.execute_git_diff(&["diff", target]),
            OperationMode::GitStatus => {
                // For status, we might want to show multiple diffs
                self.execute_git_diff(&["diff"])
            }
            OperationMode::Compare { target1, target2 } => {
                // Check if both targets are git refs
                if self.is_git_ref(target1)? && self.is_git_ref(target2)? {
                    self.execute_git_diff(&["diff", &format!("{target1}..{target2}")])
                } else {
                    // Fall back to regular diff for files/directories
                    self.execute_regular_diff(target1, target2)
                }
            }
            OperationMode::Completions { .. } => {
                Err(anyhow!("Completions mode should not call get_diff"))
            }
            OperationMode::Invalid { reason } => Err(anyhow!("Invalid operation mode: {}", reason)),
        }
    }

    /// Get list of files that have changes
    #[allow(dead_code)]
    pub fn get_changed_files(&self, mode: &OperationMode) -> Result<Vec<String>> {
        match mode {
            OperationMode::GitWorkingDirectory => {
                self.execute_git_name_only(&["diff", "--name-only"])
            }
            OperationMode::GitCached => {
                self.execute_git_name_only(&["diff", "--cached", "--name-only"])
            }
            OperationMode::GitDiff { target } => {
                self.execute_git_name_only(&["diff", "--name-only", target])
            }
            OperationMode::GitStatus => self.execute_git_name_only(&["diff", "--name-only"]),
            OperationMode::Compare { target1, target2 } => {
                if self.is_git_ref(target1)? && self.is_git_ref(target2)? {
                    self.execute_git_name_only(&[
                        "diff",
                        "--name-only",
                        &format!("{target1}..{target2}"),
                    ])
                } else {
                    // For file/directory comparison, return the file paths
                    Ok(vec![target1.clone(), target2.clone()])
                }
            }
            OperationMode::Completions { .. } => Err(anyhow!(
                "Completions mode should not call get_changed_files"
            )),
            OperationMode::Invalid { reason } => Err(anyhow!("Invalid operation mode: {}", reason)),
        }
    }

    /// Get diff for a specific file
    pub fn get_file_diff(&self, mode: &OperationMode, file_path: &str) -> Result<String> {
        match mode {
            OperationMode::GitWorkingDirectory => self.execute_git_diff(&["diff", "--", file_path]),
            OperationMode::GitCached => {
                self.execute_git_diff(&["diff", "--cached", "--", file_path])
            }
            OperationMode::GitDiff { target } => {
                self.execute_git_diff(&["diff", target, "--", file_path])
            }
            OperationMode::GitStatus => self.execute_git_diff(&["diff", "--", file_path]),
            OperationMode::Compare { target1, target2 } => {
                if self.is_git_ref(target1)? && self.is_git_ref(target2)? {
                    self.execute_git_diff(&[
                        "diff",
                        &format!("{target1}..{target2}"),
                        "--",
                        file_path,
                    ])
                } else {
                    // For file comparison, assume the file_path is one of the targets
                    self.execute_regular_diff(target1, target2)
                }
            }
            OperationMode::Completions { .. } => {
                Err(anyhow!("Completions mode should not call get_file_diff"))
            }
            OperationMode::Invalid { reason } => Err(anyhow!("Invalid operation mode: {}", reason)),
        }
    }

    /// Execute git diff command
    fn execute_git_diff(&self, args: &[&str]) -> Result<String> {
        let output = Command::new("git")
            .args(args)
            .output()
            .context("Failed to execute git diff")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Git diff failed: {}", stderr));
        }

        String::from_utf8(output.stdout).context("Git diff output is not valid UTF-8")
    }

    /// Execute git command to get file names only
    #[allow(dead_code)]
    fn execute_git_name_only(&self, args: &[&str]) -> Result<Vec<String>> {
        let output = Command::new("git")
            .args(args)
            .output()
            .context("Failed to execute git diff --name-only")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Git diff --name-only failed: {}", stderr));
        }

        let stdout = String::from_utf8(output.stdout).context("Git output is not valid UTF-8")?;

        Ok(stdout
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| line.to_string())
            .collect())
    }

    /// Execute regular diff command for non-git files
    fn execute_regular_diff(&self, file1: &str, file2: &str) -> Result<String> {
        let output = Command::new("diff")
            .args(["-u", file1, file2])
            .output()
            .context("Failed to execute diff")?;

        // diff returns exit code 1 when files differ, which is normal
        if output.status.code() == Some(2) {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Diff command failed: {}", stderr));
        }

        String::from_utf8(output.stdout).context("Diff output is not valid UTF-8")
    }

    /// Check if a string is a valid git ref
    fn is_git_ref(&self, ref_name: &str) -> Result<bool> {
        // First check if it's a file or directory path
        if Path::new(ref_name).exists() {
            return Ok(false);
        }

        // Check if git can resolve it as a ref
        let output = Command::new("git")
            .args(["rev-parse", "--verify", ref_name])
            .output()
            .context("Failed to check git ref")?;

        Ok(output.status.success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_executor_creation() {
        let _executor = GitExecutor::new();
        // Just test that we can create it without panicking
    }

    #[test]
    fn test_is_git_repo() {
        // This test will pass if run in a git repository
        // In a non-git directory, it should return false
        let result = GitExecutor::is_git_repo();
        // We can't assert a specific value since it depends on test environment
        // Just ensure it returns a boolean without panicking
        let _is_boolean = matches!(result, true | false);
    }
}
