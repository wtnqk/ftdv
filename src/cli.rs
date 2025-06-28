use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ftdv")]
#[command(about = "A TUI diff pager inspired by diffnav")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Git refs, files, or directories to compare
    #[arg(value_name = "REF_OR_PATH")]
    pub targets: Vec<String>,

    /// Show staged changes (equivalent to git diff --cached)
    #[arg(long, short)]
    pub cached: bool,

    /// Show changes in working directory (default)
    #[arg(long, short)]
    pub worktree: bool,

    /// Configuration file path
    #[arg(long, value_name = "FILE")]
    pub config: Option<String>,

    /// Verbose output
    #[arg(long, short)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Compare git refs, files, or directories
    Diff {
        /// First target (branch, commit, file, or directory)
        target1: String,
        /// Second target (branch, commit, file, or directory)
        target2: Option<String>,
        /// Show staged changes
        #[arg(long)]
        cached: bool,
    },
    /// Show current git status with diffs
    Status,
    /// Generate shell completions
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

impl Cli {
    pub fn parse_args() -> Self {
        Cli::parse()
    }

    /// Determine the operation mode based on arguments
    pub fn get_operation_mode(&self) -> OperationMode {
        if let Some(command) = &self.command {
            match command {
                Commands::Diff { target1, target2, cached } => {
                    if *cached {
                        OperationMode::GitCached
                    } else if let Some(target2) = target2 {
                        // Two targets: could be refs, files, or directories
                        OperationMode::Compare {
                            target1: target1.clone(),
                            target2: target2.clone(),
                        }
                    } else {
                        // One target: compare with working directory or HEAD
                        OperationMode::GitDiff { target: target1.clone() }
                    }
                }
                Commands::Status => OperationMode::GitStatus,
                Commands::Completions { shell } => OperationMode::Completions { shell: *shell },
            }
        } else if self.cached {
            OperationMode::GitCached
        } else if self.targets.is_empty() {
            // No arguments: show working directory changes
            OperationMode::GitWorkingDirectory
        } else if self.targets.len() == 1 {
            // One target: compare with working directory or HEAD
            OperationMode::GitDiff { target: self.targets[0].clone() }
        } else if self.targets.len() == 2 {
            // Two targets: compare them
            OperationMode::Compare {
                target1: self.targets[0].clone(),
                target2: self.targets[1].clone(),
            }
        } else {
            // Too many arguments
            OperationMode::Invalid { reason: "Too many arguments provided".to_string() }
        }
    }
}

#[derive(Debug, Clone)]
pub enum OperationMode {
    /// Compare working directory with HEAD
    GitWorkingDirectory,
    /// Compare staged changes with HEAD
    GitCached,
    /// Compare target with working directory or HEAD
    GitDiff { target: String },
    /// Show git status with diffs
    GitStatus,
    /// Compare two targets (refs, files, or directories)
    Compare { target1: String, target2: String },
    /// Generate shell completions
    Completions { shell: clap_complete::Shell },
    /// Invalid arguments
    Invalid { reason: String },
}

impl OperationMode {
    /// Check if this mode requires a git repository
    pub fn requires_git_repo(&self) -> bool {
        match self {
            OperationMode::GitWorkingDirectory
            | OperationMode::GitCached
            | OperationMode::GitDiff { .. }
            | OperationMode::GitStatus => true,
            OperationMode::Compare { .. }
            | OperationMode::Completions { .. }
            | OperationMode::Invalid { .. } => false,
        }
    }

    /// Get a description of this operation mode
    #[allow(dead_code)]
    pub fn description(&self) -> String {
        match self {
            OperationMode::GitWorkingDirectory => "Working directory changes".to_string(),
            OperationMode::GitCached => "Staged changes".to_string(),
            OperationMode::GitDiff { target } => format!("Changes from {}", target),
            OperationMode::GitStatus => "Git status with diffs".to_string(),
            OperationMode::Compare { target1, target2 } => {
                format!("Comparing {} with {}", target1, target2)
            }
            OperationMode::Completions { .. } => "Generating completions".to_string(),
            OperationMode::Invalid { reason } => format!("Invalid: {}", reason),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_args_gives_working_directory() {
        let cli = Cli {
            command: None,
            targets: vec![],
            cached: false,
            worktree: false,
            config: None,
            verbose: false,
        };
        
        match cli.get_operation_mode() {
            OperationMode::GitWorkingDirectory => (),
            _ => panic!("Expected GitWorkingDirectory mode"),
        }
    }

    #[test]
    fn test_cached_flag() {
        let cli = Cli {
            command: None,
            targets: vec![],
            cached: true,
            worktree: false,
            config: None,
            verbose: false,
        };
        
        match cli.get_operation_mode() {
            OperationMode::GitCached => (),
            _ => panic!("Expected GitCached mode"),
        }
    }

    #[test]
    fn test_single_target() {
        let cli = Cli {
            command: None,
            targets: vec!["branch1".to_string()],
            cached: false,
            worktree: false,
            config: None,
            verbose: false,
        };
        
        match cli.get_operation_mode() {
            OperationMode::GitDiff { target } => assert_eq!(target, "branch1"),
            _ => panic!("Expected GitDiff mode"),
        }
    }

    #[test]
    fn test_two_targets() {
        let cli = Cli {
            command: None,
            targets: vec!["branch1".to_string(), "branch2".to_string()],
            cached: false,
            worktree: false,
            config: None,
            verbose: false,
        };
        
        match cli.get_operation_mode() {
            OperationMode::Compare { target1, target2 } => {
                assert_eq!(target1, "branch1");
                assert_eq!(target2, "branch2");
            }
            _ => panic!("Expected Compare mode"),
        }
    }
}