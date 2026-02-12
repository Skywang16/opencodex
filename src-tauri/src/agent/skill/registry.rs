use dashmap::DashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::fs;

use crate::agent::error::{AgentError, AgentResult};

use super::loader::SkillLoader;
use super::types::{SkillContent, SkillEntry, SkillMetadata};

/// Skill registry - thread-safe skill storage
/// Uses DashMap to provide high-performance concurrent access
pub struct SkillRegistry {
    /// skill_name -> SkillEntry
    skills: DashMap<String, SkillEntry>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: DashMap::new(),
        }
    }

    /// Register a skill metadata
    pub fn register(&self, metadata: SkillMetadata) -> AgentResult<()> {
        let name = metadata.name.clone();

        // Get file modification time
        let last_modified = std::fs::metadata(metadata.skill_dir.join("SKILL.md"))
            .ok()
            .and_then(|m| m.modified().ok())
            .unwrap_or_else(SystemTime::now);

        let entry = SkillEntry {
            metadata,
            content: None,
            last_modified,
        };

        self.skills.insert(name.to_string(), entry);
        Ok(())
    }

    /// Get skill metadata
    pub fn get_metadata(&self, name: &str) -> Option<SkillMetadata> {
        self.skills.get(name).map(|entry| entry.metadata.clone())
    }

    /// Get all skill metadata
    pub fn list_all(&self) -> Vec<SkillMetadata> {
        self.skills
            .iter()
            .map(|entry| entry.metadata.clone())
            .collect()
    }

    /// Check if skill exists
    pub fn contains(&self, name: &str) -> bool {
        self.skills.contains_key(name)
    }

    /// Get or load skill content
    pub async fn get_or_load_content(&self, name: &str) -> AgentResult<SkillContent> {
        // Fast path: check cache
        if let Some(entry) = self.skills.get(name) {
            if let Some(content) = &entry.content {
                return Ok(content.clone());
            }
        }

        // Slow path: load content
        let metadata = self
            .get_metadata(name)
            .ok_or_else(|| AgentError::SkillNotFound(name.to_string()))?;

        let content = SkillLoader::load_content(&metadata).await?;

        // Update cache
        if let Some(mut entry) = self.skills.get_mut(name) {
            entry.content = Some(content.clone());
        }

        Ok(content)
    }

    /// Clear skill content cache (preserves metadata)
    pub fn clear_content_cache(&self, name: &str) {
        if let Some(mut entry) = self.skills.get_mut(name) {
            entry.content = None;
        }
    }

    /// Clear entire registry
    pub fn clear(&self) {
        self.skills.clear();
    }

    /// Reload skill (checks file modification time)
    pub async fn reload_if_modified(&self, name: &str) -> AgentResult<bool> {
        let entry = self
            .skills
            .get(name)
            .ok_or_else(|| AgentError::SkillNotFound(name.to_string()))?;

        let skill_md = entry.metadata.skill_dir.join("SKILL.md");
        let current_modified = fs::metadata(&skill_md)
            .await?
            .modified()
            .unwrap_or_else(|_| SystemTime::now());

        if current_modified > entry.last_modified {
            // Reload
            drop(entry);
            self.clear_content_cache(name);

            let skill_dir = self.get_metadata(name).unwrap().skill_dir;
            let new_metadata = SkillLoader::load_metadata(&skill_dir).await?;
            self.register(new_metadata)?;

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Batch get skill contents
    pub async fn get_multiple_contents(&self, names: &[String]) -> AgentResult<Vec<SkillContent>> {
        let mut contents = Vec::with_capacity(names.len());

        for name in names {
            match self.get_or_load_content(name).await {
                Ok(content) => contents.push(content),
                Err(e) => {
                    tracing::warn!("Failed to load skill '{}': {}", name, e);
                }
            }
        }

        Ok(contents)
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenient Arc-wrapped version
pub type SkillRegistryRef = Arc<SkillRegistry>;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // Use common test utilities
    use crate::agent::skill::test_utils::create_test_skill;

    #[tokio::test]
    async fn test_register_and_get() {
        let temp_dir = TempDir::new().unwrap();
        let skill_dir = temp_dir.path().join("test-skill");
        create_test_skill(&skill_dir, "test-skill").unwrap();

        let registry = SkillRegistry::new();
        let metadata = SkillLoader::load_metadata(&skill_dir).await.unwrap();

        registry.register(metadata).unwrap();

        assert!(registry.contains("test-skill"));
        assert!(registry.get_metadata("test-skill").is_some());
    }

    #[tokio::test]
    async fn test_get_or_load_content() {
        let temp_dir = TempDir::new().unwrap();
        let skill_dir = temp_dir.path().join("test-skill");
        create_test_skill(&skill_dir, "test-skill").unwrap();

        let registry = SkillRegistry::new();
        let metadata = SkillLoader::load_metadata(&skill_dir).await.unwrap();
        registry.register(metadata).unwrap();

        // First load
        let content1 = registry.get_or_load_content("test-skill").await.unwrap();
        assert!(content1.instructions.contains("Test content"));

        // Second time should load from cache
        let content2 = registry.get_or_load_content("test-skill").await.unwrap();
        assert_eq!(content1.instructions, content2.instructions);
    }

    #[tokio::test]
    async fn test_list_all() {
        let temp_dir = TempDir::new().unwrap();
        let registry = SkillRegistry::new();

        for i in 1..=3 {
            let name = format!("skill-{i}");
            let skill_dir = temp_dir.path().join(&name);
            create_test_skill(&skill_dir, &name).unwrap();

            let metadata = SkillLoader::load_metadata(&skill_dir).await.unwrap();
            registry.register(metadata).unwrap();
        }

        let all = registry.list_all();
        assert_eq!(all.len(), 3);
    }
}
