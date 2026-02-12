//! Test utilities module - provides common test helper functions

#![cfg(test)]

use std::fs as std_fs;
use std::path::Path;

/// Create a test skill in standard format
pub fn create_test_skill(dir: &Path, name: &str) -> std::io::Result<()> {
    std_fs::create_dir_all(dir)?;

    let skill_md = format!(
        r#"---
name: {}
description: Test skill for {}
license: MIT
---

# {}

Test content.
"#,
        name, name, name
    );

    std_fs::write(dir.join("SKILL.md"), skill_md)?;
    Ok(())
}

/// Create a test workspace containing multiple skills
pub fn create_test_workspace(dir: &Path) -> std::io::Result<()> {
    let skills_dir = dir.join(".claude").join("skills");
    std_fs::create_dir_all(&skills_dir)?;

    // Create test skills
    for (name, desc) in &[
        ("pdf-processing", "Process PDF files"),
        ("code-review", "Review code quality"),
        ("data-analysis", "Analyze data"),
    ] {
        let skill_dir = skills_dir.join(name);
        std_fs::create_dir_all(&skill_dir)?;

        let skill_md = format!(
            r#"---
name: {}
description: {}
---

# {}

Instructions for {}.
"#,
            name, desc, name, name
        );

        std_fs::write(skill_dir.join("SKILL.md"), skill_md)?;
    }

    Ok(())
}
