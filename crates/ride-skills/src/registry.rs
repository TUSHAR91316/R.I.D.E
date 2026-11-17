use crate::manifest::SkillManifest;
use ride_core::error::{Result, RideError};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct SkillRegistry {
    skills: HashMap<String, SkillManifest>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }

    pub fn register(&mut self, manifest: SkillManifest) {
        self.skills.insert(manifest.metadata.name.clone(), manifest);
    }

    pub fn get(&self, name: &str) -> Option<&SkillManifest> {
        self.skills.get(name)
    }

    pub fn list(&self) -> Vec<&SkillManifest> {
        self.skills.values().collect()
    }

    pub fn count(&self) -> usize {
        self.skills.len()
    }

    pub fn load_from_dir(&mut self, dir: &Path) -> Result<usize> {
        if !dir.exists() || !dir.is_dir() {
            return Ok(0);
        }

        let mut loaded = 0;
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let skill_md = if path.is_dir() {
                    path.join("SKILL.md")
                } else if path.file_name().map_or(false, |n| n == "SKILL.md") {
                    path.clone()
                } else {
                    continue;
                };

                if skill_md.exists() && skill_md.is_file() {
                    if let Ok(content) = std::fs::read_to_string(&skill_md) {
                        if let Ok(mut manifest) = SkillManifest::parse_skill_markdown(&content) {
                            manifest.source_path = Some(skill_md.to_string_lossy().to_string());
                            self.register(manifest);
                            loaded += 1;
                        }
                    }
                }
            }
        }
        Ok(loaded)
    }

    pub async fn load_from_dir_async(dir: PathBuf) -> Result<Self> {
        tokio::task::spawn_blocking(move || {
            let mut registry = Self::new();
            registry.load_from_dir(&dir)?;
            Ok(registry)
        })
        .await
        .map_err(|e| RideError::Skill(format!("Async load task failed: {e}")))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{SkillManifest, SkillMetadata};

    #[test]
    fn test_registry_registration() {
        let mut registry = SkillRegistry::new();
        let manifest = SkillManifest {
            metadata: SkillMetadata {
                name: "rust-pro".into(),
                description: "Rust specialist".into(),
                triggers: vec!["rust".into(), "cargo".into()],
                max_tokens: 1024,
                enabled: true,
            },
            instructions: "Write idiomatic Rust".into(),
            source_path: None,
        };

        registry.register(manifest);
        assert_eq!(registry.count(), 1);
        assert!(registry.get("rust-pro").is_some());
    }
}
