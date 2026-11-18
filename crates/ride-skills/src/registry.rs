use crate::dynamic_tools::{DynamicSkillTool, DynamicToolRegistry};
use crate::manifest::SkillManifest;
use ride_core::error::{Result, RideError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillScope {
    Global,
    Workspace,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillCollision {
    pub skill_name: String,
    pub global_path: PathBuf,
    pub workspace_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredSkill {
    pub manifest: SkillManifest,
    pub scope: SkillScope,
    pub overridden: bool,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ScanSummary {
    pub global_count: usize,
    pub workspace_count: usize,
    pub collisions: Vec<SkillCollision>,
}

#[derive(Default, Clone)]
pub struct SkillRegistry {
    /// Active effective skills (workspace overrides global on collision)
    effective_skills: HashMap<String, SkillManifest>,
    /// Global skills loaded
    global_skills: HashMap<String, SkillManifest>,
    /// Workspace skills loaded
    workspace_skills: HashMap<String, SkillManifest>,
    /// Collision records
    collisions: Vec<SkillCollision>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Default global skills directory (~/.ride/skills)
    pub fn default_global_dir() -> PathBuf {
        let home = if let Ok(up) = std::env::var("USERPROFILE") {
            if !up.trim().is_empty() {
                PathBuf::from(up)
            } else {
                dirs_home()
            }
        } else {
            dirs_home()
        };
        home.join(".ride").join("skills")
    }

    /// Workspace-local skills directory (<workspace_root>/.ride/skills)
    pub fn workspace_skills_dir(workspace_root: impl AsRef<Path>) -> PathBuf {
        workspace_root.as_ref().join(".ride").join("skills")
    }

    /// Register a skill into the registry under a specified scope.
    /// If scope is Workspace and a Global skill with the same name exists,
    /// a collision is recorded and the Workspace skill overrides it as the effective skill.
    pub fn register(&mut self, manifest: SkillManifest, scope: SkillScope) {
        let name = manifest.metadata.name.clone();

        match scope {
            SkillScope::Global => {
                let is_shadowed_by_workspace = self.workspace_skills.contains_key(&name);
                if is_shadowed_by_workspace {
                    if let Some(ws) = self.workspace_skills.get(&name) {
                        let col = SkillCollision {
                            skill_name: name.clone(),
                            global_path: manifest.source_path.clone().unwrap_or_default(),
                            workspace_path: ws.source_path.clone().unwrap_or_default(),
                        };
                        if !self.collisions.iter().any(|c| c.skill_name == name) {
                            self.collisions.push(col);
                        }
                    }
                } else {
                    self.effective_skills.insert(name.clone(), manifest.clone());
                }
                self.global_skills.insert(name, manifest);
            }
            SkillScope::Workspace => {
                if let Some(existing_global) = self.global_skills.get(&name) {
                    let col = SkillCollision {
                        skill_name: name.clone(),
                        global_path: existing_global.source_path.clone().unwrap_or_default(),
                        workspace_path: manifest.source_path.clone().unwrap_or_default(),
                    };
                    self.collisions.retain(|c| c.skill_name != name);
                    self.collisions.push(col);
                    tracing::info!(
                        "Workspace skill '{}' overrides global skill at {:?}",
                        name,
                        existing_global.source_path
                    );
                }
                self.effective_skills.insert(name.clone(), manifest.clone());
                self.workspace_skills.insert(name, manifest);
            }
        }
    }

    /// Get active effective skill by name
    pub fn get(&self, name: &str) -> Option<&SkillManifest> {
        self.effective_skills.get(name)
    }

    /// Get active effective skill mutably by name
    pub fn get_mut(&mut self, name: &str) -> Option<&mut SkillManifest> {
        self.effective_skills.get_mut(name)
    }

    /// Check if a skill exists in global scope
    pub fn get_global(&self, name: &str) -> Option<&SkillManifest> {
        self.global_skills.get(name)
    }

    /// Check if a skill exists in workspace scope
    pub fn get_workspace(&self, name: &str) -> Option<&SkillManifest> {
        self.workspace_skills.get(name)
    }

    /// Returns whether a skill was overridden by workspace
    pub fn is_overridden(&self, name: &str) -> bool {
        self.collisions.iter().any(|c| c.skill_name == name)
    }

    /// Toggle skill enabled status
    pub fn enable_skill(&mut self, name: &str, enabled: bool) -> bool {
        let mut changed = false;
        if let Some(skill) = self.effective_skills.get_mut(name) {
            skill.metadata.enabled = enabled;
            changed = true;
        }
        if let Some(skill) = self.workspace_skills.get_mut(name) {
            skill.metadata.enabled = enabled;
        }
        if let Some(skill) = self.global_skills.get_mut(name) {
            skill.metadata.enabled = enabled;
        }
        changed
    }

    /// Toggle skill pinned status
    pub fn set_pinned(&mut self, name: &str, pinned: bool) -> bool {
        let mut changed = false;
        if let Some(skill) = self.effective_skills.get_mut(name) {
            skill.metadata.pinned = pinned;
            changed = true;
        }
        if let Some(skill) = self.workspace_skills.get_mut(name) {
            skill.metadata.pinned = pinned;
        }
        if let Some(skill) = self.global_skills.get_mut(name) {
            skill.metadata.pinned = pinned;
        }
        changed
    }

    /// List all active effective skills
    pub fn list(&self) -> Vec<&SkillManifest> {
        self.effective_skills.values().collect()
    }

    /// List registered skills with metadata on scope and collision
    pub fn list_registered(&self) -> Vec<RegisteredSkill> {
        let mut out = Vec::new();
        for (_name, skill) in &self.workspace_skills {
            out.push(RegisteredSkill {
                manifest: skill.clone(),
                scope: SkillScope::Workspace,
                overridden: false,
            });
        }
        for (name, skill) in &self.global_skills {
            let overridden = self.workspace_skills.contains_key(name);
            out.push(RegisteredSkill {
                manifest: skill.clone(),
                scope: SkillScope::Global,
                overridden,
            });
        }
        out
    }

    /// Returns recorded collisions where workspace shadowed global
    pub fn collisions(&self) -> &[SkillCollision] {
        &self.collisions
    }

    /// Total count of active effective skills
    pub fn count(&self) -> usize {
        self.effective_skills.len()
    }

    /// Builds a dynamic tool registry from all enabled skills in the registry
    pub fn build_dynamic_tools(&self) -> DynamicToolRegistry {
        let mut registry = DynamicToolRegistry::new();
        for skill in self.effective_skills.values() {
            if !skill.metadata.enabled {
                continue;
            }
            let base_dir = skill
                .base_dir
                .clone()
                .or_else(|| {
                    skill
                        .source_path
                        .as_ref()
                        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
                })
                .unwrap_or_else(|| PathBuf::from("."));

            for tool_def in &skill.tools {
                let dynamic_tool = DynamicSkillTool::new(tool_def.clone(), &base_dir);
                registry.register(dynamic_tool);
            }
        }
        registry
    }

    /// Scan a directory containing skill subdirectories or SKILL.md
    pub fn scan_dir(&mut self, dir: &Path, scope: SkillScope) -> Result<usize> {
        if !dir.exists() || !dir.is_dir() {
            return Ok(0);
        }

        let mut loaded = 0;
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) => {
                return Err(RideError::Skill(format!(
                    "Failed to read skills directory {}: {e}",
                    dir.display()
                )))
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Look for SKILL.md inside subdirectory
                if let Ok(manifest) = SkillManifest::load_from_directory(&path) {
                    self.register(manifest, scope);
                    loaded += 1;
                }
            } else if path.file_name().map_or(false, |n| {
                n.to_string_lossy().eq_ignore_ascii_case("skill.md")
            }) {
                // If a SKILL.md is directly in dir
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(mut manifest) = SkillManifest::parse_skill_markdown(&content) {
                        manifest.source_path = Some(path.clone());
                        manifest.base_dir = path.parent().map(|p| p.to_path_buf());
                        self.register(manifest, scope);
                        loaded += 1;
                    }
                }
            }
        }

        Ok(loaded)
    }

    /// Scan global skills directory (~/.ride/skills)
    pub fn scan_global(&mut self, global_dir: &Path) -> Result<usize> {
        self.scan_dir(global_dir, SkillScope::Global)
    }

    /// Load skills from directory (backwards-compatible alias for scan_global)
    pub fn load_from_dir(&mut self, dir: &Path) -> Result<usize> {
        self.scan_dir(dir, SkillScope::Global)
    }

    /// Async load from directory (backwards-compatible alias)
    pub async fn load_from_dir_async(dir: PathBuf) -> Result<Self> {
        tokio::task::spawn_blocking(move || {
            let mut reg = Self::new();
            reg.load_from_dir(&dir)?;
            Ok(reg)
        })
        .await
        .map_err(|e| RideError::Skill(format!("Async load task failed: {e}")))?
    }

    /// Scan workspace skills directory (<workspace_root>/.ride/skills)
    pub fn scan_workspace(&mut self, workspace_root: &Path) -> Result<usize> {
        let ws_skills = Self::workspace_skills_dir(workspace_root);
        if ws_skills.exists() {
            self.scan_dir(&ws_skills, SkillScope::Workspace)
        } else {
            // Also check if workspace_root itself has skills
            self.scan_dir(workspace_root, SkillScope::Workspace)
        }
    }

    /// Perform a full dual-scope scan loading global first, then workspace
    pub fn scan_dual(
        &mut self,
        global_dir: &Path,
        workspace_root: Option<&Path>,
    ) -> Result<ScanSummary> {
        let global_count = self.scan_global(global_dir)?;
        let mut workspace_count = 0;
        if let Some(ws_root) = workspace_root {
            workspace_count = self.scan_workspace(ws_root)?;
        }

        Ok(ScanSummary {
            global_count,
            workspace_count,
            collisions: self.collisions.clone(),
        })
    }

    /// Async dual-scope scan using background blocking thread
    pub async fn scan_dual_async(
        global_dir: PathBuf,
        workspace_root: Option<PathBuf>,
    ) -> Result<Self> {
        tokio::task::spawn_blocking(move || {
            let mut reg = Self::new();
            let _ = reg.scan_dual(&global_dir, workspace_root.as_deref())?;
            Ok(reg)
        })
        .await
        .map_err(|e| RideError::Skill(format!("Async scan task failed: {e}")))?
    }
}

fn dirs_home() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        if !home.trim().is_empty() {
            return PathBuf::from(home);
        }
    }
    PathBuf::from(".")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::SkillMetadata;

    fn make_test_manifest(name: &str, instructions: &str) -> SkillManifest {
        SkillManifest {
            metadata: SkillMetadata {
                name: name.into(),
                description: format!("Description of {name}"),
                version: "0.1.0".into(),
                author: None,
                triggers: vec![name.into()],
                max_tokens: 1024,
                pinned: false,
                compatible_models: vec![],
                enabled: true,
                tools: vec![],
            },
            instructions: instructions.into(),
            source_path: None,
            base_dir: None,
            tools: vec![],
        }
    }

    #[test]
    fn test_registry_registration_and_get() {
        let mut registry = SkillRegistry::new();
        let manifest = make_test_manifest("rust-pro", "Write idiomatic Rust");

        registry.register(manifest, SkillScope::Global);
        assert_eq!(registry.count(), 1);
        assert!(registry.get("rust-pro").is_some());
        assert_eq!(registry.get("rust-pro").unwrap().instructions, "Write idiomatic Rust");
    }

    #[test]
    fn test_workspace_overrides_global() {
        let mut registry = SkillRegistry::new();

        let mut global_skill = make_test_manifest("git-expert", "Global git instructions");
        global_skill.source_path = Some(PathBuf::from("/home/user/.ride/skills/git-expert/SKILL.md"));

        let mut workspace_skill = make_test_manifest("git-expert", "Workspace specialized git instructions");
        workspace_skill.source_path = Some(PathBuf::from("/project/.ride/skills/git-expert/SKILL.md"));

        registry.register(global_skill, SkillScope::Global);
        assert_eq!(registry.get("git-expert").unwrap().instructions, "Global git instructions");
        assert!(!registry.is_overridden("git-expert"));

        // Register workspace skill with same name
        registry.register(workspace_skill, SkillScope::Workspace);

        // Workspace skill must override global
        assert_eq!(registry.count(), 1);
        assert_eq!(
            registry.get("git-expert").unwrap().instructions,
            "Workspace specialized git instructions"
        );
        assert!(registry.is_overridden("git-expert"));
        assert_eq!(registry.collisions().len(), 1);
        assert_eq!(registry.collisions()[0].skill_name, "git-expert");

        // Toggle enable
        assert!(registry.enable_skill("git-expert", false));
        assert!(!registry.get("git-expert").unwrap().metadata.enabled);
    }

    #[test]
    fn test_scan_dir_structure() {
        let tmp = std::env::temp_dir().join("ride_test_dual_skills");
        let global_dir = tmp.join("global_skills");
        let ws_dir = tmp.join("ws_skills");

        let skill_a_dir = global_dir.join("skill-a");
        let skill_b_global = global_dir.join("shared-skill");
        let skill_b_ws = ws_dir.join(".ride").join("skills").join("shared-skill");

        let _ = std::fs::create_dir_all(&skill_a_dir);
        let _ = std::fs::create_dir_all(&skill_b_global);
        let _ = std::fs::create_dir_all(&skill_b_ws);

        std::fs::write(
            skill_a_dir.join("SKILL.md"),
            "---\nname: skill-a\ndescription: A\n---\nBody A",
        )
        .unwrap();

        std::fs::write(
            skill_b_global.join("SKILL.md"),
            "---\nname: shared-skill\ndescription: Shared Global\n---\nGlobal instructions",
        )
        .unwrap();

        std::fs::write(
            skill_b_ws.join("SKILL.md"),
            "---\nname: shared-skill\ndescription: Shared Workspace\n---\nWorkspace instructions",
        )
        .unwrap();

        let mut registry = SkillRegistry::new();
        let summary = registry.scan_dual(&global_dir, Some(&ws_dir)).unwrap();

        assert_eq!(summary.global_count, 2);
        assert_eq!(summary.workspace_count, 1);
        assert_eq!(summary.collisions.len(), 1);

        assert_eq!(
            registry.get("shared-skill").unwrap().instructions,
            "Workspace instructions"
        );
        assert_eq!(registry.get("skill-a").unwrap().instructions, "Body A");

        let _ = std::fs::remove_dir_all(tmp);
    }
}
