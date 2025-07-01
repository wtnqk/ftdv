use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use crate::parser::DiffFileKey;

#[derive(Debug, Serialize, Deserialize)]
struct CheckState {
    checked_files: HashSet<String>,
}

pub struct PersistenceManager {
    base_dir: PathBuf,
}

impl PersistenceManager {
    pub fn new() -> Result<Self> {
        let base_dir = Self::get_base_directory()?;
        fs::create_dir_all(&base_dir)?;

        Ok(Self { base_dir })
    }

    fn get_base_directory() -> Result<PathBuf> {
        let home_dir =
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;

        Ok(home_dir.join(".local/share/ftdv/checks"))
    }

    fn get_check_file_path(&self, key: &DiffFileKey) -> PathBuf {
        // Create a safe filename from the key
        let safe_filename = format!(
            "{}_{}_{}",
            key.from_hash,
            key.to_hash,
            key.file_path.replace(['/', '\\'], "_")
        );

        self.base_dir.join(format!("{safe_filename}.json"))
    }

    pub fn load_checked_files(&self, keys: &[DiffFileKey]) -> Result<HashSet<String>> {
        let mut all_checked = HashSet::new();

        for key in keys {
            let file_path = self.get_check_file_path(key);

            if file_path.exists() {
                let content = fs::read_to_string(&file_path)?;
                let check_state: CheckState = serde_json::from_str(&content)
                    .map_err(|e| anyhow::anyhow!("Failed to parse check state: {}", e))?;

                // Add the file path to checked set if it was checked
                if check_state.checked_files.contains(&key.file_path) {
                    all_checked.insert(key.file_path.clone());
                }
            }
        }

        Ok(all_checked)
    }

    pub fn save_check_state(&self, key: &DiffFileKey, is_checked: bool) -> Result<()> {
        let file_path = self.get_check_file_path(key);

        let mut checked_files = HashSet::new();
        if is_checked {
            checked_files.insert(key.file_path.clone());
        }

        let check_state = CheckState { checked_files };
        let content = serde_json::to_string_pretty(&check_state)?;

        fs::write(&file_path, content)
            .map_err(|e| anyhow::anyhow!("Failed to write check state: {}", e))?;

        Ok(())
    }

    #[allow(dead_code)]
    pub fn remove_check_state(&self, key: &DiffFileKey) -> Result<()> {
        let file_path = self.get_check_file_path(key);

        if file_path.exists() {
            fs::remove_file(&file_path)
                .map_err(|e| anyhow::anyhow!("Failed to remove check state: {}", e))?;
        }

        Ok(())
    }

    // Optional: cleanup old check files
    #[allow(dead_code)]
    pub fn cleanup_old_files(&self, max_age_days: u64) -> Result<()> {
        use std::time::{Duration, SystemTime};

        let max_age = Duration::from_secs(max_age_days * 24 * 60 * 60);
        let cutoff_time = SystemTime::now() - max_age;

        for entry in fs::read_dir(&self.base_dir)? {
            let entry = entry?;
            let metadata = entry.metadata()?;

            if let Ok(modified) = metadata.modified() {
                if modified < cutoff_time {
                    fs::remove_file(entry.path())?;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_manager() -> (PersistenceManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let manager = PersistenceManager {
            base_dir: temp_dir.path().to_path_buf(),
        };
        (manager, temp_dir)
    }

    #[test]
    fn test_save_and_load_check_state() {
        let (manager, _temp_dir) = create_test_manager();

        let key = DiffFileKey {
            from_hash: "abc123".to_string(),
            to_hash: "def456".to_string(),
            file_path: "src/main.rs".to_string(),
        };

        // Save checked state
        manager.save_check_state(&key, true).unwrap();

        // Load and verify
        let checked = manager.load_checked_files(&[key.clone()]).unwrap();
        assert!(checked.contains("src/main.rs"));

        // Save unchecked state
        manager.save_check_state(&key, false).unwrap();

        // Load and verify
        let checked = manager.load_checked_files(&[key]).unwrap();
        assert!(!checked.contains("src/main.rs"));
    }

    #[test]
    fn test_file_path_safety() {
        let (manager, _temp_dir) = create_test_manager();

        let key = DiffFileKey {
            from_hash: "abc123".to_string(),
            to_hash: "def456".to_string(),
            file_path: "deep/path/with/slashes.rs".to_string(),
        };

        let file_path = manager.get_check_file_path(&key);
        let filename = file_path.file_name().unwrap().to_str().unwrap();

        // Should not contain slashes in filename
        assert!(!filename.contains('/'));
        assert!(filename.contains("deep_path_with_slashes.rs"));
    }
}
