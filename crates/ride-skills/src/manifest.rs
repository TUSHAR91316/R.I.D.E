use ride_core::error::{Result, RideError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub triggers: Vec<String>,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_max_tokens() -> usize {
    1024
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillManifest {
    pub metadata: SkillMetadata,
    pub instructions: String,
    pub source_path: Option<String>,
}

impl SkillManifest {
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

        Ok(Self {
            metadata,
            instructions: body_lines.join("\n").trim().to_string(),
            source_path: None,
        })
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
triggers:
  - refactor
  - clean
max_tokens: 500
enabled: true
---
# Test Skill Instructions
Please write clean code and test edge cases.
"#;
        let manifest = SkillManifest::parse_skill_markdown(content).unwrap();
        assert_eq!(manifest.metadata.name, "test-skill");
        assert_eq!(manifest.metadata.triggers, vec!["refactor", "clean"]);
        assert_eq!(manifest.metadata.max_tokens, 500);
        assert!(manifest.instructions.contains("clean code"));
    }

    #[test]
    fn test_parse_invalid_frontmatter() {
        let content = "Just markdown without frontmatter";
        assert!(SkillManifest::parse_skill_markdown(content).is_err());
    }

    #[test]
    fn test_parse_crlf_and_bom_skill_md() {
        let content = "\u{feff}---\r\nname: bom-skill\r\ndescription: Skill with BOM and CRLF\r\ntriggers:\r\n  - bom\r\n--- \r\n# Body\r\nHello Windows!";
        let manifest = SkillManifest::parse_skill_markdown(content).unwrap();
        assert_eq!(manifest.metadata.name, "bom-skill");
        assert_eq!(manifest.metadata.triggers, vec!["bom"]);
        assert_eq!(manifest.instructions, "# Body\nHello Windows!");
    }
}
