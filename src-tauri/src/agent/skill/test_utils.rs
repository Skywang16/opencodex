//! Test utilities module - provides common test helper functions

use std::fs as std_fs;
use std::path::Path;

/// Create a test skill in standard format
pub fn create_test_skill(dir: &Path, name: &str) -> std::io::Result<()> {
    std_fs::create_dir_all(dir)?;

    let skill_md = format!(
        r#"---
name: {name}
description: Test skill for {name}
license: MIT
---

# {name}

Test content.
"#
    );

    std_fs::write(dir.join("SKILL.md"), skill_md)?;
    Ok(())
}
