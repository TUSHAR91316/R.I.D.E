use crate::parser::ParsedToolCall;
use crate::tools::{ToolResult, Toolset};
use ride_skills::SkillManifest;

pub struct AgentExecutor;

impl AgentExecutor {
    /// Formats active skill instructions into a system prompt segment for local LLMs
    pub fn build_skills_prompt(skills: &[&SkillManifest]) -> String {
        if skills.is_empty() {
            return String::new();
        }

        let mut out = String::from("\n## Active Agent Skills\n");
        for skill in skills {
            out.push_str(&format!(
                "### Skill: {}\n{}\n\n",
                skill.metadata.name, skill.instructions
            ));
        }
        out
    }

    pub fn execute_tool(call: &ParsedToolCall) -> ToolResult {
        match call.name.as_str() {
            "read_file" => {
                let path = call.arguments["path"].as_str().unwrap_or_default();
                let start = call.arguments["start_line"].as_u64().map(|v| v as usize);
                let end = call.arguments["end_line"].as_u64().map(|v| v as usize);
                match Toolset::read_file(path, start, end) {
                    Ok(out) => ToolResult {
                        success: true,
                        output: out,
                    },
                    Err(e) => ToolResult {
                        success: false,
                        output: e.to_string(),
                    },
                }
            }
            "write_file" => {
                let path = call.arguments["path"].as_str().unwrap_or_default();
                let content = call.arguments["content"].as_str().unwrap_or_default();
                match Toolset::write_file(path, content) {
                    Ok(_) => ToolResult {
                        success: true,
                        output: format!("Successfully wrote {}", path),
                    },
                    Err(e) => ToolResult {
                        success: false,
                        output: e.to_string(),
                    },
                }
            }
            "edit_file" => {
                let path = call.arguments["path"].as_str().unwrap_or_default();
                let target = call.arguments["target"].as_str().unwrap_or_default();
                let replacement = call.arguments["replacement"].as_str().unwrap_or_default();
                match Toolset::edit_file(path, target, replacement) {
                    Ok(_) => ToolResult {
                        success: true,
                        output: format!("Successfully edited {}", path),
                    },
                    Err(e) => ToolResult {
                        success: false,
                        output: e.to_string(),
                    },
                }
            }
            "list_dir" => {
                let path = call.arguments["path"].as_str().unwrap_or(".");
                match Toolset::list_dir(path) {
                    Ok(entries) => {
                        let lines: Vec<String> = entries.into_iter().map(|p| p.to_string_lossy().to_string()).collect();
                        ToolResult {
                            success: true,
                            output: lines.join("\n"),
                        }
                    }
                    Err(e) => ToolResult {
                        success: false,
                        output: e.to_string(),
                    },
                }
            }
            "search_code" => {
                let dir = call.arguments["path"].as_str().or_else(|| call.arguments["dir"].as_str()).unwrap_or(".");
                let query = call.arguments["query"].as_str().unwrap_or_default();
                let case_sensitive = call.arguments["case_sensitive"].as_bool().unwrap_or(false);
                match Toolset::search_code(dir, query, case_sensitive) {
                    Ok(matches) => {
                        let lines: Vec<String> = matches
                            .into_iter()
                            .map(|m| format!("{}:{}: {}", m.file.display(), m.line_number, m.line_text))
                            .collect();
                        ToolResult {
                            success: true,
                            output: lines.join("\n"),
                        }
                    }
                    Err(e) => ToolResult {
                        success: false,
                        output: e.to_string(),
                    },
                }
            }
            "run_command" => {
                let cmd = call.arguments["cmd"].as_str().or_else(|| call.arguments["command"].as_str()).unwrap_or_default();
                match Toolset::run_command(cmd, None) {
                    Ok(out) => ToolResult {
                        success: out.exit_code == 0,
                        output: if !out.stderr.is_empty() {
                            format!("{}\n[stderr]: {}", out.stdout, out.stderr)
                        } else {
                            out.stdout
                        },
                    },
                    Err(e) => ToolResult {
                        success: false,
                        output: e.to_string(),
                    },
                }
            }
            other => ToolResult {
                success: false,
                output: format!("Unknown tool: {}", other),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_execute_list_dir() {
        let call = ParsedToolCall {
            name: "list_dir".into(),
            arguments: json!({ "path": "." }),
        };
        let res = AgentExecutor::execute_tool(&call);
        assert!(res.success);
        assert!(res.output.contains("Cargo.toml"));
    }

    #[test]
    fn test_execute_unknown_tool() {
        let call = ParsedToolCall {
            name: "nonexistent".into(),
            arguments: json!({}),
        };
        let res = AgentExecutor::execute_tool(&call);
        assert!(!res.success);
        assert!(res.output.contains("Unknown tool"));
    }
}
