use ride_agent::{
    AgentSession, AgentSessionConfig, ApprovalDecision, ApprovalPolicy,
    DiffManager, MockLlmClient, TerminalApprovalGate,
    ToolOutputBudget, ToolOutputBudgeter,
};
use ride_core::events::AgentEvent;
use ride_skills::{DynamicSkillTool, DynamicToolDefinition, DynamicToolRegistry, SkillManifest, SkillMetadata};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn test_end_to_end_agent_session_multi_turn_with_skills_and_diffs() {
    let tmp_dir = std::env::temp_dir().join("ride_e2e_agent_test");
    let _ = std::fs::create_dir_all(&tmp_dir);

    let main_rs = tmp_dir.join("main.rs");
    std::fs::write(
        &main_rs,
        "fn calculate_score() -> i32 {\n    10\n}\n",
    )
    .unwrap();

    // Set up active skill
    let skill = SkillManifest {
        metadata: SkillMetadata {
            name: "math-refactor".into(),
            description: "Refactor score calculators".into(),
            version: "1.0.0".into(),
            author: None,
            triggers: vec!["calculate".into(), "score".into()],
            max_tokens: 1000,
            pinned: true,
            compatible_models: vec![],
            enabled: true,
            tools: vec![],
        },
        instructions: "Always replace hardcoded numbers with parameterized constants.".into(),
        source_path: None,
        base_dir: Some(tmp_dir.clone()),
        tools: vec![],
    };

    // Simulated LLM conversation:
    // Turn 1 (LLM calls read_file)
    // Turn 2 (LLM calls edit_file with unified diff)
    // Turn 3 (LLM provides final response)
    let responses = vec![
        format!(
            "<tool_call name=\"read_file\"><path>{}</path></tool_call>",
            main_rs.to_string_lossy().replace('\\', "/")
        ),
        format!(
            "<tool_call name=\"edit_file\"><path>{}</path><target>10</target><replacement>DEFAULT_SCORE</replacement></tool_call>",
            main_rs.to_string_lossy().replace('\\', "/")
        ),
        "Refactoring complete: replaced hardcoded 10 with DEFAULT_SCORE constant as requested by math-refactor skill.".into(),
    ];

    let mock_client = Arc::new(MockLlmClient::new_with_queue(responses));
    let (event_tx, mut event_rx) = ride_core::new_agent_event_bus();

    let diff_mgr = Arc::new(DiffManager::new(2));
    let approval_gate = Arc::new(TerminalApprovalGate::new(ApprovalPolicy::AlwaysAllow));

    let config = AgentSessionConfig {
        max_turns: 5,
        working_dir: Some(tmp_dir.clone()),
        ..Default::default()
    };

    let mut session = AgentSession::new(config, mock_client)
        .with_skills(vec![skill])
        .with_diff_manager(diff_mgr)
        .with_approval_gate(approval_gate)
        .with_event_sender(event_tx);

    let cancel = CancellationToken::new();
    let turn_result = session
        .execute_turn("Refactor calculate_score", cancel)
        .await
        .unwrap();

    // Verify turn loop completion
    assert!(turn_result.completed);
    assert_eq!(turn_result.turn_count, 3);
    assert_eq!(turn_result.executed_tools.len(), 2);
    assert_eq!(turn_result.executed_tools[0].call.name, "read_file");
    assert_eq!(turn_result.executed_tools[1].call.name, "edit_file");

    // Verify diff generation
    assert_eq!(turn_result.diffs.len(), 1);
    let diff = &turn_result.diffs[0];
    assert!(diff.diff_text.contains("-    10"));
    assert!(diff.diff_text.contains("+    DEFAULT_SCORE"));

    // Verify file content on disk
    let updated_content = std::fs::read_to_string(&main_rs).unwrap();
    assert!(updated_content.contains("DEFAULT_SCORE"));

    // Verify single-click rollback
    let rollback_snap = session.rollback_latest(&main_rs).unwrap();
    assert_eq!(
        rollback_snap.previous_content,
        Some("fn calculate_score() -> i32 {\n    10\n}\n".into())
    );
    let rolled_back_content = std::fs::read_to_string(&main_rs).unwrap();
    assert!(rolled_back_content.contains("10"));
    assert!(!rolled_back_content.contains("DEFAULT_SCORE"));

    // Verify events were emitted to the event bus
    let mut received_events = Vec::new();
    while let Ok(ev) = event_rx.try_recv() {
        received_events.push(ev);
    }
    assert!(received_events.iter().any(|e| matches!(e, AgentEvent::TurnStarted { .. })));
    assert!(received_events.iter().any(|e| matches!(e, AgentEvent::ToolCallRequested { tool_name, .. } if tool_name == "read_file")));
    assert!(received_events.iter().any(|e| matches!(e, AgentEvent::DiffGenerated { .. })));
    assert!(received_events.iter().any(|e| matches!(e, AgentEvent::TurnCompleted { .. })));

    let _ = std::fs::remove_dir_all(tmp_dir);
}

#[tokio::test]
async fn test_end_to_end_dynamic_skill_tool_with_approval_gate() {
    let tmp_dir = std::env::temp_dir().join("ride_e2e_dyn_tool");
    let _ = std::fs::create_dir_all(&tmp_dir);

    // Dynamic tool script
    let script_name = if cfg!(target_os = "windows") {
        let script = tmp_dir.join("calc.bat");
        std::fs::write(&script, "@echo off\r\necho RESULT: 42\r\n").unwrap();
        "calc.bat"
    } else {
        let script = tmp_dir.join("calc.sh");
        std::fs::write(&script, "#!/bin/sh\necho 'RESULT: 42'\n").unwrap();
        "calc.sh"
    };

    let def = DynamicToolDefinition {
        name: "calculate_custom".into(),
        description: "Calculate custom values".into(),
        parameters: json!({}),
        script: script_name.into(),
        timeout_ms: 5000,
    };

    let mut dyn_registry = DynamicToolRegistry::new();
    dyn_registry.register(DynamicSkillTool::new(def, &tmp_dir));

    let responses = vec![
        "<tool_call name=\"calculate_custom\"></tool_call>".into(),
        "Calculation gave 42.".into(),
    ];

    let mock_client = Arc::new(MockLlmClient::new_with_queue(responses));
    let config = AgentSessionConfig {
        working_dir: Some(tmp_dir.clone()),
        ..Default::default()
    };

    let mut session = AgentSession::new(config, mock_client)
        .with_dynamic_tools(dyn_registry);

    let cancel = CancellationToken::new();
    let res = session.execute_turn("Run calc", cancel).await.unwrap();

    assert!(res.completed);
    assert_eq!(res.executed_tools.len(), 1);
    assert!(res.executed_tools[0].output.contains("RESULT: 42"));

    let _ = std::fs::remove_dir_all(tmp_dir);
}

#[tokio::test]
async fn test_end_to_end_dynamic_skill_tool_denied_by_approval_gate() {
    let tmp_dir = std::env::temp_dir().join("ride_e2e_dyn_tool_deny");
    let _ = std::fs::create_dir_all(&tmp_dir);

    let script_name = if cfg!(target_os = "windows") {
        let script = tmp_dir.join("calc_deny.bat");
        std::fs::write(&script, "@echo off\r\necho DANGEROUS\r\n").unwrap();
        "calc_deny.bat"
    } else {
        let script = tmp_dir.join("calc_deny.sh");
        std::fs::write(&script, "#!/bin/sh\necho 'DANGEROUS'\n").unwrap();
        "calc_deny.sh"
    };

    let def = DynamicToolDefinition {
        name: "calculate_custom_denied".into(),
        description: "Calculate custom values".into(),
        parameters: json!({}),
        script: script_name.into(),
        timeout_ms: 5000,
    };

    let mut dyn_registry = DynamicToolRegistry::new();
    dyn_registry.register(DynamicSkillTool::new(def, &tmp_dir));

    let responses = vec![
        "<tool_call name=\"calculate_custom_denied\"></tool_call>".into(),
        "Calculation was blocked.".into(),
    ];

    let mock_client = Arc::new(MockLlmClient::new_with_queue(responses));
    let approval_gate = Arc::new(TerminalApprovalGate::new(ApprovalPolicy::Deny));

    let config = AgentSessionConfig {
        approval_policy: ApprovalPolicy::Deny,
        working_dir: Some(tmp_dir.clone()),
        ..Default::default()
    };

    let mut session = AgentSession::new(config, mock_client)
        .with_dynamic_tools(dyn_registry)
        .with_approval_gate(approval_gate);

    let cancel = CancellationToken::new();
    let res = session.execute_turn("Run calc", cancel).await.unwrap();

    assert!(res.completed);
    assert_eq!(res.executed_tools.len(), 1);
    assert!(!res.executed_tools[0].success);
    assert!(res.executed_tools[0].output.contains("denied by Deny policy"));

    let _ = std::fs::remove_dir_all(tmp_dir);
}

#[tokio::test]
async fn test_end_to_end_terminal_approval_denial_halts_destructive_command() {
    let responses = vec![
        "<tool_call name=\"run_command\"><cmd>rm -rf /sensitive_data</cmd></tool_call>".into(),
        "I could not delete the data because permission was denied.".into(),
    ];

    let mock_client = Arc::new(MockLlmClient::new_with_queue(responses));

    // Approval gate with user rejection
    let (tx, mut rx) = mpsc::channel(4);
    let approval_gate = Arc::new(
        TerminalApprovalGate::new(ApprovalPolicy::AskEveryTime).with_channel(tx),
    );

    tokio::spawn(async move {
        if let Some((_req, resp_tx)) = rx.recv().await {
            let _ = resp_tx.send(ApprovalDecision::Denied {
                reason: "Destructive command rejected by human reviewer".into(),
            });
        }
    });

    let config = AgentSessionConfig {
        approval_policy: ApprovalPolicy::AskEveryTime,
        ..Default::default()
    };

    let mut session = AgentSession::new(config, mock_client).with_approval_gate(approval_gate);

    let cancel = CancellationToken::new();
    let res = session.execute_turn("Clean sensitive data", cancel).await.unwrap();

    assert!(res.completed);
    assert_eq!(res.executed_tools.len(), 1);
    assert!(!res.executed_tools[0].success);
    assert!(res.executed_tools[0].output.contains("Destructive command rejected by human reviewer"));
}

#[tokio::test]
async fn test_end_to_end_context_budget_truncation_warning() {
    let long_output = (1..=500).map(|i| format!("log line {i}")).collect::<Vec<_>>().join("\n");
    let budget = ToolOutputBudget {
        max_chars: 500,
        max_lines: 20,
        ..Default::default()
    };

    let truncated = ToolOutputBudgeter::truncate(&long_output, &budget);
    assert!(truncated.contains("[Output truncated: showing 20 of 500 lines"));
    assert!(truncated.chars().count() < 1000);
}
