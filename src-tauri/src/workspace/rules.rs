/*!
 * Workspace Rules Management
 *
 * Finding and managing project rule files
 * Migrated from agent/context/project_context.rs
 */

use super::types::RULES_FILES;
use std::path::PathBuf;

/// Get list of all existing rule files in the specified directory
///
/// Returns existing rule file names in priority order
pub fn get_available_rules_files<P: Into<PathBuf>>(project_root: P) -> Vec<String> {
    let root: PathBuf = project_root.into();
    RULES_FILES
        .iter()
        .filter_map(|&filename| {
            let file_path = root.join(filename);
            if file_path.exists() {
                Some(filename.to_string())
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_get_available_rules_files() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create some rule files
        fs::write(temp_path.join("CLAUDE.md"), "test content").unwrap();
        fs::write(temp_path.join("README.md"), "readme content").unwrap();

        let available = get_available_rules_files(temp_path);

        assert_eq!(available.len(), 2);
        assert!(available.contains(&"CLAUDE.md".to_string()));
        assert!(available.contains(&"README.md".to_string()));
    }

    #[test]
    fn test_preserves_priority_order() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create files in reverse order
        fs::write(temp_path.join("README.md"), "readme").unwrap();
        fs::write(temp_path.join("CLAUDE.md"), "claude").unwrap();

        let available = get_available_rules_files(temp_path);

        // Should return in priority order: CLAUDE.md comes first
        assert_eq!(available[0], "CLAUDE.md");
        assert_eq!(available[1], "README.md");
    }

    #[test]
    fn test_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        let available = get_available_rules_files(temp_path);

        assert!(available.is_empty());
    }
}
