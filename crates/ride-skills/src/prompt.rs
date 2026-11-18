use crate::budgeter::{BudgetedSkills, SkillContextBudgeter};
use crate::dynamic_tools::DynamicToolRegistry;
use crate::manifest::SkillManifest;
use serde::{Deserialize, Serialize};

pub const DEFAULT_RIDE_BASE_PROMPT: &str = r#"You are R.I.D.E (Rust Intelligent Development Environment), an autonomous, local-model AI coding assistant running directly on the user's workstation.
You operate completely offline with zero telemetry and full privacy.

Core Guidelines:
1. When asked to inspect, edit, or create code, call the appropriate tools.
2. Read files or inspect directories before editing files to ensure exact line matching and context understanding.
3. Be precise with hunk replacements in `edit_file`: ensure the target string uniquely and accurately matches existing lines.
4. For terminal commands, keep commands concise and targeted.
"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptSynthesisResult {
    /// Complete synthesized system prompt text for local LLMs
    pub system_prompt: String,
    /// Detailed breakdown of skill budgeting (activated, summarized, omitted)
    pub budgeted_skills: BudgetedSkills,
    /// OpenAI-compatible tools definition list (built-in + dynamic tools)
    pub openai_tools: serde_json::Value,
}

pub struct SystemPromptBuilder {
    base_instructions: String,
    budgeter: SkillContextBudgeter,
    include_built_in_tools: bool,
    custom_tool_registry: DynamicToolRegistry,
}

impl Default for SystemPromptBuilder {
    fn default() -> Self {
        Self {
            base_instructions: DEFAULT_RIDE_BASE_PROMPT.to_string(),
            budgeter: SkillContextBudgeter::default(),
            include_built_in_tools: true,
            custom_tool_registry: DynamicToolRegistry::new(),
        }
    }
}

impl SystemPromptBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_base_instructions(mut self, instructions: impl Into<String>) -> Self {
        self.base_instructions = instructions.into();
        self
    }

    pub fn with_budgeter(mut self, budgeter: SkillContextBudgeter) -> Self {
        self.budgeter = budgeter;
        self
    }

    pub fn with_built_in_tools(mut self, include: bool) -> Self {
        self.include_built_in_tools = include;
        self
    }

    pub fn with_dynamic_tools(mut self, registry: DynamicToolRegistry) -> Self {
        self.custom_tool_registry = registry;
        self
    }

    /// Synthesizes the full system prompt:
    /// - Base R.I.D.E instructions
    /// - Tool definitions and invocation schema (built-in + dynamic)
    /// - Capability summary of available inactive skills
    /// - Full instructions of activated skills (budgeted)
    pub fn build(
        &self,
        skills: &[&SkillManifest],
        user_prompt: &str,
        model_name: Option<&str>,
    ) -> PromptSynthesisResult {
        let budgeted = self.budgeter.budget_skills(skills, user_prompt, model_name);

        let mut prompt = String::new();

        // 1. Base Instructions
        prompt.push_str(&self.base_instructions);
        if !prompt.ends_with('\n') {
            prompt.push('\n');
        }

        // 2. Built-in and Custom Tools Schema
        if self.include_built_in_tools || !self.custom_tool_registry.is_empty() {
            prompt.push_str("\n## Available Tools\n");
            prompt.push_str("You can invoke tools using XML tags:\n");
            prompt.push_str("```xml\n<tool_call name=\"tool_name\">\n  <arg_name>value</arg_name>\n</tool_call>\n```\n");
            prompt.push_str("Or using JSON markdown block:\n");
            prompt.push_str("```json\n{\n  \"name\": \"tool_name\",\n  \"arguments\": { \"arg_name\": \"value\" }\n}\n```\n\n");

            if self.include_built_in_tools {
                prompt.push_str("### Built-in Core Tools\n");
                prompt.push_str("- `read_file(path, start_line, end_line)`: Read lines from a file.\n");
                prompt.push_str("- `write_file(path, content)`: Overwrite or create a file.\n");
                prompt.push_str("- `edit_file(path, target, replacement)`: Replace an exact substring in a file.\n");
                prompt.push_str("- `list_dir(path)`: List directory entries.\n");
                prompt.push_str("- `search_code(path, query, case_sensitive)`: Fast recursive search for text/code.\n");
                prompt.push_str("- `run_command(cmd)`: Execute shell command on host machine.\n\n");
            }

            if !self.custom_tool_registry.is_empty() {
                prompt.push_str("### Dynamic Skill Tools\n");
                for tool in self.custom_tool_registry.list() {
                    let param_str = serde_json::to_string(&tool.definition.parameters).unwrap_or_default();
                    prompt.push_str(&format!(
                        "- `{}`: {}\n  Parameters schema: {}\n",
                        tool.definition.name, tool.definition.description, param_str
                    ));
                }
                prompt.push('\n');
            }
        }

        // 3. Capability Summary of Available Skills (inactive / not fully activated)
        if !budgeted.summarized.is_empty() {
            prompt.push_str("## Available Registered Skills (Capabilities Overview)\n");
            prompt.push_str("The following specialized skills are available when triggered by relevant tasks:\n");
            for s in &budgeted.summarized {
                prompt.push_str(&format!("- **{}** (v{}): {}\n", s.name, s.version, s.description));
            }
            prompt.push('\n');
        }

        // 4. Full Instructions of Activated Skills
        if !budgeted.activated.is_empty() {
            prompt.push_str("## Active Agent Skills\n");
            for skill in &budgeted.activated {
                prompt.push_str(&format!(
                    "### Skill: {} (v{})\n{}\n\n",
                    skill.metadata.name, skill.metadata.version, skill.instructions
                ));
            }
        }

        // 5. Build OpenAI-compatible tools JSON
        let openai_tools = self.build_openai_tools_json();

        PromptSynthesisResult {
            system_prompt: prompt.trim().to_string(),
            budgeted_skills: budgeted,
            openai_tools,
        }
    }

    fn build_openai_tools_json(&self) -> serde_json::Value {
        let mut tools_vec = Vec::new();

        if self.include_built_in_tools {
            tools_vec.push(serde_json::json!({
                "type": "function",
                "function": {
                    "name": "read_file",
                    "description": "Read contents of a file with optional line bounds",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string", "description": "Path to file" },
                            "start_line": { "type": "integer", "description": "1-based starting line number" },
                            "end_line": { "type": "integer", "description": "1-based ending line number" }
                        },
                        "required": ["path"]
                    }
                }
            }));

            tools_vec.push(serde_json::json!({
                "type": "function",
                "function": {
                    "name": "write_file",
                    "description": "Write or overwrite file contents",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string", "description": "Path to file" },
                            "content": { "type": "string", "description": "Complete file content" }
                        },
                        "required": ["path", "content"]
                    }
                }
            }));

            tools_vec.push(serde_json::json!({
                "type": "function",
                "function": {
                    "name": "edit_file",
                    "description": "Replace exact target string with replacement in a file",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string", "description": "Path to file" },
                            "target": { "type": "string", "description": "Exact text to replace" },
                            "replacement": { "type": "string", "description": "Replacement text" }
                        },
                        "required": ["path", "target", "replacement"]
                    }
                }
            }));

            tools_vec.push(serde_json::json!({
                "type": "function",
                "function": {
                    "name": "list_dir",
                    "description": "List files and directories in a given path",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string", "description": "Directory path (defaults to current dir)" }
                        }
                    }
                }
            }));

            tools_vec.push(serde_json::json!({
                "type": "function",
                "function": {
                    "name": "search_code",
                    "description": "Recursively search directory for matching string",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string", "description": "Directory path to search" },
                            "query": { "type": "string", "description": "Query string to search for" },
                            "case_sensitive": { "type": "boolean", "description": "Whether search is case-sensitive" }
                        },
                        "required": ["query"]
                    }
                }
            }));

            tools_vec.push(serde_json::json!({
                "type": "function",
                "function": {
                    "name": "run_command",
                    "description": "Run shell command on the host machine",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "cmd": { "type": "string", "description": "Shell command line to execute" }
                        },
                        "required": ["cmd"]
                    }
                }
            }));
        }

        for tool in self.custom_tool_registry.list() {
            tools_vec.push(serde_json::json!({
                "type": "function",
                "function": {
                    "name": tool.definition.name,
                    "description": tool.definition.description,
                    "parameters": tool.definition.parameters,
                }
            }));
        }

        serde_json::Value::Array(tools_vec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dynamic_tools::{DynamicSkillTool, DynamicToolDefinition};
    use crate::manifest::SkillMetadata;
    use std::path::PathBuf;

    fn make_skill(name: &str, triggers: Vec<&str>, pinned: bool) -> SkillManifest {
        SkillManifest {
            metadata: SkillMetadata {
                name: name.into(),
                description: format!("Description of {name}"),
                version: "1.0.0".into(),
                author: None,
                triggers: triggers.into_iter().map(String::from).collect(),
                max_tokens: 500,
                pinned,
                compatible_models: vec![],
                enabled: true,
                tools: vec![],
            },
            instructions: format!("Detailed instructions for {name}"),
            source_path: None,
            base_dir: None,
            tools: vec![],
        }
    }

    #[test]
    fn test_prompt_synthesis_with_active_and_summarized_skills() {
        let active_skill = make_skill("rust-guru", vec!["rust", "cargo"], true);
        let inactive_skill = make_skill("python-pro", vec!["python"], false);

        let skills = vec![&active_skill, &inactive_skill];
        let builder = SystemPromptBuilder::new();

        let res = builder.build(&skills, "How do I optimize Rust code?", None);

        // System prompt contains base
        assert!(res.system_prompt.contains("R.I.D.E"));
        // Active skill instructions
        assert!(res.system_prompt.contains("Detailed instructions for rust-guru"));
        // Inactive skill summarized
        assert!(res.system_prompt.contains("**python-pro** (v1.0.0): Description of python-pro"));
        // Built in tools included
        assert!(res.system_prompt.contains("read_file"));
        assert!(res.system_prompt.contains("run_command"));

        // OpenAI tools schema contains functions
        assert!(res.openai_tools.is_array());
        assert!(res.openai_tools.as_array().unwrap().len() >= 6);
    }

    #[test]
    fn test_prompt_synthesis_with_dynamic_tools() {
        let mut custom_reg = DynamicToolRegistry::new();
        let def = DynamicToolDefinition {
            name: "benchmark".into(),
            description: "Run criterion benchmarks".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "bench_name": { "type": "string" }
                }
            }),
            script: "bench.ps1".into(),
            timeout_ms: 10000,
        };
        custom_reg.register(DynamicSkillTool::new(def, PathBuf::from(".")));

        let builder = SystemPromptBuilder::new().with_dynamic_tools(custom_reg);
        let res = builder.build(&[], "test prompt", None);

        assert!(res.system_prompt.contains("benchmark"));
        assert!(res.system_prompt.contains("Run criterion benchmarks"));

        let arr = res.openai_tools.as_array().unwrap();
        assert!(arr.iter().any(|t| t["function"]["name"] == "benchmark"));
    }
}
