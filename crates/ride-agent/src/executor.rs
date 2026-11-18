use crate::approval::TerminalApprovalGate;
use crate::budget::{ToolOutputBudget, ToolOutputBudgeter};
use crate::diff::{BackupSnapshot, DiffManager, UnifiedDiff};
use crate::parser::ParsedToolCall;
use crate::tools::{ToolResult, Toolset};
use ride_skills::{DynamicToolRegistry, PromptSynthesisResult, SkillManifest, SystemPromptBuilder};
use std::path::Path;

/// Outcome of a tool execution including potential diffs and backup snapshots.
#[derive(Debug, Clone)]
pub struct ToolExecutionOutcome {
    pub success: bool,
    pub output: String,
    pub diff: Option<UnifiedDiff>,
    pub snapshot: Option<BackupSnapshot>,
}

pub struct AgentExecutor;

impl AgentExecutor {
    /// Formats active skill instructions into a system prompt segment for local LLMs
    pub fn build_skills_prompt(skills: &[&SkillManifest]) -> String {
        let active_skills: Vec<&SkillManifest> = skills
            .iter()
            .copied()
            .filter(|s| s.metadata.enabled)
            .collect();

        if active_skills.is_empty() {
            return String::new();
        }

        let mut out = String::from("\n## Active Agent Skills\n");
        for skill in active_skills {
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
        } else {
            // Auto-populate tools from active skills when no registry is passed
            let mut auto_tools = DynamicToolRegistry::new();
            for skill in skills {
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
                    .unwrap_or_else(|| std::path::PathBuf::from("."));
                for tool_def in &skill.tools {
                    auto_tools.register(ride_skills::DynamicSkillTool::new(tool_def.clone(), &base_dir));
                }
            }
            if !auto_tools.is_empty() {
                builder = builder.with_dynamic_tools(auto_tools);
            }
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

    /// Executes tool call with full security approval gate, diff tracking, and output context budgeting.
    pub async fn execute_tool_full(
        call: &ParsedToolCall,
        dynamic_tools: Option<&DynamicToolRegistry>,
        working_dir: Option<&Path>,
        approval_gate: Option<&TerminalApprovalGate>,
        diff_manager: Option<&DiffManager>,
        budget: &ToolOutputBudget,
    ) -> ToolExecutionOutcome {
        match call.name.as_str() {
            "read_file" => {
                let path = call.arguments["path"].as_str().unwrap_or_default();
                let start = call.arguments["start_line"].as_u64().map(|v| v as usize);
                let end = call.arguments["end_line"].as_u64().map(|v| v as usize);
                let resolved_path = match working_dir {
                    Some(dir) if !Path::new(path).is_absolute() => dir.join(path),
                    _ => Path::new(path).to_path_buf(),
                };
                match Toolset::read_file_budgeted(&resolved_path, start, end, budget.max_file_read_lines) {
                    Ok(out) => {
                        let budgeted = ToolOutputBudgeter::truncate(&out, budget);
                        ToolExecutionOutcome {
                            success: true,
                            output: budgeted,
                            diff: None,
                            snapshot: None,
                        }
                    }
                    Err(e) => ToolExecutionOutcome {
                        success: false,
                        output: e.to_string(),
                        diff: None,
                        snapshot: None,
                    },
                }
            }
            "write_file" => {
                let path = call.arguments["path"].as_str().unwrap_or_default();
                let content = call.arguments["content"].as_str().unwrap_or_default();
                let resolved_path = match working_dir {
                    Some(dir) if !Path::new(path).is_absolute() => dir.join(path),
                    _ => Path::new(path).to_path_buf(),
                };

                if let Some(dm) = diff_manager {
                    match dm.apply_write_with_backup(&resolved_path, content) {
                        Ok((diff, snap)) => ToolExecutionOutcome {
                            success: true,
                            output: format!(
                                "Successfully wrote {} ({} additions, {} deletions)",
                                path, diff.additions, diff.deletions
                            ),
                            diff: Some(diff),
                            snapshot: Some(snap),
                        },
                        Err(e) => ToolExecutionOutcome {
                            success: false,
                            output: e.to_string(),
                            diff: None,
                            snapshot: None,
                        },
                    }
                } else {
                    match Toolset::write_file(&resolved_path, content) {
                        Ok(_) => ToolExecutionOutcome {
                            success: true,
                            output: format!("Successfully wrote {path}"),
                            diff: None,
                            snapshot: None,
                        },
                        Err(e) => ToolExecutionOutcome {
                            success: false,
                            output: e.to_string(),
                            diff: None,
                            snapshot: None,
                        },
                    }
                }
            }
            "edit_file" => {
                let path = call.arguments["path"].as_str().unwrap_or_default();
                let target = call.arguments["target"].as_str().unwrap_or_default();
                let replacement = call.arguments["replacement"].as_str().unwrap_or_default();
                let resolved_path = match working_dir {
                    Some(dir) if !Path::new(path).is_absolute() => dir.join(path),
                    _ => Path::new(path).to_path_buf(),
                };

                if let Some(dm) = diff_manager {
                    match dm.apply_edit_with_backup(&resolved_path, target, replacement) {
                        Ok((diff, snap)) => ToolExecutionOutcome {
                            success: true,
                            output: format!(
                                "Successfully edited {} ({} additions, {} deletions)",
                                path, diff.additions, diff.deletions
                            ),
                            diff: Some(diff),
                            snapshot: Some(snap),
                        },
                        Err(e) => ToolExecutionOutcome {
                            success: false,
                            output: e.to_string(),
                            diff: None,
                            snapshot: None,
                        },
                    }
                } else {
                    match Toolset::edit_file(&resolved_path, target, replacement) {
                        Ok(_) => ToolExecutionOutcome {
                            success: true,
                            output: format!("Successfully edited {path}"),
                            diff: None,
                            snapshot: None,
                        },
                        Err(e) => ToolExecutionOutcome {
                            success: false,
                            output: e.to_string(),
                            diff: None,
                            snapshot: None,
                        },
                    }
                }
            }
            "list_dir" => {
                let path = call.arguments["path"].as_str().unwrap_or(".");
                let resolved_path = match working_dir {
                    Some(dir) if !Path::new(path).is_absolute() => dir.join(path),
                    _ => Path::new(path).to_path_buf(),
                };
                match Toolset::list_dir_budgeted(&resolved_path, budget.max_dir_entries) {
                    Ok(entries) => {
                        let lines: Vec<String> = entries
                            .into_iter()
                            .map(|p| p.to_string_lossy().to_string())
                            .collect();
                        let raw = lines.join("\n");
                        let budgeted = ToolOutputBudgeter::truncate(&raw, budget);
                        ToolExecutionOutcome {
                            success: true,
                            output: budgeted,
                            diff: None,
                            snapshot: None,
                        }
                    }
                    Err(e) => ToolExecutionOutcome {
                        success: false,
                        output: e.to_string(),
                        diff: None,
                        snapshot: None,
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
                let resolved_path = match working_dir {
                    Some(d) if !Path::new(dir).is_absolute() => d.join(dir),
                    _ => Path::new(dir).to_path_buf(),
                };
                match Toolset::search_code_budgeted(
                    &resolved_path,
                    query,
                    case_sensitive,
                    budget.max_search_matches,
                ) {
                    Ok(matches) => {
                        let lines: Vec<String> = matches
                            .into_iter()
                            .map(|m| {
                                format!("{}:{}: {}", m.file.display(), m.line_number, m.line_text)
                            })
                            .collect();
                        let raw = lines.join("\n");
                        let budgeted = ToolOutputBudgeter::truncate(&raw, budget);
                        ToolExecutionOutcome {
                            success: true,
                            output: budgeted,
                            diff: None,
                            snapshot: None,
                        }
                    }
                    Err(e) => ToolExecutionOutcome {
                        success: false,
                        output: e.to_string(),
                        diff: None,
                        snapshot: None,
                    },
                }
            }
            "run_command" => {
                let cmd = call.arguments["cmd"]
                    .as_str()
                    .or_else(|| call.arguments["command"].as_str())
                    .unwrap_or_default();

                // Check security approval gate
                if let Some(gate) = approval_gate {
                    if let Err(e) = gate.check_approval(cmd, working_dir).await {
                        return ToolExecutionOutcome {
                            success: false,
                            output: e.to_string(),
                            diff: None,
                            snapshot: None,
                        };
                    }
                }

                match Toolset::run_command(cmd, working_dir) {
                    Ok(out) => {
                        let combined = if !out.stderr.is_empty() {
                            format!("{}\n[stderr]: {}", out.stdout, out.stderr)
                        } else {
                            out.stdout
                        };
                        let budgeted = ToolOutputBudgeter::truncate(&combined, budget);
                        ToolExecutionOutcome {
                            success: out.exit_code == 0,
                            output: budgeted,
                            diff: None,
                            snapshot: None,
                        }
                    }
                    Err(e) => ToolExecutionOutcome {
                        success: false,
                        output: e.to_string(),
                        diff: None,
                        snapshot: None,
                    },
                }
            }
            custom_name => {
                if let Some(registry) = dynamic_tools {
                    if let Some(tool) = registry.get(custom_name) {
                        return match tool.execute_sync(&call.arguments, working_dir) {
                            Ok(out) => {
                                let combined = if !out.stderr.is_empty() {
                                    format!("{}\n[stderr]: {}", out.stdout, out.stderr)
                                } else {
                                    out.stdout
                                };
                                let budgeted = ToolOutputBudgeter::truncate(&combined, budget);
                                ToolExecutionOutcome {
                                    success: out.exit_code == 0,
                                    output: budgeted,
                                    diff: None,
                                    snapshot: None,
                                }
                            }
                            Err(e) => ToolExecutionOutcome {
                                success: false,
                                output: format!(
                                    "Dynamic tool '{}' execution error: {e}",
                                    custom_name
                                ),
                                diff: None,
                                snapshot: None,
                            },
                        };
                    }
                }

                ToolExecutionOutcome {
                    success: false,
                    output: format!("Unknown tool: {}", custom_name),
                    diff: None,
                    snapshot: None,
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

    #[test]
    fn test_build_system_prompt_auto_populates_dynamic_tools() {
        let manifest = SkillManifest {
            metadata: ride_skills::SkillMetadata {
                name: "auto-tool-skill".into(),
                description: "Skill with embedded tool".into(),
                version: "1.0.0".into(),
                author: None,
                triggers: vec!["auto".into()],
                max_tokens: 500,
                pinned: true,
                compatible_models: vec![],
                enabled: true,
                tools: vec![],
            },
            instructions: "Auto tool instructions".into(),
            source_path: None,
            base_dir: Some(std::path::PathBuf::from(".")),
            tools: vec![DynamicToolDefinition {
                name: "auto_action".into(),
                description: "Action from skill".into(),
                parameters: json!({"type": "object"}),
                script: "action.bat".into(),
                timeout_ms: 5000,
            }],
        };

        // Pass dynamic_tools: None
        let res = AgentExecutor::build_system_prompt(&[&manifest], None, "auto prompt", None);

        // Tool should be auto-extracted and present in prompt and openai_tools
        assert!(res.system_prompt.contains("auto_action"));
        let tools_arr = res.openai_tools.as_array().unwrap();
        assert!(tools_arr.iter().any(|t| t["function"]["name"] == "auto_action"));
    }

    #[test]
    fn test_build_skills_prompt_ignores_disabled() {
        let enabled_skill = SkillManifest {
            metadata: ride_skills::SkillMetadata {
                name: "enabled-skill".into(),
                description: "Enabled".into(),
                version: "1.0.0".into(),
                author: None,
                triggers: vec![],
                max_tokens: 500,
                pinned: true,
                compatible_models: vec![],
                enabled: true,
                tools: vec![],
            },
            instructions: "Enabled instructions".into(),
            source_path: None,
            base_dir: None,
            tools: vec![],
        };

        let mut disabled_skill = enabled_skill.clone();
        disabled_skill.metadata.name = "disabled-skill".into();
        disabled_skill.metadata.enabled = false;
        disabled_skill.instructions = "Disabled instructions".into();

        let prompt = AgentExecutor::build_skills_prompt(&[&enabled_skill, &disabled_skill]);
        assert!(prompt.contains("enabled-skill"));
        assert!(!prompt.contains("disabled-skill"));
    }
}
