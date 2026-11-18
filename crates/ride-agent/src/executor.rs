use crate::parser::ParsedToolCall;
use crate::tools::{ToolResult, Toolset};
use ride_skills::{DynamicToolRegistry, PromptSynthesisResult, SkillManifest, SystemPromptBuilder};
use std::path::Path;

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

    /// Synthesizes complete system prompt including base instructions, dynamic tools,
    /// capability summaries, and activated skill instructions.
    pub fn build_system_prompt(
        skills: &[&SkillManifest],
        dynamic_tools: Option<&DynamicToolRegistry>,
        user_prompt: &str,
        model_name: Option<&str>,
    ) -> PromptSynthesisResult {
        let mut builder = SystemPromptBuilder::new();
        if let Some(dyn_tools) = dynamic_tools {
            builder = builder.with_dynamic_tools(dyn_tools.clone());
        }
        builder.build(skills, user_prompt, model_name)
    }

    /// Executes tool call against built-in tools
    pub fn execute_tool(call: &ParsedToolCall) -> ToolResult {
        Self::execute_tool_with_dynamic(call, None, None)
    }

    /// Executes tool call against built-in tools or registered dynamic skill tools
    pub fn execute_tool_with_dynamic(
        call: &ParsedToolCall,
        dynamic_tools: Option<&DynamicToolRegistry>,
        working_dir: Option<&Path>,
    ) -> ToolResult {
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
                        let lines: Vec<String> = entries
                            .into_iter()
                            .map(|p| p.to_string_lossy().to_string())
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
            "search_code" => {
                let dir = call.arguments["path"]
                    .as_str()
                    .or_else(|| call.arguments["dir"].as_str())
                    .unwrap_or(".");
                let query = call.arguments["query"].as_str().unwrap_or_default();
                let case_sensitive = call.arguments["case_sensitive"].as_bool().unwrap_or(false);
                match Toolset::search_code(dir, query, case_sensitive) {
                    Ok(matches) => {
                        let lines: Vec<String> = matches
                            .into_iter()
                            .map(|m| {
                                format!("{}:{}: {}", m.file.display(), m.line_number, m.line_text)
                            })
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
                let cmd = call.arguments["cmd"]
                    .as_str()
                    .or_else(|| call.arguments["command"].as_str())
                    .unwrap_or_default();
                match Toolset::run_command(cmd, working_dir) {
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
            custom_name => {
                // Check if custom_name is registered in dynamic skill tools
                if let Some(registry) = dynamic_tools {
                    if let Some(tool) = registry.get(custom_name) {
                        return match tool.execute_sync(&call.arguments, working_dir) {
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
                                output: format!("Dynamic tool '{}' execution error: {e}", custom_name),
                            },
                        };
                    }
                }

                ToolResult {
                    success: false,
                    output: format!("Unknown tool: {}", custom_name),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ride_skills::{DynamicSkillTool, DynamicToolDefinition};
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

    #[test]
    fn test_execute_dynamic_tool_integration() {
        let temp_dir = std::env::temp_dir().join("ride_test_executor_dynamic");
        let _ = std::fs::create_dir_all(&temp_dir);

        if cfg!(target_os = "windows") {
            let script_file = temp_dir.join("test_dynamic.bat");
            std::fs::write(&script_file, "@echo off\r\necho dynamic_tool_ran_successfully\r\n").unwrap();

            let def = DynamicToolDefinition {
                name: "test_dynamic".into(),
                description: "Test dynamic tool in executor".into(),
                parameters: json!({}),
                script: "test_dynamic.bat".into(),
                timeout_ms: 5000,
            };

            let mut registry = DynamicToolRegistry::new();
            registry.register(DynamicSkillTool::new(def, &temp_dir));

            let call = ParsedToolCall {
                name: "test_dynamic".into(),
                arguments: json!({}),
            };

            let res = AgentExecutor::execute_tool_with_dynamic(&call, Some(&registry), None);
            assert!(res.success, "Tool execution failed: {}", res.output);
            assert!(res.output.contains("dynamic_tool_ran_successfully"));
        }

        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
