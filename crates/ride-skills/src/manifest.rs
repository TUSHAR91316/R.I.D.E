use crate::dynamic_tools::DynamicToolDefinition;
use ride_core::error::{Result, RideError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillMetadata {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub triggers: Vec<String>,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub compatible_models: Vec<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub tools: Vec<DynamicToolDefinition>,
}

fn default_version() -> String {
    "0.1.0".to_string()
}

fn default_max_tokens() -> usize {
    1024
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillManifest {
    pub metadata: SkillMetadata,
    pub instructions: String,
    pub source_path: Option<PathBuf>,
    pub base_dir: Option<PathBuf>,
    #[serde(default)]
    pub tools: Vec<DynamicToolDefinition>,
}

impl SkillManifest {
    /// Parse SKILL.md markdown text with YAML frontmatter.
    /// Handles optional UTF-8 BOM, CRLF/LF line endings, and missing optional fields.
    pub fn parse_skill_markdown(content: &str) -> Result<Self> {
        let content = content.strip_prefix("\u{feff}").unwrap_or(content);
        let trimmed = content.trim_start();
        if !trimmed.starts_with("---") {
            return Err(RideError::Skill(
                "SKILL.md must start with YAML frontmatter delimiters '---'".into(),
            ));
        }

        let mut lines = trimmed.lines();
        let first_line = lines.next().unwrap_or_default();
        if first_line.trim() != "---" {
            return Err(RideError::Skill(
                "SKILL.md first line must be frontmatter delimiter '---'".into(),
            ));
        }

        let mut yaml_lines = Vec::new();
        let mut found_closing = false;
        let mut body_lines = Vec::new();

        for line in lines {
            if !found_closing {
                if line.trim() == "---" {
                    found_closing = true;
                } else {
                    yaml_lines.push(line);
                }
            } else {
                body_lines.push(line);
            }
        }

        if !found_closing {
            return Err(RideError::Skill(
                "SKILL.md missing closing '---' delimiter for YAML frontmatter".into(),
            ));
        }

        let yaml_str = yaml_lines.join("\n");
        let metadata: SkillMetadata = serde_yaml::from_str(&yaml_str)
            .map_err(|e| RideError::Skill(format!("Failed to parse YAML frontmatter: {e}")))?;

        if metadata.name.trim().is_empty() {
            return Err(RideError::Skill(
                "SKILL.md manifest missing valid 'name' field".into(),
            ));
        }

        let frontmatter_tools = metadata.tools.clone();

        Ok(Self {
            metadata,
            instructions: body_lines.join("\n").trim().to_string(),
            source_path: None,
            base_dir: None,
            tools: frontmatter_tools,
        })
    }

    /// Load skill manifest from a directory containing SKILL.md and optional tools.json
    pub fn load_from_directory(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref();
        if !dir.exists() || !dir.is_dir() {
            return Err(RideError::Skill(format!(
                "Directory does not exist: {}",
                dir.display()
            )));
        }

        let skill_md = dir.join("SKILL.md");
        let alt_skill_md = dir.join("skill.md");
        let target_path = if skill_md.exists() && skill_md.is_file() {
            skill_md
        } else if alt_skill_md.exists() && alt_skill_md.is_file() {
            alt_skill_md
        } else {
            return Err(RideError::Skill(format!(
                "No SKILL.md found in {}",
                dir.display()
            )));
        };

        let content = std::fs::read_to_string(&target_path).map_err(|e| {
            RideError::Skill(format!("Failed to read {}: {e}", target_path.display()))
        })?;

        let mut manifest = Self::parse_skill_markdown(&content)?;
        manifest.source_path = Some(target_path);
        manifest.base_dir = Some(dir.to_path_buf());

        // Check for optional tools.json
        let tools_json = dir.join("tools.json");
        if tools_json.exists() && tools_json.is_file() {
            match std::fs::read_to_string(&tools_json) {
                Ok(tools_content) => match Self::parse_tools_json(&tools_content) {
                    Ok(loaded_tools) => {
                        for tool in loaded_tools {
                            if !manifest.tools.iter().any(|t| t.name == tool.name) {
                                manifest.tools.push(tool);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to parse tools.json in {}: {e}",
                            dir.display()
                        );
                    }
                },
                Err(e) => {
                    tracing::warn!(
                        "Failed to read tools.json in {}: {e}",
                        dir.display()
                    );
                }
            }
        }

        // Keep metadata tools synchronized with manifest tools
        manifest.metadata.tools = manifest.tools.clone();

        Ok(manifest)
    }

    /// Parse tools from tools.json content (accepts either array of tools or object with "tools" key)
    pub fn parse_tools_json(content: &str) -> Result<Vec<DynamicToolDefinition>> {
        let trimmed = content.trim();
        if trimmed.starts_with('[') {
            serde_json::from_str(trimmed)
                .map_err(|e| RideError::Skill(format!("Failed to parse tools.json array: {e}")))
        } else if trimmed.starts_with('{') {
            #[derive(Deserialize)]
            struct Wrapper {
                tools: Vec<DynamicToolDefinition>,
            }
            let wrapper: Wrapper = serde_json::from_str(trimmed)
                .map_err(|e| RideError::Skill(format!("Failed to parse tools.json object: {e}")))?;
            Ok(wrapper.tools)
        } else {
            Err(RideError::Skill("Invalid tools.json format".into()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_skill_md() {
        let content = r#"---
name: test-skill
description: A helpful test skill
version: 1.2.0
author: DeepSeek
triggers:
  - refactor
  - clean
max_tokens: 500
pinned: true
compatible_models:
  - qwen2.5-coder
  - deepseek-coder
enabled: true
---
# Test Skill Instructions
Please write clean code and test edge cases.
"#;
        let manifest = SkillManifest::parse_skill_markdown(content).unwrap();
        assert_eq!(manifest.metadata.name, "test-skill");
        assert_eq!(manifest.metadata.description, "A helpful test skill");
        assert_eq!(manifest.metadata.version, "1.2.0");
        assert_eq!(manifest.metadata.author, Some("DeepSeek".into()));
        assert_eq!(manifest.metadata.triggers, vec!["refactor", "clean"]);
        assert_eq!(manifest.metadata.max_tokens, 500);
        assert!(manifest.metadata.pinned);
        assert_eq!(
            manifest.metadata.compatible_models,
            vec!["qwen2.5-coder", "deepseek-coder"]
        );
        assert!(manifest.instructions.contains("clean code"));
    }

    #[test]
    fn test_parse_minimal_skill_md() {
        let content = r#"---
name: minimal-skill
---
Instructions here.
"#;
        let manifest = SkillManifest::parse_skill_markdown(content).unwrap();
        assert_eq!(manifest.metadata.name, "minimal-skill");
        assert_eq!(manifest.metadata.version, "0.1.0");
        assert_eq!(manifest.metadata.max_tokens, 1024);
        assert!(!manifest.metadata.pinned);
        assert!(manifest.metadata.enabled);
        assert!(manifest.metadata.triggers.is_empty());
        assert_eq!(manifest.instructions, "Instructions here.");
    }

    #[test]
    fn test_parse_invalid_frontmatter() {
        let content = "Just markdown without frontmatter";
        assert!(SkillManifest::parse_skill_markdown(content).is_err());

        let missing_closing = "---\nname: foo\nNo closing delimiter";
        assert!(SkillManifest::parse_skill_markdown(missing_closing).is_err());

        let empty_name = "---\nname: ''\n---\nBody";
        assert!(SkillManifest::parse_skill_markdown(empty_name).is_err());
    }

    #[test]
    fn test_parse_crlf_and_bom_skill_md() {
        let content = "\u{feff}---\r\nname: bom-skill\r\ndescription: Skill with BOM and CRLF\r\ntriggers:\r\n  - bom\r\n--- \r\n# Body\r\nHello Windows!";
        let manifest = SkillManifest::parse_skill_markdown(content).unwrap();
        assert_eq!(manifest.metadata.name, "bom-skill");
        assert_eq!(manifest.metadata.triggers, vec!["bom"]);
        assert_eq!(manifest.instructions, "# Body\nHello Windows!");
    }

    #[test]
    fn test_tools_json_parsing_and_merging() {
        let json_array = r#"[
            {
                "name": "run_audit",
                "description": "Run security audit",
                "parameters": { "type": "object" },
                "script": "audit.sh"
            }
        ]"#;
        let tools = SkillManifest::parse_tools_json(json_array).unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "run_audit");

        let json_wrapped = r#"{
            "tools": [
                {
                    "name": "run_linter",
                    "description": "Run linter",
                    "parameters": { "type": "object" },
                    "script": "lint.py"
                }
            ]
        }"#;
        let tools2 = SkillManifest::parse_tools_json(json_wrapped).unwrap();
        assert_eq!(tools2.len(), 1);
        assert_eq!(tools2[0].name, "run_linter");
    }
}
