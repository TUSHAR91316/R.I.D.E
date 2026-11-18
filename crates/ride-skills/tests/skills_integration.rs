use ride_skills::{
    DynamicSkillTool, DynamicToolDefinition, DynamicToolRegistry, PromptSynthesisResult,
    SkillContextBudgeter, SkillManifest, SkillMetadata, SkillRegistry, SystemPromptBuilder,
};
use serde_json::json;
use std::path::PathBuf;

#[test]
fn test_skill_manifest_robust_parsing_edge_cases() {
    // 1. BOM + CRLF + all frontmatter fields using raw string literal
    let raw = concat!(
        "\u{feff}",
        r#"---
name: deep-refactor
description: Structural code refactoring specialist
version: 2.1.0
author: RIDE Team
triggers:
  - /refactor(ing)?/
  - restructure
max_tokens: 800
pinned: true
compatible_models:
  - qwen2.5-coder
  - deepseek-coder
enabled: true
tools:
  - name: ast_check
    description: Run syntax AST check
    parameters:
      type: object
      properties:
        file: { type: string }
      required: [file]
    script: scripts/ast.py
---
# Deep Refactor
Always produce minimal diffs and verify invariants."#
    );

    let manifest = SkillManifest::parse_skill_markdown(raw).unwrap();
    assert_eq!(manifest.metadata.name, "deep-refactor");
    assert_eq!(manifest.metadata.version, "2.1.0");
    assert_eq!(manifest.metadata.author, Some("RIDE Team".into()));
    assert_eq!(manifest.metadata.triggers.len(), 2);
    assert!(manifest.metadata.pinned);
    assert_eq!(manifest.metadata.compatible_models.len(), 2);
    assert_eq!(manifest.tools.len(), 1);
    assert_eq!(manifest.tools[0].name, "ast_check");
    assert!(manifest.instructions.contains("minimal diffs"));

    // 2. Whitespace-only name should fail cleanly
    let invalid_name = "---\nname: '   '\n---\nBody";
    assert!(SkillManifest::parse_skill_markdown(invalid_name).is_err());

    // 3. Unclosed delimiter should fail cleanly
    let unclosed = "---\nname: unclosed\nNo delimiter here";
    assert!(SkillManifest::parse_skill_markdown(unclosed).is_err());
}

#[test]
fn test_dual_scope_precedence_and_collisions_integration() {
    let tmp = std::env::temp_dir().join("ride_dual_scope_test");
    let global_dir = tmp.join("global");
    let ws_root = tmp.join("workspace");
    let ws_skills = ws_root.join(".ride").join("skills");

    let global_skill_dir = global_dir.join("code-review");
    let ws_skill_dir = ws_skills.join("code-review");
    let ws_unique_dir = ws_skills.join("local-only");

    let _ = std::fs::create_dir_all(&global_skill_dir);
    let _ = std::fs::create_dir_all(&ws_skill_dir);
    let _ = std::fs::create_dir_all(&ws_unique_dir);

    // Global version of code-review
    std::fs::write(
        global_skill_dir.join("SKILL.md"),
        "---\nname: code-review\ndescription: Global Reviewer\nversion: 1.0.0\n---\nGlobal instructions",
    ).unwrap();

    // Workspace version of code-review (overrides global)
    std::fs::write(
        ws_skill_dir.join("SKILL.md"),
        "---\nname: code-review\ndescription: Project Custom Reviewer\nversion: 2.0.0\n---\nProject custom instructions",
    ).unwrap();

    // Workspace unique skill
    std::fs::write(
        ws_unique_dir.join("SKILL.md"),
        "---\nname: local-only\ndescription: Only in this project\n---\nLocal instructions",
    ).unwrap();

    let mut registry = SkillRegistry::new();
    let summary = registry.scan_dual(&global_dir, Some(&ws_root)).unwrap();

    assert_eq!(summary.global_count, 1);
    assert_eq!(summary.workspace_count, 2);
    assert_eq!(summary.collisions.len(), 1);
    assert_eq!(summary.collisions[0].skill_name, "code-review");

    // Effective active skill should be workspace version
    let active = registry.get("code-review").unwrap();
    assert_eq!(active.metadata.description, "Project Custom Reviewer");
    assert_eq!(active.metadata.version, "2.0.0");
    assert_eq!(active.instructions, "Project custom instructions");
    assert!(registry.is_overridden("code-review"));

    // Global skill can still be inspected
    let global_ver = registry.get_global("code-review").unwrap();
    assert_eq!(global_ver.metadata.description, "Global Reviewer");

    // Toggling enable/disable
    assert!(registry.enable_skill("code-review", false));
    assert!(!registry.get("code-review").unwrap().metadata.enabled);

    assert!(registry.enable_skill("code-review", true));
    assert!(registry.get("code-review").unwrap().metadata.enabled);

    let _ = std::fs::remove_dir_all(tmp);
}

#[test]
fn test_token_budgeter_and_dynamic_activation_matrix() {
    fn make_skill_helper(
        name: &str,
        triggers: Vec<&str>,
        pinned: bool,
        models: Vec<&str>,
        instr_len: usize,
    ) -> SkillManifest {
        SkillManifest {
            metadata: SkillMetadata {
                name: name.into(),
                description: format!("Description for {name}"),
                version: "1.0.0".into(),
                author: None,
                triggers: triggers.into_iter().map(String::from).collect(),
                max_tokens: 1024,
                pinned,
                compatible_models: models.into_iter().map(String::from).collect(),
                enabled: true,
                tools: vec![],
            },
            instructions: "X".repeat(instr_len),
            source_path: None,
            base_dir: None,
            tools: vec![],
        }
    }

    // Skill 1: Pinned + Triggered (Highest priority)
    let s1 = make_skill_helper("rust-core", vec!["rust", "cargo"], true, vec![], 200);
    // Skill 2: Pinned only
    let s2 = make_skill_helper("safety-guard", vec!["audit"], true, vec![], 200);
    // Skill 3: Triggered only (keyword)
    let s3 = make_skill_helper("web-css", vec!["css", "tailwind"], false, vec![], 400);
    // Skill 4: Triggered only (regex)
    let s4 = make_skill_helper("git-flow", vec![r"\b(git|commit|rebase)\b"], false, vec![], 400);
    // Skill 5: Inactive (not triggered)
    let s5 = make_skill_helper("docker-ops", vec!["docker", "k8s"], false, vec![], 300);
    // Skill 6: Incompatible model
    let s6 = make_skill_helper("colibri-special", vec!["moe"], true, vec!["colibri-moe"], 200);

    let all_skills = vec![&s1, &s2, &s3, &s4, &s5, &s6];

    // Scenario A: Generous budget with model filtering
    let budgeter = SkillContextBudgeter::new(4000);
    let budgeted = budgeter.budget_skills(
        &all_skills,
        "Help me format CSS and commit via git",
        Some("qwen2.5-coder:7b"),
    );

    // s1 is pinned, s2 is pinned, s3 triggered by CSS, s4 triggered by git
    let active_names: Vec<&str> = budgeted.activated.iter().map(|s| s.metadata.name.as_str()).collect();
    assert!(active_names.contains(&"rust-core"));
    assert!(active_names.contains(&"safety-guard"));
    assert!(active_names.contains(&"web-css"));
    assert!(active_names.contains(&"git-flow"));

    // s5 was not triggered, so it should be in summarized list!
    let summary_names: Vec<&str> = budgeted.summarized.iter().map(|s| s.name.as_str()).collect();
    assert!(summary_names.contains(&"docker-ops"));

    // s6 is incompatible with qwen2.5-coder, so omitted
    assert!(budgeted.omitted.iter().any(|o| o.contains("colibri-special")));

    // Scenario B: Moderate budget (250 tokens)
    // 3 pinned skills get activated (approx 210 tokens), triggered skills demoted to summaries
    let mod_budgeter = SkillContextBudgeter::new(250);
    let mod_res = mod_budgeter.budget_skills(
        &all_skills,
        "Help me format CSS and commit via git",
        None,
    );

    assert_eq!(mod_res.activated.len(), 3);
    assert!(mod_res.activated.iter().any(|s| s.metadata.name == "rust-core"));
    assert!(mod_res.activated.iter().any(|s| s.metadata.name == "safety-guard"));
    assert!(mod_res.activated.iter().any(|s| s.metadata.name == "colibri-special"));
    // Demoted skills get summarized
    assert!(!mod_res.summarized.is_empty());
    assert!(mod_res.summarized.iter().any(|s| s.name == "web-css" || s.name == "git-flow"));

    // Scenario C: Tiny budget (50 tokens)
    // Budget cannot even fit full instructions of one skill -> demoted to summaries/omitted
    let tiny_budgeter = SkillContextBudgeter::new(50);
    let tiny_res = tiny_budgeter.budget_skills(
        &all_skills,
        "Help me format CSS and commit via git",
        None,
    );

    assert_eq!(tiny_res.activated.len(), 0);
    // At least some summarized or omitted
    assert!(!tiny_res.omitted.is_empty() || !tiny_res.summarized.is_empty());
}

#[test]
fn test_prompt_synthesis_and_openai_schema_integration() {
    let mut manifest = SkillManifest {
        metadata: SkillMetadata {
            name: "unit-tester".into(),
            description: "Generate thorough unit tests".into(),
            version: "1.0.0".into(),
            author: None,
            triggers: vec!["test".into()],
            max_tokens: 500,
            pinned: false,
            compatible_models: vec![],
            enabled: true,
            tools: vec![],
        },
        instructions: "Always write failing tests first, then implement.".into(),
        source_path: None,
        base_dir: None,
        tools: vec![],
    };

    let def = DynamicToolDefinition {
        name: "run_test_suite".into(),
        description: "Executes test suite".into(),
        parameters: json!({
            "type": "object",
            "properties": {
                "filter": { "type": "string" }
            }
        }),
        script: "test_runner.bat".into(),
        timeout_ms: 10000,
    };
    manifest.tools.push(def.clone());

    let mut dyn_tools = DynamicToolRegistry::new();
    dyn_tools.register(DynamicSkillTool::new(def, PathBuf::from(".")));

    let builder = SystemPromptBuilder::new()
        .with_dynamic_tools(dyn_tools);

    let res: PromptSynthesisResult = builder.build(&[&manifest], "Please run the test suite", None);

    // Prompt contains instructions
    assert!(res.system_prompt.contains("Active Agent Skills"));
    assert!(res.system_prompt.contains("unit-tester"));
    assert!(res.system_prompt.contains("failing tests first"));

    // Prompt contains custom tool schema
    assert!(res.system_prompt.contains("run_test_suite"));
    assert!(res.system_prompt.contains("Executes test suite"));

    // OpenAI JSON array contains both built-in tools and dynamic tool
    let tools_array = res.openai_tools.as_array().expect("Tools must be an array");
    let names: Vec<&str> = tools_array
        .iter()
        .filter_map(|t| t["function"]["name"].as_str())
        .collect();

    assert!(names.contains(&"read_file"));
    assert!(names.contains(&"write_file"));
    assert!(names.contains(&"edit_file"));
    assert!(names.contains(&"list_dir"));
    assert!(names.contains(&"search_code"));
    assert!(names.contains(&"run_command"));
    assert!(names.contains(&"run_test_suite"));
}
