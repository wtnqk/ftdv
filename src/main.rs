mod cli;
mod config;
mod diff;
mod git;
mod icons;
mod parser;
mod persistence;
mod render;
mod theme;
mod tree;

use crate::cli::{Cli, OperationMode};
use crate::config::{Config, DiffCommandType};
use crate::git::GitExecutor;
use crate::parser::{DiffFileKey, DiffParser, FileDiff};
use crate::persistence::PersistenceManager;
use crate::render::{render_diff_content, render_file_list, render_search_box, render_status_line};
use crate::theme::Theme;
use crate::tree::{FileTreeBuilder, FileTreeItem};
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::ListState,
};
use std::io::{self, Read};
use std::process::{Command, Stdio};

// Constants for external tool integration
const DEFAULT_TERMINAL_HEIGHT: &str = "50";
const DEFAULT_TERMINAL_TYPE: &str = "xterm-256color";

// Template variable values for command substitution
#[derive(Debug, Clone)]
struct TemplateValues {
    width: u16,
    column_width: u16,
    diff_area_width: u16,
    diff_column_width: u16,
}

struct App {
    should_quit: bool,
    config: Config,
    theme: Theme,
    diff_output: String,
    file_tree_items: Vec<FileTreeItem>,
    original_file_diffs: Vec<FileDiff>, // Store original file diffs
    selected_index: usize,
    vertical_scroll: u16,
    horizontal_scroll: u16,
    collapsed_directories: std::collections::HashSet<String>, // Track collapsed directories
    checked_files: std::collections::HashSet<String>,         // Track checked files by path
    persistence_manager: PersistenceManager,                  // For saving/loading check states
    git_executor: Option<GitExecutor>,                        // For getting individual file diffs
    operation_mode: OperationMode,                            // Track how the app was invoked
    // Search functionality
    search_mode: bool,                           // Track if we're in search mode
    search_input_mode: bool,                     // Track if we're actively typing in search
    search_query: String,                        // Current search query
    filtered_file_tree_items: Vec<FileTreeItem>, // Filtered items for search
    // UI state
    file_list_state: ListState,                  // For stateful file tree scrolling
}

impl App {
    fn new(
        config: Config,
        file_diffs: Vec<FileDiff>,
        operation_mode: OperationMode,
    ) -> Result<Self> {
        let diff_output = if file_diffs.is_empty() {
            String::from("No diff content available")
        } else {
            file_diffs[0].content.clone()
        };

        let file_tree_items = FileTreeBuilder::build_file_tree(&file_diffs);
        let theme = config.theme.clone();

        // Initialize persistence manager
        let persistence_manager = PersistenceManager::new()?;

        // Initialize git executor if needed for interactive file viewing
        let git_executor = if operation_mode.requires_git_repo() {
            Some(GitExecutor::new())
        } else {
            None
        };

        // Load existing check states
        let diff_keys: Vec<DiffFileKey> = file_diffs
            .iter()
            .filter_map(|fd| fd.diff_key.clone())
            .collect();

        let checked_files = persistence_manager
            .load_checked_files(&diff_keys)
            .unwrap_or_else(|_| std::collections::HashSet::new());

        Ok(Self {
            should_quit: false,
            config,
            theme,
            diff_output,
            file_tree_items: file_tree_items.clone(),
            original_file_diffs: file_diffs,
            selected_index: 0,
            vertical_scroll: 0,
            horizontal_scroll: 0,
            collapsed_directories: std::collections::HashSet::new(),
            checked_files,
            persistence_manager,
            git_executor,
            operation_mode,
            search_mode: false,
            search_input_mode: false,
            search_query: String::new(),
            filtered_file_tree_items: file_tree_items,
            file_list_state: {
                let mut state = ListState::default();
                state.select(Some(0));
                state
            },
        })
    }

    fn select_next(&mut self) {
        let current_items = self.get_current_file_tree_items();
        if !current_items.is_empty() && self.selected_index < current_items.len() - 1 {
            self.selected_index += 1;
            self.file_list_state.select(Some(self.selected_index));
            self.update_diff_content();
        }
    }

    fn select_previous(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.file_list_state.select(Some(self.selected_index));
            self.update_diff_content();
        }
    }

    fn update_diff_content(&mut self) {
        let current_items = self.get_current_file_tree_items();
        if let Some(tree_item) = current_items.get(self.selected_index) {
            if let Some(file_diff) = &tree_item.file_diff {
                // Try to get individual file diff if we have a git executor
                if let Some(ref git_executor) = self.git_executor {
                    match git_executor.get_file_diff(&self.operation_mode, &tree_item.full_path) {
                        Ok(fresh_diff) => {
                            self.diff_output = fresh_diff;
                        }
                        Err(_) => {
                            // Fallback to stored diff content
                            self.diff_output = file_diff.content.clone();
                        }
                    }
                } else {
                    // Use stored diff content
                    self.diff_output = file_diff.content.clone();
                }

                // Apply external diff tool if configured
                // Use terminal width for proper side-by-side display (lazygit style)
                if let Ok((terminal_width, _)) = crossterm::terminal::size() {
                    self.apply_external_diff_tool_with_width(Some(terminal_width));
                } else {
                    self.apply_external_diff_tool();
                }

                // Reset scroll position when switching files
                self.vertical_scroll = 0;
                self.horizontal_scroll = 0;
            } else {
                // Directory selected - show directory info
                self.diff_output = format!("Directory: {}", tree_item.full_path);
                self.vertical_scroll = 0;
                self.horizontal_scroll = 0;
            }
        }
    }

    fn apply_external_diff_tool(&mut self) {
        self.apply_external_diff_tool_with_width(None);
    }

    fn apply_external_diff_tool_with_width(&mut self, width: Option<u16>) {
        // Check if we should use a diff tool (pager or external)
        match self.config.get_diff_command_type() {
            DiffCommandType::GitDefault => {
                // No processing needed
            }
            DiffCommandType::Pager(_) | DiffCommandType::External(_) => {
                match self.execute_external_diff_tool_with_width(&self.diff_output, width) {
                    Ok(processed_output) => {
                        self.diff_output = processed_output;
                    }
                    Err(e) => {
                        // Log error but continue with original output
                        eprintln!("Warning: Failed to process with diff tool: {e}");
                    }
                }
            }
        }
    }

    #[allow(dead_code)]
    fn execute_external_diff_tool(&self, diff_content: &str) -> Result<String> {
        self.execute_external_diff_tool_with_width(diff_content, None)
    }

    fn execute_external_diff_tool_with_width(
        &self,
        diff_content: &str,
        width: Option<u16>,
    ) -> Result<String> {
        let diff_command_type = self.config.get_diff_command_type();

        match diff_command_type {
            DiffCommandType::GitDefault => {
                Ok(diff_content.to_string()) // No processing needed
            }
            DiffCommandType::Pager(ref cmd) => {
                // Use stdin-based approach for pagers (delta, bat, ydiff, etc.)
                self.execute_pager_with_stdin_legacy(cmd, diff_content, width)
            }
            DiffCommandType::External(ref cmd) => {
                // Use Git's external diff mechanism for external diff tools like difftastic
                if let Some(w) = width {
                    self.execute_external_diff_via_git(cmd, w.saturating_sub(2), w)
                } else {
                    // Fallback with default widths
                    if let Ok((terminal_width, _)) = crossterm::terminal::size() {
                        self.execute_external_diff_via_git(
                            cmd,
                            terminal_width.saturating_sub(2),
                            terminal_width,
                        )
                    } else {
                        self.execute_external_diff_via_git(cmd, 78, 80)
                    }
                }
            }
        }
    }

    /// Common helper to execute external command with stdin input
    fn execute_command_with_stdin(
        &self,
        command_str: &str,
        input: &str,
        env_vars: &[(&str, String)],
    ) -> Result<String> {
        use std::io::Write;

        // Parse command and arguments
        let parts: Vec<&str> = command_str.split_whitespace().collect();
        if parts.is_empty() {
            return Err(anyhow::anyhow!("Empty command"));
        }

        let command_name = parts[0];
        let mut cmd = Command::new(command_name);

        // Add arguments
        if parts.len() > 1 {
            cmd.args(&parts[1..]);
        }

        // Set environment variables
        for (key, value) in env_vars {
            cmd.env(key, value);
        }

        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to spawn {}: {}", command_name, e))?;

        // Write input
        if let Some(stdin) = child.stdin.take() {
            let mut writer = std::io::BufWriter::new(stdin);
            writer
                .write_all(input.as_bytes())
                .map_err(|e| anyhow::anyhow!("Failed to write to command: {}", e))?;
            writer
                .flush()
                .map_err(|e| anyhow::anyhow!("Failed to flush command input: {}", e))?;
        }

        let output = child
            .wait_with_output()
            .map_err(|e| anyhow::anyhow!("Failed to read from command: {}", e))?;

        if output.status.success() {
            String::from_utf8(output.stdout)
                .map_err(|e| anyhow::anyhow!("Command output is not valid UTF-8: {}", e))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!("Command failed: {}", stderr))
        }
    }

    /// Legacy pager execution for backward compatibility with existing tools
    fn execute_pager_with_stdin_legacy(
        &self,
        command_str: &str,
        diff_content: &str,
        width: Option<u16>,
    ) -> Result<String> {
        // Apply template variable substitution
        let final_command_str = if let Some(w) = width {
            let content_width = w.saturating_sub(2);
            self.resolve_template_variables(command_str, content_width)
        } else {
            command_str.to_string()
        };

        // Prepare environment variables
        let mut env_vars = vec![
            ("TERM", DEFAULT_TERMINAL_TYPE.to_string()),
            ("LINES", DEFAULT_TERMINAL_HEIGHT.to_string()),
        ];

        if let Some(w) = width {
            env_vars.push(("COLUMNS", w.to_string()));
        }

        self.execute_command_with_stdin(&final_command_str, diff_content, &env_vars)
    }

    fn execute_external_diff_tool_with_area_width(
        &self,
        diff_content: &str,
        area_width: u16,
        terminal_width: u16,
    ) -> Result<String> {
        let diff_command_type = self.config.get_diff_command_type();

        match diff_command_type {
            DiffCommandType::GitDefault => {
                Ok(diff_content.to_string()) // No processing needed
            }
            DiffCommandType::Pager(ref cmd) => {
                // Use stdin-based approach for pagers
                self.execute_pager_with_stdin(cmd, diff_content, area_width, terminal_width)
            }
            DiffCommandType::External(ref cmd) => {
                // Use Git's external diff mechanism for external diff tools like difftastic
                self.execute_external_diff_via_git(cmd, area_width, terminal_width)
            }
        }
    }

    /// Execute pager commands via stdin (delta, bat, ydiff, etc.)
    fn execute_pager_with_stdin(
        &self,
        command_str: &str,
        diff_content: &str,
        area_width: u16,
        terminal_width: u16,
    ) -> Result<String> {
        // Apply template variable substitution with both area and terminal width
        let final_command_str = self.resolve_template_variables_with_area_width(
            command_str,
            area_width,
            terminal_width,
        );

        // Prepare environment variables
        let env_vars = vec![
            ("TERM", DEFAULT_TERMINAL_TYPE.to_string()),
            ("COLUMNS", terminal_width.to_string()),
            ("LINES", DEFAULT_TERMINAL_HEIGHT.to_string()),
        ];

        self.execute_command_with_stdin(&final_command_str, diff_content, &env_vars)
    }

    /// Setup essential environment variables for Git external diff tools
    fn setup_git_external_diff_env(
        &self,
        cmd: &mut Command,
        _area_width: u16,
        terminal_width: u16,
    ) {
        // Essential terminal environment only
        cmd.env("TERM", DEFAULT_TERMINAL_TYPE);
        cmd.env("COLUMNS", terminal_width.to_string());
        cmd.env("LINES", DEFAULT_TERMINAL_HEIGHT);
    }

    /// Execute external diff tools via Git's external diff mechanism
    fn execute_external_diff_via_git(
        &self,
        command_str: &str,
        area_width: u16,
        terminal_width: u16,
    ) -> Result<String> {
        use std::process::{Command, Stdio};

        // Apply template variable substitution
        let final_command_str = self.resolve_template_variables_with_area_width(
            command_str,
            area_width,
            terminal_width,
        );

        // Get current file path if available
        let current_items = self.get_current_file_tree_items();
        let file_path = if let Some(tree_item) = current_items.get(self.selected_index) {
            if !tree_item.is_directory {
                Some(&tree_item.full_path)
            } else {
                None
            }
        } else {
            None
        };

        if file_path.is_none() {
            return Err(anyhow::anyhow!("No file selected for external diff"));
        }

        // Build git command using external diff mechanism (like lazygit)
        let mut cmd = Command::new("git");
        let external_diff_config = format!("diff.external={final_command_str}");

        cmd.args([
            "-c",
            &external_diff_config,
            "-c",
            "diff.noprefix=false",
            "diff",
            "--ext-diff",
            "--color=always",
        ]);

        // Add operation mode specific arguments
        match &self.operation_mode {
            OperationMode::GitWorkingDirectory => {
                // Compare working directory with index
            }
            OperationMode::GitCached => {
                cmd.arg("--cached");
            }
            OperationMode::Compare { target1, target2 } => {
                cmd.arg(target1);
                cmd.arg(target2);
            }
            OperationMode::GitDiff { target } => {
                cmd.arg(target);
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "External diff not supported for this operation mode"
                ));
            }
        }

        // Add specific file path
        cmd.arg("--");
        cmd.arg(file_path.unwrap());

        // Set environment variables for git and child processes
        self.setup_git_external_diff_env(&mut cmd, area_width, terminal_width);

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let output = cmd
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to execute git with external diff: {}", e))?;

        if output.status.success() {
            String::from_utf8(output.stdout)
                .map_err(|e| anyhow::anyhow!("Git external diff output is not valid UTF-8: {}", e))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!("Git external diff failed: {}", stderr))
        }
    }

    fn scroll_up(&mut self, amount: u16) {
        self.vertical_scroll = self.vertical_scroll.saturating_sub(amount);
        // No need to clamp here - it will be clamped in render
    }

    fn scroll_down(&mut self, amount: u16) {
        self.vertical_scroll = self.vertical_scroll.saturating_add(amount);
        // No need to clamp here - it will be clamped in render
    }

    fn scroll_left(&mut self, amount: u16) {
        self.horizontal_scroll = self.horizontal_scroll.saturating_sub(amount);
        // No need to clamp here - it will be clamped in render
    }

    fn scroll_right(&mut self, amount: u16) {
        self.horizontal_scroll = self.horizontal_scroll.saturating_add(amount);
        // No need to clamp here - it will be clamped in render
    }

    fn jump_to_top(&mut self) {
        self.selected_index = 0;
        self.file_list_state.select(Some(self.selected_index));
        self.update_diff_content();
    }

    fn jump_to_bottom(&mut self) {
        let current_items = self.get_current_file_tree_items();
        if !current_items.is_empty() {
            self.selected_index = current_items.len() - 1;
            self.file_list_state.select(Some(self.selected_index));
            self.update_diff_content();
        }
    }

    fn toggle_file_checked(&mut self) {
        let current_items = if self.search_mode {
            &self.filtered_file_tree_items
        } else {
            &self.file_tree_items
        };

        if let Some(tree_item) = current_items.get(self.selected_index) {
            // Only toggle check state for files, not directories
            if !tree_item.is_directory {
                let file_path = tree_item.full_path.clone();
                let was_checked = self.checked_files.contains(&file_path);

                if was_checked {
                    self.checked_files.remove(&file_path);
                } else {
                    self.checked_files.insert(file_path.clone());
                }

                // Save to persistence if we have a diff key
                if let Some(file_diff) = tree_item.file_diff.as_ref() {
                    if let Some(diff_key) = &file_diff.diff_key {
                        let is_now_checked = !was_checked;
                        if let Err(e) = self
                            .persistence_manager
                            .save_check_state(diff_key, is_now_checked)
                        {
                            eprintln!("Warning: Failed to save check state: {e}");
                        }
                    }
                }
            }
        }
    }

    fn get_current_file_tree_items(&self) -> &Vec<FileTreeItem> {
        if self.search_mode {
            &self.filtered_file_tree_items
        } else {
            &self.file_tree_items
        }
    }

    fn enter_search_mode(&mut self) {
        if self.search_mode {
            // Already in search mode, clear query and start fresh input
            self.search_query.clear();
            self.search_input_mode = true;
            self.selected_index = 0;
            self.file_list_state.select(Some(self.selected_index));
            self.update_search_filter();
        } else {
            // Enter search mode for the first time
            self.search_mode = true;
            self.search_input_mode = true;
            self.search_query.clear();
            self.selected_index = 0;
            self.file_list_state.select(Some(self.selected_index));
            self.update_search_filter();
        }
    }

    fn exit_search_mode(&mut self) {
        self.search_mode = false;
        self.search_input_mode = false;
        self.search_query.clear();
        self.selected_index = 0;
        self.file_list_state.select(Some(self.selected_index));
        self.update_diff_content();
    }

    fn confirm_search(&mut self) {
        self.search_input_mode = false;
        // Keep search_mode = true to show filtered results
        // But allow navigation with hjkl
    }

    fn add_search_char(&mut self, c: char) {
        if self.search_input_mode {
            self.search_query.push(c);
            self.update_search_filter();
        }
    }

    fn remove_search_char(&mut self) {
        if self.search_input_mode && !self.search_query.is_empty() {
            self.search_query.pop();
            self.update_search_filter();
        }
    }

    fn update_search_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_file_tree_items = self.file_tree_items.clone();
        } else {
            // Simple fuzzy matching - each character in query should appear in order
            self.filtered_file_tree_items = self
                .file_tree_items
                .iter()
                .filter(|item| self.fuzzy_match(&item.full_path, &self.search_query))
                .cloned()
                .collect();
        }

        // Reset selection and update diff content
        self.selected_index = 0;
        self.file_list_state.select(Some(self.selected_index));
        self.update_diff_content();
    }

    fn fuzzy_match(&self, text: &str, pattern: &str) -> bool {
        // Simple substring matching like diffnav
        text.to_lowercase().contains(&pattern.to_lowercase())
    }

    fn toggle_directory(&mut self) {
        if let Some(tree_item) = self.file_tree_items.get(self.selected_index) {
            if tree_item.is_directory {
                let path = tree_item.full_path.clone();
                if self.collapsed_directories.contains(&path) {
                    self.collapsed_directories.remove(&path);
                } else {
                    self.collapsed_directories.insert(path);
                }
                // Rebuild the tree with updated collapsed state
                self.rebuild_file_tree();
            }
        }
    }

    fn rebuild_file_tree(&mut self) {
        // Use original file diffs instead of extracting from current items
        self.file_tree_items = FileTreeBuilder::build_file_tree_with_collapsed(
            &self.original_file_diffs,
            &self.collapsed_directories,
        );

        // Adjust selected index if needed
        if self.selected_index >= self.file_tree_items.len() {
            self.selected_index = self.file_tree_items.len().saturating_sub(1);
            self.file_list_state.select(Some(self.selected_index));
        }
    }

    /// Refresh diff output with specific width for side-by-side display
    fn refresh_diff_with_width(&mut self, width: u16) {
        // Re-execute diff tool with the new width for proper side-by-side alignment
        match self.config.get_diff_command_type() {
            DiffCommandType::GitDefault => {
                // No processing needed for default git diff
            }
            DiffCommandType::Pager(_) | DiffCommandType::External(_) => {
                let current_items = self.get_current_file_tree_items();
                if let Some(tree_item) = current_items.get(self.selected_index) {
                    if let Some(file_diff) = &tree_item.file_diff {
                        // Get fresh diff content for the current file
                        let base_diff = if let Some(ref git_executor) = self.git_executor {
                            match git_executor
                                .get_file_diff(&self.operation_mode, &tree_item.full_path)
                            {
                                Ok(fresh_diff) => fresh_diff,
                                Err(_) => file_diff.content.clone(),
                            }
                        } else {
                            file_diff.content.clone()
                        };

                        // Apply diff tool with width
                        match self.execute_external_diff_tool_with_width(&base_diff, Some(width)) {
                            Ok(processed_output) => {
                                self.diff_output = processed_output;
                            }
                            Err(e) => {
                                eprintln!("Warning: Failed to refresh diff with width: {e}");
                            }
                        }
                    }
                }
            }
        }
    }

    /// Refresh diff output with area width and terminal width for better template calculations
    fn refresh_diff_with_area_width(&mut self, area_width: u16, terminal_width: u16) {
        match self.config.get_diff_command_type() {
            DiffCommandType::GitDefault => {
                // No processing needed for default git diff
            }
            DiffCommandType::Pager(_) | DiffCommandType::External(_) => {
                let current_items = self.get_current_file_tree_items();
                if let Some(tree_item) = current_items.get(self.selected_index) {
                    if let Some(file_diff) = &tree_item.file_diff {
                        // Get fresh diff content for the current file
                        let base_diff = if let Some(ref git_executor) = self.git_executor {
                            match git_executor
                                .get_file_diff(&self.operation_mode, &tree_item.full_path)
                            {
                                Ok(fresh_diff) => fresh_diff,
                                Err(_) => file_diff.content.clone(),
                            }
                        } else {
                            file_diff.content.clone()
                        };

                        // Execute diff tool with area width for optimal template variable usage
                        match self.execute_external_diff_tool_with_area_width(
                            &base_diff,
                            area_width,
                            terminal_width,
                        ) {
                            Ok(processed_output) => {
                                self.diff_output = processed_output;
                            }
                            Err(e) => {
                                eprintln!("Warning: Failed to refresh diff with area width: {e}");
                            }
                        }
                    }
                }
            }
        }
    }

    /// Clamp scroll values to valid ranges based on content and viewport size
    fn clamp_scroll(&mut self, viewport_height: u16, viewport_width: u16) {
        // Calculate content dimensions
        let content_height = self.diff_output.lines().count() as u16;

        // Calculate the maximum display width, accounting for ANSI escape sequences
        let max_line_width = self
            .diff_output
            .lines()
            .map(|line| self.calculate_display_width(line))
            .max()
            .unwrap_or(0) as u16;

        // Account for borders (subtract 2 for top and bottom borders)
        let available_height = viewport_height.saturating_sub(2);
        let available_width = viewport_width.saturating_sub(2);

        // Vertical scroll limit: can't scroll beyond content
        let max_vertical_scroll = content_height.saturating_sub(available_height);

        // Horizontal scroll limit: can't scroll beyond the longest line
        let max_horizontal_scroll = max_line_width.saturating_sub(available_width);

        // Clamp the scroll values
        self.vertical_scroll = self.vertical_scroll.min(max_vertical_scroll);
        self.horizontal_scroll = self.horizontal_scroll.min(max_horizontal_scroll);
    }

    /// Calculate the display width of a line, excluding ANSI escape sequences
    fn calculate_display_width(&self, line: &str) -> usize {
        // Use strip_ansi_escapes to remove ANSI sequences, then calculate width
        if self.contains_ansi_codes(line) {
            let stripped = strip_ansi_escapes::strip(line);
            // Convert to string and calculate width
            match String::from_utf8(stripped) {
                Ok(clean_line) => self.calculate_text_width(&clean_line),
                Err(_) => line.len(), // Fallback to raw length
            }
        } else {
            self.calculate_text_width(line)
        }
    }

    /// Calculate the display width of plain text (no ANSI sequences)
    fn calculate_text_width(&self, text: &str) -> usize {
        text.chars()
            .map(|ch| {
                if ch == '\t' {
                    4 // Tab character: assume 4 spaces
                } else if ch.is_control() {
                    0 // Skip control characters
                } else {
                    1 // Regular character
                }
            })
            .sum()
    }

    /// Check if a string contains ANSI escape sequences
    pub fn contains_ansi_codes(&self, text: &str) -> bool {
        text.contains('\x1b') || text.contains("\u{001b}")
    }

    /// Calculate template variable values
    fn calculate_template_values(&self, area_width: u16, terminal_width: u16) -> TemplateValues {
        let diff_area_width = area_width.saturating_sub(2); // Remove borders
        let column_width = (terminal_width / 2).saturating_sub(6);
        let diff_column_width = (diff_area_width / 2).saturating_sub(6);

        TemplateValues {
            width: terminal_width,
            column_width,
            diff_area_width,
            diff_column_width,
        }
    }

    /// Apply template variable substitutions to command string
    fn apply_template_substitutions(&self, command_str: &str, values: &TemplateValues) -> String {
        let mut result = command_str.to_string();

        // Replace all template variable variants
        let substitutions = [
            ("{{width}}", values.width.to_string()),
            ("{{.width}}", values.width.to_string()),
            ("{{columnWidth}}", values.column_width.to_string()),
            ("{{.columnWidth}}", values.column_width.to_string()),
            ("{{diffAreaWidth}}", values.diff_area_width.to_string()),
            ("{{.diffAreaWidth}}", values.diff_area_width.to_string()),
            ("{{diffColumnWidth}}", values.diff_column_width.to_string()),
            ("{{.diffColumnWidth}}", values.diff_column_width.to_string()),
        ];

        for (template, value) in &substitutions {
            result = result.replace(template, value);
        }

        result
    }

    /// Resolve template variables in command string (lazygit style)
    fn resolve_template_variables(&self, command_str: &str, width: u16) -> String {
        let area_width = (width * 80 / 100).saturating_sub(2); // 80% minus borders
        let values = self.calculate_template_values(area_width, width);
        self.apply_template_substitutions(command_str, &values)
    }

    /// Resolve template variables with separate area and terminal widths for better precision
    fn resolve_template_variables_with_area_width(
        &self,
        command_str: &str,
        area_width: u16,
        terminal_width: u16,
    ) -> String {
        let values = self.calculate_template_values(area_width, terminal_width);
        self.apply_template_substitutions(command_str, &values)
    }
}

fn main() -> Result<()> {
    // Parse command line arguments
    let cli = Cli::parse_args();
    let operation_mode = cli.get_operation_mode();

    // Handle special modes first
    match &operation_mode {
        OperationMode::Completions { shell } => {
            generate_completions(*shell);
            return Ok(());
        }
        OperationMode::Invalid { reason } => {
            eprintln!("Error: {reason}");
            std::process::exit(1);
        }
        _ => {}
    }

    // Load configuration
    let config = if let Some(config_path) = cli.config {
        Config::load_from_path(&config_path)?
    } else {
        Config::load()?
    };

    // Check if we need a git repository
    if operation_mode.requires_git_repo() && !GitExecutor::is_git_repo() {
        return Err(anyhow::anyhow!("Not in a git repository"));
    }

    // Get diff data based on operation mode
    let is_stdin_terminal = io::IsTerminal::is_terminal(&io::stdin());
    if cli.verbose {
        eprintln!("Debug: stdin is terminal: {is_stdin_terminal}");
        eprintln!("Debug: operation mode: {operation_mode:?}");
    }

    let file_diffs = if !is_stdin_terminal {
        // Stdin mode: read piped input (backward compatibility)
        if cli.verbose {
            eprintln!("Debug: Using stdin mode");
        }
        read_input_completely().unwrap_or_else(|_| {
            if cli.verbose {
                eprintln!("Debug: No stdin input, falling back to git executor");
            }
            get_diffs_from_git(&operation_mode).unwrap_or_default()
        })
    } else {
        // Interactive mode: use git executor
        if cli.verbose {
            eprintln!("Debug: Using git executor mode");
        }
        get_diffs_from_git(&operation_mode)?
    };

    if file_diffs.is_empty() {
        println!("No differences found.");
        return Ok(());
    }

    // Initialize TUI
    enable_raw_mode()
        .map_err(|e| anyhow::anyhow!("Failed to initialize terminal raw mode: {}", e))?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = App::new(config, file_diffs, operation_mode)?;
    let res = run_app(&mut terminal, app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("{err:?}")
    }

    Ok(())
}

fn generate_completions(shell: clap_complete::Shell) {
    use clap::CommandFactory;
    use clap_complete::{Generator, generate};
    use std::io;

    fn print_completions<G: Generator>(generator: G, cmd: &mut clap::Command) {
        generate(
            generator,
            cmd,
            cmd.get_name().to_string(),
            &mut io::stdout(),
        );
    }

    let mut cmd = Cli::command();
    print_completions(shell, &mut cmd);
}

fn get_diffs_from_git(mode: &OperationMode) -> Result<Vec<FileDiff>> {
    let git_executor = GitExecutor::new();

    // Get overall diff output
    let diff_output = git_executor.get_diff(mode)?;

    if diff_output.is_empty() {
        return Ok(vec![]);
    }

    // Parse the diff output to get individual file diffs
    Ok(DiffParser::parse(&diff_output))
}

fn read_input_completely() -> Result<Vec<FileDiff>> {
    // Read all stdin content at once
    let mut buffer = String::new();
    io::stdin()
        .read_to_string(&mut buffer)
        .map_err(|e| anyhow::anyhow!("Failed to read from stdin: {}", e))?;

    if buffer.trim().is_empty() {
        anyhow::bail!("No input received from stdin");
    }

    Ok(DiffParser::parse(&buffer))
}

fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, mut app: App) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        // Use poll to handle the case where stdin might not be available
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    // Quit or exit search mode
                    KeyCode::Char('q') => {
                        if app.search_mode {
                            app.exit_search_mode();
                        } else {
                            app.should_quit = true;
                        }
                    }
                    KeyCode::Esc => {
                        if app.search_mode {
                            app.exit_search_mode();
                        } else {
                            app.should_quit = true;
                        }
                    }

                    // Search mode (use '/' key)
                    KeyCode::Char('/') if !app.search_input_mode => {
                        app.enter_search_mode();
                    }

                    // Enter to confirm search
                    KeyCode::Enter if app.search_input_mode => {
                        app.confirm_search();
                    }

                    // Backspace in search input mode
                    KeyCode::Backspace => {
                        if app.search_input_mode {
                            app.remove_search_char();
                        }
                    }

                    // File navigation (disabled only when actively typing in search)
                    KeyCode::Down | KeyCode::Char('j') if !app.search_input_mode => {
                        app.select_next()
                    }
                    KeyCode::Up | KeyCode::Char('k') if !app.search_input_mode => {
                        app.select_previous()
                    }

                    // Handle character input in search input mode (must be after other char handlers)
                    KeyCode::Char(c) if app.search_input_mode => {
                        app.add_search_char(c);
                    }
                    KeyCode::Enter => {
                        // Toggle directory expansion/collapse or update diff view
                        if let Some(tree_item) = app.file_tree_items.get(app.selected_index) {
                            if tree_item.is_directory {
                                app.toggle_directory();
                            } else {
                                app.update_diff_content();
                            }
                        }
                    }

                    // Jump navigation (disabled only when typing in search)
                    KeyCode::Char('g') if !app.search_input_mode => app.jump_to_top(),
                    KeyCode::Char('G') if !app.search_input_mode => app.jump_to_bottom(),

                    // Vertical scrolling (disabled only when typing in search)
                    KeyCode::Char('e') | KeyCode::Char('J') if !app.search_input_mode => {
                        app.scroll_down(1)
                    }
                    KeyCode::Char('y') | KeyCode::Char('K') if !app.search_input_mode => {
                        app.scroll_up(1)
                    }
                    KeyCode::Char('d') | KeyCode::PageDown if !app.search_input_mode => {
                        app.scroll_down(10)
                    }
                    KeyCode::Char('u') | KeyCode::PageUp if !app.search_input_mode => {
                        app.scroll_up(10)
                    }
                    KeyCode::Char('f') if !app.search_input_mode => app.scroll_down(20),
                    KeyCode::Char('b') if !app.search_input_mode => app.scroll_up(20),

                    // Horizontal scrolling (disabled only when typing in search)
                    KeyCode::Char('h') | KeyCode::Left if !app.search_input_mode => {
                        app.scroll_left(5)
                    }
                    KeyCode::Char('l') | KeyCode::Right if !app.search_input_mode => {
                        app.scroll_right(5)
                    }
                    KeyCode::Char('H') if !app.search_input_mode => app.scroll_left(20),
                    KeyCode::Char('L') if !app.search_input_mode => app.scroll_right(20),

                    // Space key (disabled only when typing in search)
                    KeyCode::Char(' ') if !app.search_input_mode => {
                        // File is already selected, just update view
                        app.update_diff_content();
                    }

                    // Checkbox toggle (works in both modes)
                    KeyCode::Tab => app.toggle_file_checked(),

                    _ => {}
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    // Main horizontal split: file list (30%) and diff content area (70%)
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
        .split(f.area());

    // Render search box and file list based on search mode
    if app.search_mode {
        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(main_chunks[0]);

        render_search_box(f, left_chunks[0], app);
        render_file_list(f, left_chunks[1], app);
    } else {
        render_file_list(f, main_chunks[0], app);
    }

    // Right side vertical split: status line and diff content
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(main_chunks[1]);

    render_status_line(f, right_chunks[0], app);
    render_diff_content(f, right_chunks[1], app);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::FileDiff;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    #[test]
    fn test_app_new() {
        let config = Config::default();
        let app = App::new(config, vec![], OperationMode::GitWorkingDirectory).unwrap();
        assert!(!app.should_quit);
        assert_eq!(app.selected_index, 0);
        assert_eq!(app.vertical_scroll, 0);
        assert_eq!(app.horizontal_scroll, 0);
    }

    #[test]
    fn test_ui_layout() {
        let backend = TestBackend::new(100, 50);
        let mut terminal = Terminal::new(backend).unwrap();
        let config = Config::default();
        let mut app = App::new(config, vec![], OperationMode::GitWorkingDirectory).unwrap();

        terminal.draw(|f| ui(f, &mut app)).unwrap();

        let buffer = terminal.backend().buffer();
        assert!(buffer.area().width == 100);
        assert!(buffer.area().height == 50);
    }

    #[test]
    fn test_render_file_list() {
        let backend = TestBackend::new(40, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let config = Config::default();
        let file_diffs = vec![
            FileDiff {
                filename: "test1.rs".to_string(),
                old_path: None,
                new_path: None,
                content: "test content".to_string(),
                added_lines: 1,
                removed_lines: 0,
                diff_key: None,
            },
            FileDiff {
                filename: "test2.rs".to_string(),
                old_path: None,
                new_path: None,
                content: "test content 2".to_string(),
                added_lines: 0,
                removed_lines: 1,
                diff_key: None,
            },
        ];
        let mut app = App::new(config, file_diffs, OperationMode::GitWorkingDirectory).unwrap();

        terminal
            .draw(|f| {
                let area = Rect::new(0, 0, 40, 20);
                render_file_list(f, area, &mut app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer_to_string(buffer);
        assert!(content.contains("Files & Directories"));
        assert!(content.contains("test1.rs"));
        assert!(content.contains("test2.rs"));
    }

    #[test]
    fn test_render_diff_content() {
        let backend = TestBackend::new(60, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let config = Config::default();
        let mut app = App::new(config, vec![], OperationMode::GitWorkingDirectory).unwrap();

        terminal
            .draw(|f| {
                let area = Rect::new(0, 0, 60, 20);
                render_diff_content(f, area, &mut app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer_to_string(buffer);
        assert!(content.contains("Diff Content"));
        assert!(content.contains("No diff content available"));
    }

    fn buffer_to_string(buffer: &Buffer) -> String {
        let mut result = String::new();
        for y in 0..buffer.area().height {
            for x in 0..buffer.area().width {
                let cell = buffer.cell((x, y)).unwrap();
                result.push_str(cell.symbol());
            }
            result.push('\n');
        }
        result
    }
}
