
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct FileDiff {
    pub filename: String,
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub content: String,
    pub added_lines: usize,
    pub removed_lines: usize,
    pub diff_key: Option<DiffFileKey>, // Add key for persistence
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct DiffFileKey {
    pub from_hash: String,
    pub to_hash: String,
    pub file_path: String,
}

impl FileDiff {
    /// Get appropriate nerd font icon based on file extension
    pub fn get_file_icon(&self) -> char {
        let filename = if self.filename.contains('/') {
            self.filename.split('/').next_back().unwrap_or(&self.filename)
        } else {
            &self.filename
        };
        
        crate::icons::get_file_icon(filename)
    }
    
    /// Get diff statistics as string with icons
    pub fn diff_stats(&self) -> String {
        format!(" +{} -{}", self.added_lines, self.removed_lines)
    }
}

pub struct DiffParser;

impl DiffParser {
    fn calculate_diff_stats(file_diff: &mut FileDiff, content: &str) {
        for line in content.lines() {
            if line.starts_with('+') && !line.starts_with("+++") {
                file_diff.added_lines += 1;
            } else if line.starts_with('-') && !line.starts_with("---") {
                file_diff.removed_lines += 1;
            }
        }
    }

    fn parse_index_line(line: &str) -> Option<(String, String)> {
        // Parse line like: "index abc123..def456 100644"
        if !line.starts_with("index ") {
            return None;
        }
        
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            return None;
        }
        
        let hash_part = parts[1];
        if let Some(dot_pos) = hash_part.find("..") {
            let from_hash = hash_part[..dot_pos].to_string();
            let to_hash = hash_part[dot_pos + 2..].to_string();
            Some((from_hash, to_hash))
        } else {
            None
        }
    }
    
    pub fn parse(diff_content: &str) -> Vec<FileDiff> {
        let mut file_diffs = Vec::new();
        let mut current_file: Option<FileDiff> = None;
        let mut current_content = String::new();
        
        for line in diff_content.lines() {
            if line.starts_with("diff --git") {
                // Save previous file if exists
                if let Some(mut file) = current_file.take() {
                    file.content = current_content.clone();
                    Self::calculate_diff_stats(&mut file, &current_content);
                    file_diffs.push(file);
                }
                
                // Extract filename from diff --git a/file b/file
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    let filename = parts[2].trim_start_matches("a/").to_string();
                    current_file = Some(FileDiff {
                        filename: filename.clone(),
                        old_path: Some(format!("a/{}", filename)),
                        new_path: Some(format!("b/{}", filename)),
                        content: String::new(),
                        added_lines: 0,
                        removed_lines: 0,
                        diff_key: None, // Will be set when we parse index line
                    });
                }
                current_content.clear();
            } else if line.starts_with("index ") {
                // Parse index line to extract commit hashes
                let current_hashes = Self::parse_index_line(line);
                if let (Some(file), Some((from_hash, to_hash))) = (&mut current_file, &current_hashes) {
                    file.diff_key = Some(DiffFileKey {
                        from_hash: from_hash.clone(),
                        to_hash: to_hash.clone(),
                        file_path: file.filename.clone(),
                    });
                }
            } else if let Some(stripped) = line.strip_prefix("--- ") {
                if let Some(ref mut file) = current_file {
                    file.old_path = Some(stripped.to_string());
                }
            } else if let Some(stripped) = line.strip_prefix("+++ ") {
                if let Some(ref mut file) = current_file {
                    file.new_path = Some(stripped.to_string());
                }
            }
            
            // Always append line to current content
            if current_file.is_some() {
                current_content.push_str(line);
                current_content.push('\n');
            }
        }
        
        // Don't forget the last file
        if let Some(mut file) = current_file {
            file.content = current_content.clone();
            Self::calculate_diff_stats(&mut file, &current_content);
            file_diffs.push(file);
        }
        
        file_diffs
    }
    
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_simple_diff() {
        let diff_content = r#"diff --git a/file1.rs b/file1.rs
index 1234567..abcdefg 100644
--- a/file1.rs
+++ b/file1.rs
@@ -1,3 +1,3 @@
 fn main() {
-    println!("Hello");
+    println!("Hello, World!");
 }
"#;
        
        let diffs = DiffParser::parse(diff_content);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].filename, "file1.rs");
        assert!(diffs[0].content.contains("Hello, World!"));
    }
    
    #[test]
    fn test_parse_multiple_files() {
        let diff_content = r#"diff --git a/file1.rs b/file1.rs
--- a/file1.rs
+++ b/file1.rs
@@ -1,3 +1,3 @@
-old line
+new line
diff --git a/file2.rs b/file2.rs
--- a/file2.rs
+++ b/file2.rs
@@ -1,3 +1,3 @@
-another old
+another new
"#;
        
        let diffs = DiffParser::parse(diff_content);
        assert_eq!(diffs.len(), 2);
        assert_eq!(diffs[0].filename, "file1.rs");
        assert_eq!(diffs[1].filename, "file2.rs");
    }
}