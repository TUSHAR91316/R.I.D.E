use crate::manifest::SkillManifest;
use regex::RegexBuilder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillSummary {
    pub name: String,
    pub description: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetedSkills {
    /// Skills fully activated with complete instructions included
    pub activated: Vec<SkillManifest>,
    /// Inactive or budget-limited skills summarized as 1-line capability descriptions
    pub summarized: Vec<SkillSummary>,
    /// Skills omitted entirely due to hard budget overflow or model incompatibility
    pub omitted: Vec<String>,
    /// Estimated total tokens consumed by activated skills + summaries
    pub estimated_tokens: usize,
    /// Maximum token budget
    pub max_budget_tokens: usize,
}

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

    pub fn max_budget_tokens(&self) -> usize {
        self.max_total_tokens
    }

    /// Select skills relevant to user input based on triggers and enabled state,
    /// maintaining backwards compatibility for simple selection.
    pub fn select_skills<'a>(
        &self,
        available_skills: &'a [&'a SkillManifest],
        user_prompt: &str,
    ) -> Vec<&'a SkillManifest> {
        let budgeted = self.budget_skills(available_skills, user_prompt, None);
        let active_names: Vec<String> = budgeted.activated.into_iter().map(|s| s.metadata.name).collect();

        available_skills
            .iter()
            .copied()
            .filter(|s| active_names.contains(&s.metadata.name))
            .collect()
    }

    /// Comprehensive dynamic skill budgeter:
    /// - Filters for model compatibility and enabled state
    /// - Matches triggers (keywords and regex)
    /// - Prioritizes: Pinned + Triggered > Pinned > Triggered > Inactive
    /// - Enforces token budget: full instructions for active skills, 1-line capability summary
    ///   for inactive skills, gracefully omitting overflow.
    pub fn budget_skills(
        &self,
        available_skills: &[&SkillManifest],
        user_prompt: &str,
        model_name: Option<&str>,
    ) -> BudgetedSkills {
        let mut activated = Vec::new();
        let mut summarized = Vec::new();
        let mut omitted = Vec::new();
        let mut tokens_used = 0;

        // 1. Filter enabled and model-compatible skills
        let mut candidates = Vec::new();
        for skill in available_skills {
            if !skill.metadata.enabled {
                continue;
            }

            if let Some(target_model) = model_name {
                if !skill.metadata.compatible_models.is_empty() {
                    let compatible = skill.metadata.compatible_models.iter().any(|cm| {
                        let cm_lower = cm.to_lowercase();
                        let target_lower = target_model.to_lowercase();
                        target_lower.contains(&cm_lower) || cm_lower.contains(&target_lower)
                    });
                    if !compatible {
                        omitted.push(format!(
                            "{} (incompatible with model '{}')",
                            skill.metadata.name, target_model
                        ));
                        continue;
                    }
                }
            }

            let triggered = Self::matches_triggers(skill, user_prompt);
            let pinned = skill.metadata.pinned;

            // Priority:
            // 0: Pinned && Triggered
            // 1: Pinned
            // 2: Triggered
            // 3: Inactive / Candidate for summary
            let priority = match (pinned, triggered) {
                (true, true) => 0,
                (true, false) => 1,
                (false, true) => 2,
                (false, false) => 3,
            };

            candidates.push((priority, *skill));
        }

        // Sort candidates deterministically:
        // Priority ascending (0 first, then 1, 2, 3), then skill name alphabetically
        candidates.sort_by(|(p_a, a), (p_b, b)| {
            p_a.cmp(p_b).then_with(|| a.metadata.name.cmp(&b.metadata.name))
        });

        // 2. Allocate tokens for full activation (priorities 0, 1, 2)
        let mut summary_candidates = Vec::new();

        for (priority, skill) in candidates {
            if priority < 3 {
                // Candidate for full activation
                let est = Self::estimate_skill_tokens(skill);
                if tokens_used + est <= self.max_total_tokens {
                    tokens_used += est;
                    activated.push((*skill).clone());
                } else {
                    // Exceeded budget for full instructions: demote to summary candidate
                    summary_candidates.push(skill);
                }
            } else {
                // Priority 3: Inactive candidate for summary
                summary_candidates.push(skill);
            }
        }

        // 3. Allocate tokens for 1-line capability summaries
        for skill in summary_candidates {
            let summary = SkillSummary {
                name: skill.metadata.name.clone(),
                description: if skill.metadata.description.is_empty() {
                    format!("Skill {}", skill.metadata.name)
                } else {
                    skill.metadata.description.clone()
                },
                version: skill.metadata.version.clone(),
            };

            let summary_str = format!("- {} (v{}): {}", summary.name, summary.version, summary.description);
            let summary_tokens = (summary_str.chars().count() / 4) + 4;

            if tokens_used + summary_tokens <= self.max_total_tokens {
                tokens_used += summary_tokens;
                summarized.push(summary);
            } else {
                omitted.push(format!("{} (token budget exhausted)", skill.metadata.name));
            }
        }

        BudgetedSkills {
            activated,
            summarized,
            omitted,
            estimated_tokens: tokens_used,
            max_budget_tokens: self.max_total_tokens,
        }
    }

    /// Check if user prompt triggers skill via keywords or regex
    pub fn matches_triggers(skill: &SkillManifest, user_prompt: &str) -> bool {
        let prompt_trimmed = user_prompt.trim();
        if prompt_trimmed.is_empty() {
            return false;
        }

        let prompt_lower = prompt_trimmed.to_lowercase();

        if skill.metadata.triggers.is_empty() {
            let name_clean = skill.metadata.name.to_lowercase();
            let name_spaced = name_clean.replace('-', " ");
            return contains_keyword(&prompt_lower, &name_clean)
                || contains_keyword(&prompt_lower, &name_spaced);
        }

        for trigger in &skill.metadata.triggers {
            let t_trimmed = trigger.trim();
            if t_trimmed.is_empty() {
                continue;
            }

            // Check if trigger is surrounded by slashes: e.g. /regex/
            // or contains explicit regex anchors like \b, ^, $
            let is_slash_regex = t_trimmed.starts_with('/') && t_trimmed.ends_with('/') && t_trimmed.len() > 2;
            let is_explicit_regex = is_slash_regex
                || t_trimmed.contains(r"\b")
                || t_trimmed.starts_with('^')
                || t_trimmed.ends_with('$');

            if is_explicit_regex {
                let pat = if is_slash_regex {
                    &t_trimmed[1..t_trimmed.len() - 1]
                } else {
                    t_trimmed
                };

                if let Ok(re) = RegexBuilder::new(pat)
                    .case_insensitive(true)
                    .size_limit(100_000)
                    .build()
                {
                    if re.is_match(user_prompt) {
                        return true;
                    }
                    continue;
                }
            }

            // Keyword match with word boundary check
            if contains_keyword(&prompt_lower, t_trimmed) {
                return true;
            }
        }

        false
    }

    /// Estimate context tokens consumed by a skill's full instructions
    pub fn estimate_skill_tokens(skill: &SkillManifest) -> usize {
        let chars = skill.instructions.chars().count();
        let token_est = (chars / 4) + 20;
        token_est.min(skill.metadata.max_tokens).max(1)
    }
}

/// Case-insensitive keyword matching respecting word boundaries.
/// Ensures keyword triggers like "git" do not spuriously match inside "digital",
/// while matching standalone occurrences like "git", "git status", "(git)", etc.
pub fn contains_keyword(text: &str, keyword: &str) -> bool {
    let kw = keyword.trim();
    if kw.is_empty() {
        return false;
    }

    let kw_lower = kw.to_lowercase();
    let text_lower = text.to_lowercase();

    let first_char = kw_lower.chars().next().unwrap();
    let last_char = kw_lower.chars().next_back().unwrap();
    let kw_starts_word = first_char.is_alphanumeric() || first_char == '_';
    let kw_ends_word = last_char.is_alphanumeric() || last_char == '_';

    let mut start_idx = 0;
    while let Some(idx) = text_lower[start_idx..].find(&kw_lower) {
        let abs_idx = start_idx + idx;
        let end_idx = abs_idx + kw_lower.len();

        let left_ok = if kw_starts_word && abs_idx > 0 {
            let prev_char = text_lower[..abs_idx].chars().next_back().unwrap();
            !prev_char.is_alphanumeric() && prev_char != '_'
        } else {
            true
        };

        let right_ok = if kw_ends_word && end_idx < text_lower.len() {
            let next_char = text_lower[end_idx..].chars().next().unwrap();
            !next_char.is_alphanumeric() && next_char != '_'
        } else {
            true
        };

        if left_ok && right_ok {
            return true;
        }

        start_idx = abs_idx + 1;
        if start_idx >= text_lower.len() {
            break;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{SkillManifest, SkillMetadata};

    fn make_skill(name: &str, triggers: Vec<&str>, pinned: bool, tokens: usize) -> SkillManifest {
        SkillManifest {
            metadata: SkillMetadata {
                name: name.into(),
                description: format!("Description of {name}"),
                version: "0.1.0".into(),
                author: None,
                triggers: triggers.into_iter().map(String::from).collect(),
                max_tokens: tokens,
                pinned,
                compatible_models: vec![],
                enabled: true,
                tools: vec![],
            },
            instructions: format!("Instructions for {name} with enough words to count"),
            source_path: None,
            base_dir: None,
            tools: vec![],
        }
    }

    #[test]
    fn test_budgeter_selection() {
        let skill1 = make_skill("rust", vec!["cargo", "rust"], false, 500);
        let skill2 = make_skill("python", vec!["python"], false, 500);

        let list = vec![&skill1, &skill2];
        let budgeter = SkillContextBudgeter::default();

        let matches = budgeter.select_skills(&list, "Please build this with cargo");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].metadata.name, "rust");

        // Skill without triggers matches only if prompt mentions name
        let skill_no_triggers = make_skill("docker-helper", vec![], false, 500);
        let list_with_untriggered = vec![&skill1, &skill_no_triggers];
        let matches2 = budgeter.select_skills(&list_with_untriggered, "What is the capital of France?");
        assert_eq!(matches2.len(), 0);

        let matches3 = budgeter.select_skills(&list_with_untriggered, "Use docker-helper for this container");
        assert_eq!(matches3.len(), 1);
        assert_eq!(matches3[0].metadata.name, "docker-helper");
    }

    #[test]
    fn test_pinned_skills_prioritized() {
        let mut skill_pinned = make_skill("base-linter", vec!["lint"], true, 100);
        skill_pinned.instructions = "Always follow strict linting rules".into();

        let mut skill_triggered = make_skill("react-expert", vec!["react", "frontend"], false, 100);
        skill_triggered.instructions = "React component best practices".into();

        let list = vec![&skill_pinned, &skill_triggered];

        // Prompt triggers react-expert, but base-linter is pinned so both should activate
        let budgeter = SkillContextBudgeter::new(500);
        let budgeted = budgeter.budget_skills(&list, "Build a react component", None);

        assert_eq!(budgeted.activated.len(), 2);
        // Pinned is index 0 or prioritized
        assert!(budgeted.activated.iter().any(|s| s.metadata.name == "base-linter"));
        assert!(budgeted.activated.iter().any(|s| s.metadata.name == "react-expert"));
    }

    #[test]
    fn test_budget_exhaustion_summarizes_skill() {
        let mut big_skill = make_skill("heavy-skill", vec!["heavy"], false, 1000);
        big_skill.instructions = "A".repeat(1200); // ~300 tokens

        let mut small_skill = make_skill("small-skill", vec!["small"], false, 100);
        small_skill.instructions = "Short instruction".into();

        let list = vec![&big_skill, &small_skill];

        // Set budget so big_skill does not fit full instructions (e.g. 50 tokens)
        let budgeter = SkillContextBudgeter::new(60);
        let budgeted = budgeter.budget_skills(&list, "heavy and small", None);

        // Big skill gets demoted to summary because its full instructions exceed budget
        assert!(budgeted.summarized.iter().any(|s| s.name == "heavy-skill"));
    }

    #[test]
    fn test_regex_trigger_matching() {
        let skill_regex = make_skill("regex-test", vec![r"\btest(ing|ed)?\b"], false, 500);
        let list = vec![&skill_regex];
        let budgeter = SkillContextBudgeter::default();

        let matches1 = budgeter.select_skills(&list, "We are testing this code");
        assert_eq!(matches1.len(), 1);

        let matches2 = budgeter.select_skills(&list, "No match here at all");
        assert_eq!(matches2.len(), 0);
    }

    #[test]
    fn test_model_compatibility_filter() {
        let mut skill_qwen = make_skill("qwen-only", vec!["code"], false, 500);
        skill_qwen.metadata.compatible_models = vec!["qwen2.5-coder".into()];

        let list = vec![&skill_qwen];
        let budgeter = SkillContextBudgeter::default();

        let budgeted_llama = budgeter.budget_skills(&list, "write code", Some("llama-3.1-8b"));
        assert_eq!(budgeted_llama.activated.len(), 0);
        assert_eq!(budgeted_llama.omitted.len(), 1);

        let budgeted_qwen = budgeter.budget_skills(&list, "write code", Some("qwen2.5-coder:7b"));
        assert_eq!(budgeted_qwen.activated.len(), 1);
    }

    #[test]
    fn test_word_boundary_matching_prevents_false_positives() {
        let skill_git = make_skill("git-assistant", vec!["git"], false, 500);
        let skill_go = make_skill("go-expert", vec!["go"], false, 500);
        let skill_cpp = make_skill("cpp-dev", vec!["c++"], false, 500);

        // "digital" contains "git" as substring, but word boundary matching must reject it
        assert!(!SkillContextBudgeter::matches_triggers(&skill_git, "This is a digital signal processing task."));

        // "cargo" contains "go" as substring, but word boundary matching must reject it
        assert!(!SkillContextBudgeter::matches_triggers(&skill_go, "Run cargo check please."));

        // Valid standalone occurrences must match
        assert!(SkillContextBudgeter::matches_triggers(&skill_git, "How do I use git commit?"));
        assert!(SkillContextBudgeter::matches_triggers(&skill_git, "Run (git) here."));
        assert!(SkillContextBudgeter::matches_triggers(&skill_go, "Write idiomatic go code."));
        assert!(SkillContextBudgeter::matches_triggers(&skill_cpp, "Compile with c++20 please."));
    }

    #[test]
    fn test_name_with_hyphen_matching() {
        let skill_code_review = make_skill("code-review", vec![], false, 500);
        // Prompt says "code review" with space instead of hyphen
        assert!(SkillContextBudgeter::matches_triggers(&skill_code_review, "Can you do a code review on this file?"));
        assert!(SkillContextBudgeter::matches_triggers(&skill_code_review, "Run code-review"));
    }

    #[test]
    fn test_deterministic_budget_ordering() {
        let s_c = make_skill("charlie", vec!["common"], true, 100);
        let s_a = make_skill("alpha", vec!["common"], true, 100);
        let s_b = make_skill("bravo", vec!["common"], true, 100);

        // Put in reverse alphabetical order
        let list = vec![&s_c, &s_a, &s_b];
        let budgeter = SkillContextBudgeter::new(1000);
        let res = budgeter.budget_skills(&list, "common task", None);

        // Must be sorted alpha, bravo, charlie
        assert_eq!(res.activated.len(), 3);
        assert_eq!(res.activated[0].metadata.name, "alpha");
        assert_eq!(res.activated[1].metadata.name, "bravo");
        assert_eq!(res.activated[2].metadata.name, "charlie");
    }
}
