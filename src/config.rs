use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use crate::theme::Theme;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitPagingConfig {
    /// Regular pager that processes git diff output (e.g., delta, diff-so-fancy)
    #[serde(default)]
    pub pager: String,
    
    /// External diff command that replaces git's diff algorithm (e.g., difftastic)
    #[serde(default, rename = "externalDiffCommand")]
    pub external_diff_command: String,
    
    /// Color argument passed to git diff (always/never/auto)
    #[serde(default = "default_color_arg")]
    pub color_arg: String,
    
    /// Use system-configured pager from git config
    #[serde(default)]
    pub use_config: bool,
}

fn default_color_arg() -> String {
    "always".to_string()
}

impl Default for GitPagingConfig {
    fn default() -> Self {
        Self {
            pager: String::new(),
            external_diff_command: String::new(),
            color_arg: default_color_arg(),
            use_config: false,
        }
    }
}

impl GitPagingConfig {
    /// Check if external diff command is configured
    pub fn has_external_diff_command(&self) -> bool {
        !self.external_diff_command.trim().is_empty()
    }
    
    /// Check if regular pager is configured
    pub fn has_pager(&self) -> bool {
        !self.pager.trim().is_empty()
    }
    
    /// Get the effective diff command (external diff takes precedence)
    pub fn get_effective_command(&self) -> DiffCommandType {
        if self.has_external_diff_command() {
            DiffCommandType::External(self.external_diff_command.clone())
        } else if self.has_pager() {
            DiffCommandType::Pager(self.pager.clone())
        } else {
            DiffCommandType::GitDefault
        }
    }
}

#[derive(Debug, Clone)]
pub enum DiffCommandType {
    /// Use git's default diff output
    GitDefault,
    /// Regular pager that processes git diff output
    Pager(String),
    /// External diff command that replaces git's diff algorithm
    External(String),
}

/// Legacy DiffCommand for backward compatibility
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DiffCommand {
    pub command: String,
    pub args: Vec<String>,
    #[serde(default = "default_display_name")]
    pub display_name: String,
}

fn default_display_name() -> String {
    "git diff".to_string()
}

impl Default for DiffCommand {
    fn default() -> Self {
        Self {
            command: "diff".to_string(),
            args: vec!["-u".to_string()],
            display_name: default_display_name(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct GitConfig {
    #[serde(default)]
    pub paging: GitPagingConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Config {
    #[serde(default)]
    pub git: GitConfig,
    
    /// Legacy diff_command field for backward compatibility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_command: Option<DiffCommand>,
    
    #[serde(default)]
    pub theme: Theme,
}

impl Config {
    /// Get the effective diff command, handling both new and legacy configuration
    pub fn get_diff_command_type(&self) -> DiffCommandType {
        // Check new git.paging configuration first
        let effective_command = self.git.paging.get_effective_command();
        
        match effective_command {
            DiffCommandType::GitDefault => {
                // Fall back to legacy diff_command if available
                if let Some(ref legacy_cmd) = self.diff_command {
                    if legacy_cmd.command != "diff" {
                        // Convert legacy command to pager format
                        let full_command = if legacy_cmd.args.is_empty() {
                            legacy_cmd.command.clone()
                        } else {
                            format!("{} {}", legacy_cmd.command, legacy_cmd.args.join(" "))
                        };
                        DiffCommandType::Pager(full_command)
                    } else {
                        DiffCommandType::GitDefault
                    }
                } else {
                    DiffCommandType::GitDefault
                }
            }
            _ => effective_command,
        }
    }
    
    /// Get display name for the current diff configuration
    pub fn get_diff_display_name(&self) -> String {
        match self.get_diff_command_type() {
            DiffCommandType::GitDefault => "git diff".to_string(),
            DiffCommandType::Pager(ref cmd) => {
                let tool_name = cmd.split_whitespace().next().unwrap_or("pager");
                format!("{} (pager)", tool_name)
            }
            DiffCommandType::External(ref cmd) => {
                let tool_name = cmd.split_whitespace().next().unwrap_or("external");
                format!("{} (external)", tool_name)
            }
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;
        Self::load_from_path_buf(&config_path)
    }

    pub fn load_from_path(path: &str) -> Result<Self> {
        let config_path = PathBuf::from(path);
        Self::load_from_path_buf(&config_path)
    }

    fn load_from_path_buf(config_path: &PathBuf) -> Result<Self> {
        if !config_path.exists() {
            let config = Config::default();
            if config_path == &Self::config_path()? {
                // Only auto-save if it's the default config path
                config.save()?;
            }
            return Ok(config);
        }

        let contents = fs::read_to_string(config_path)
            .with_context(|| format!("Failed to read config file: {:?}", config_path))?;

        let config: Config =
            serde_yaml::from_str(&contents).with_context(|| "Failed to parse config file")?;

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {:?}", parent))?;
        }

        let yaml = serde_yaml::to_string(self).with_context(|| "Failed to serialize config")?;

        fs::write(&config_path, yaml)
            .with_context(|| format!("Failed to write config file: {:?}", config_path))?;

        Ok(())
    }

    fn config_path() -> Result<PathBuf> {
        let home_dir = dirs::home_dir().context("Failed to get home directory")?;

        Ok(home_dir.join(".config").join("ftdv").join("config.yaml"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(config.diff_command.is_none());
        assert_eq!(config.git.paging.color_arg, "always");
        assert!(!config.git.paging.has_pager());
        assert!(!config.git.paging.has_external_diff_command());
    }

    #[test]
    fn test_config_serialize_deserialize() {
        let mut config = Config::default();
        config.git.paging.pager = "delta --dark".to_string();
        config.git.paging.color_arg = "always".to_string();
        
        let yaml = serde_yaml::to_string(&config).unwrap();
        let deserialized: Config = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(config.git.paging.pager, deserialized.git.paging.pager);
        assert_eq!(config.git.paging.color_arg, deserialized.git.paging.color_arg);
    }

    #[test]
    fn test_pager_configuration() {
        let mut config = Config::default();
        config.git.paging.pager = "delta --side-by-side".to_string();

        let yaml = serde_yaml::to_string(&config).unwrap();
        let deserialized: Config = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(deserialized.git.paging.pager, "delta --side-by-side");
        assert!(deserialized.git.paging.has_pager());
    }

    #[test]
    fn test_external_diff_command() {
        let mut config = Config::default();
        config.git.paging.external_diff_command = "difft --color=always".to_string();

        let yaml = serde_yaml::to_string(&config).unwrap();
        let deserialized: Config = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(deserialized.git.paging.external_diff_command, "difft --color=always");
        assert!(deserialized.git.paging.has_external_diff_command());
    }

    #[test]
    fn test_diff_command_type_precedence() {
        let mut config = Config::default();
        
        // Test external diff takes precedence over pager
        config.git.paging.pager = "delta".to_string();
        config.git.paging.external_diff_command = "difft".to_string();
        
        match config.get_diff_command_type() {
            DiffCommandType::External(cmd) => assert_eq!(cmd, "difft"),
            _ => panic!("Expected external diff command"),
        }
        
        // Test pager when no external diff
        config.git.paging.external_diff_command = String::new();
        match config.get_diff_command_type() {
            DiffCommandType::Pager(cmd) => assert_eq!(cmd, "delta"),
            _ => panic!("Expected pager command"),
        }
    }

    #[test]
    fn test_config_save_load() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let temp_home = temp_dir.path().to_path_buf();

        // Override home directory for testing
        unsafe {
            env::set_var("HOME", &temp_home);
        }

        let mut config = Config::default();
        config.git.paging.pager = "delta --dark".to_string();
        config.save()?;

        let loaded_config = Config::load()?;

        assert_eq!(config.git.paging.pager, loaded_config.git.paging.pager);
        assert_eq!(config.git.paging.color_arg, loaded_config.git.paging.color_arg);

        Ok(())
    }
}

