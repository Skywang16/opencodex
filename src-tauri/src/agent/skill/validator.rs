use std::path::Path;
use tokio::fs;

use crate::agent::agents::frontmatter::split_frontmatter;
use crate::agent::error::AgentResult;

use super::types::ValidationResult;

/// Skill validator - ensures skills conform to Agent Skills standard
pub struct SkillValidator;

impl SkillValidator {
    /// Validate if skill directory conforms to Agent Skills standard
    pub async fn validate(skill_dir: &Path) -> AgentResult<ValidationResult> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // 1. Check if it's a directory
        if !skill_dir.is_dir() {
            errors.push(format!(
                "Skill path is not a directory: {}",
                skill_dir.display()
            ));
            return Ok(ValidationResult {
                valid: false,
                errors,
                warnings,
            });
        }

        // 2. Check if SKILL.md exists
        let skill_md = skill_dir.join("SKILL.md");
        if !skill_md.exists() {
            errors.push("Missing SKILL.md file".to_string());
            return Ok(ValidationResult {
                valid: false,
                errors,
                warnings,
            });
        }

        // 3. Validate SKILL.md format
        let content = fs::read_to_string(&skill_md).await?;
        Self::validate_skill_md(&content, &mut errors, &mut warnings);

        // 4. Check subdirectories (optional but recommended)
        Self::check_optional_directories(skill_dir, &mut warnings).await;

        Ok(ValidationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
        })
    }

    /// Validate SKILL.md content format
    fn validate_skill_md(content: &str, errors: &mut Vec<String>, warnings: &mut Vec<String>) {
        let (frontmatter, body) = split_frontmatter(content);

        // Check if frontmatter exists
        if frontmatter.is_none() {
            errors.push("Missing frontmatter in SKILL.md".to_string());
            return;
        }

        let frontmatter = frontmatter.unwrap();

        // Parse frontmatter
        let parsed = crate::agent::agents::frontmatter::parse_frontmatter(frontmatter);

        // Check required fields
        if !parsed.fields.contains_key("name") {
            errors.push("Missing required field 'name' in frontmatter".to_string());
        }

        if !parsed.fields.contains_key("description") {
            errors.push("Missing required field 'description' in frontmatter".to_string());
        }

        // Check recommended fields
        if !parsed.fields.contains_key("license") {
            warnings.push("Missing recommended field 'license' in frontmatter".to_string());
        }

        // Check body content
        if body.trim().is_empty() {
            warnings.push("SKILL.md body is empty".to_string());
        }

        // Check body content length (too short may not be detailed enough)
        if body.trim().len() < 50 {
            warnings.push("SKILL.md body is very short (< 50 chars)".to_string());
        }
    }

    /// Check optional directories
    async fn check_optional_directories(skill_dir: &Path, warnings: &mut Vec<String>) {
        let optional_dirs = ["scripts", "references", "assets"];

        for dir_name in &optional_dirs {
            let dir = skill_dir.join(dir_name);
            if dir.exists() && dir.is_dir() {
                // Check if directory is empty
                if let Ok(mut entries) = fs::read_dir(&dir).await {
                    if entries.next_entry().await.ok().flatten().is_none() {
                        warnings.push(format!("Directory '{dir_name}' exists but is empty"));
                    }
                }
            }
        }
    }

    /// Quick check (only checks structure, does not read file content)
    pub async fn quick_check(skill_dir: &Path) -> bool {
        skill_dir.is_dir() && skill_dir.join("SKILL.md").exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs as std_fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_valid_skill() {
        let temp_dir = TempDir::new().unwrap();
        let skill_dir = temp_dir.path().join("valid-skill");
        std_fs::create_dir_all(&skill_dir).unwrap();

        let skill_md = r#"---
name: valid-skill
description: A valid test skill
license: MIT
---

# Valid Skill

This is a valid skill with proper structure and content.
"#;

        std_fs::write(skill_dir.join("SKILL.md"), skill_md).unwrap();

        let result = SkillValidator::validate(&skill_dir).await.unwrap();

        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[tokio::test]
    async fn test_missing_skill_md() {
        let temp_dir = TempDir::new().unwrap();
        let skill_dir = temp_dir.path().join("invalid-skill");
        std_fs::create_dir_all(&skill_dir).unwrap();

        let result = SkillValidator::validate(&skill_dir).await.unwrap();

        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("Missing SKILL.md")));
    }

    #[tokio::test]
    async fn test_missing_frontmatter() {
        let temp_dir = TempDir::new().unwrap();
        let skill_dir = temp_dir.path().join("no-frontmatter");
        std_fs::create_dir_all(&skill_dir).unwrap();

        let skill_md = "# No Frontmatter\n\nThis skill has no frontmatter.";
        std_fs::write(skill_dir.join("SKILL.md"), skill_md).unwrap();

        let result = SkillValidator::validate(&skill_dir).await.unwrap();

        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("Missing frontmatter")));
    }

    #[tokio::test]
    async fn test_missing_required_fields() {
        let temp_dir = TempDir::new().unwrap();
        let skill_dir = temp_dir.path().join("incomplete");
        std_fs::create_dir_all(&skill_dir).unwrap();

        let skill_md = r#"---
name: incomplete-skill
---

# Incomplete Skill

Missing description field.
"#;

        std_fs::write(skill_dir.join("SKILL.md"), skill_md).unwrap();

        let result = SkillValidator::validate(&skill_dir).await.unwrap();

        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("Missing required field 'description'")));
    }

    #[tokio::test]
    async fn test_quick_check() {
        let temp_dir = TempDir::new().unwrap();
        let skill_dir = temp_dir.path().join("quick-test");
        std_fs::create_dir_all(&skill_dir).unwrap();
        std_fs::write(skill_dir.join("SKILL.md"), "test").unwrap();

        assert!(SkillValidator::quick_check(&skill_dir).await);
    }
}
