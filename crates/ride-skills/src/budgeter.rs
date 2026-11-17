use crate::manifest::SkillManifest;

pub struct SkillContextBudgeter {
    max_total_tokens: usize,
}

impl Default for SkillContextBudgeter {
    fn default() -> Self {
        Self {
            max_total_tokens: 2048,
        }
    }
}

impl SkillContextBudgeter {
    pub fn new(max_total_tokens: usize) -> Self {
        Self { max_total_tokens }
    }

    /// Select skills relevant to user input based on triggers and enabled state,
    /// while staying within max_total_tokens budget (approx 4 chars/token).
    pub fn select_skills<'a>(
        &self,
        available_skills: &'a [&'a SkillManifest],
        user_prompt: &str,
    ) -> Vec<&'a SkillManifest> {
        let prompt_lower = user_prompt.to_lowercase();
        let mut selected = Vec::new();
        let mut token_estimate = 0;

        for skill in available_skills {
            if !skill.metadata.enabled {
                continue;
            }

            let triggered = if skill.metadata.triggers.is_empty() {
                prompt_lower.contains(&skill.metadata.name.to_lowercase())
            } else {
                skill.metadata.triggers.iter().any(|t| {
                    let t_trimmed = t.trim().to_lowercase();
                    !t_trimmed.is_empty() && prompt_lower.contains(&t_trimmed)
                })
            };

            if triggered {
                let estimated_tokens = ((skill.instructions.len() / 4) + 50).min(skill.metadata.max_tokens);
                if token_estimate + estimated_tokens <= self.max_total_tokens {
                    token_estimate += estimated_tokens;
                    selected.push(*skill);
                }
            }
        }

        selected
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{SkillManifest, SkillMetadata};

    #[test]
    fn test_budgeter_selection() {
        let skill1 = SkillManifest {
            metadata: SkillMetadata {
                name: "rust".into(),
                description: "Rust".into(),
                triggers: vec!["cargo".into(), "rust".into()],
                max_tokens: 500,
                enabled: true,
            },
            instructions: "Rust instructions".into(),
            source_path: None,
        };

        let skill2 = SkillManifest {
            metadata: SkillMetadata {
                name: "python".into(),
                description: "Python".into(),
                triggers: vec!["python".into()],
                max_tokens: 500,
                enabled: true,
            },
            instructions: "Python instructions".into(),
            source_path: None,
        };

        let list = vec![&skill1, &skill2];
        let budgeter = SkillContextBudgeter::default();

        let matches = budgeter.select_skills(&list, "Please build this with cargo");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].metadata.name, "rust");

        // Skill without triggers matches only if prompt mentions name
        let skill_no_triggers = SkillManifest {
            metadata: SkillMetadata {
                name: "docker-helper".into(),
                description: "Docker helper".into(),
                triggers: vec![],
                max_tokens: 500,
                enabled: true,
            },
            instructions: "Docker commands".into(),
            source_path: None,
        };
        let list_with_untriggered = vec![&skill1, &skill_no_triggers];
        let matches2 = budgeter.select_skills(&list_with_untriggered, "What is the capital of France?");
        assert_eq!(matches2.len(), 0);

        let matches3 = budgeter.select_skills(&list_with_untriggered, "Use docker-helper for this container");
        assert_eq!(matches3.len(), 1);
        assert_eq!(matches3[0].metadata.name, "docker-helper");
    }
}
