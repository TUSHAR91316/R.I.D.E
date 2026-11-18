use crate::approval::{ApprovalPolicy, TerminalApprovalGate};
use crate::budget::ToolOutputBudget;
use crate::diff::{BackupSnapshot, DiffManager, UnifiedDiff};
use crate::executor::AgentExecutor;
use crate::parser::{AgentToolParser, ParsedToolCall};
use reqwest::Client;
use ride_core::error::{Result, RideError};
use ride_core::events::AgentEvent;
use ride_core::model::ProviderType;
use ride_discovery::sse::{stream_sse, SseFormat, SseToken};
use ride_skills::{DynamicToolRegistry, SkillManifest};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_util::sync::CancellationToken;

/// The role of a participant in the conversation history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
    Tool,
}

/// A single message in the multi-turn conversation history.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ParsedToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    pub timestamp_secs: u64,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::System,
            content: content.into(),
            tool_calls: None,
            tool_name: None,
            timestamp_secs: current_timestamp(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::User,
            content: content.into(),
            tool_calls: None,
            tool_name: None,
            timestamp_secs: current_timestamp(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::Assistant,
            content: content.into(),
            tool_calls: None,
            tool_name: None,
            timestamp_secs: current_timestamp(),
        }
    }

    pub fn assistant_with_tools(
        content: impl Into<String>,
        tool_calls: Vec<ParsedToolCall>,
    ) -> Self {
        Self {
            role: ChatRole::Assistant,
            content: content.into(),
            tool_calls: Some(tool_calls),
            tool_name: None,
            timestamp_secs: current_timestamp(),
        }
    }

    pub fn tool(tool_name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::Tool,
            content: content.into(),
            tool_calls: None,
            tool_name: Some(tool_name.into()),
            timestamp_secs: current_timestamp(),
        }
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Multi-turn conversation history manager with context window safeguarding.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConversationHistory {
    messages: Vec<ChatMessage>,
}

impl ConversationHistory {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    pub fn push(&mut self, msg: ChatMessage) {
        self.messages.push(msg);
    }

    pub fn messages(&self) -> &[ChatMessage] {
        &self.messages
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }

    pub fn insert_or_update_system(&mut self, system_prompt: &str) {
        if self.messages.is_empty() {
            self.messages.push(ChatMessage::system(system_prompt));
        } else if self.messages[0].role == ChatRole::System {
            self.messages[0] = ChatMessage::system(system_prompt);
        } else {
            self.messages.insert(0, ChatMessage::system(system_prompt));
        }
    }

    /// Approximate token count across conversation (heuristic: ~4 chars per token).
    pub fn estimate_token_count(&self) -> usize {
        let total_chars: usize = self
            .messages
            .iter()
            .map(|m| {
                let mut c = m.content.chars().count();
                if let Some(calls) = &m.tool_calls {
                    c += serde_json::to_string(calls).unwrap_or_default().chars().count();
                }
                c
            })
            .sum();
        total_chars / 4
    }

    /// Prunes older intermediate tool and assistant responses if history exceeds the token limit.
    pub fn prune_if_exceeds(&mut self, max_tokens: usize) {
        if self.estimate_token_count() <= max_tokens || self.messages.len() <= 2 {
            return;
        }

        // Keep system prompt (index 0) and latest messages intact
        // Compact intermediate tool outputs
        let len = self.messages.len();
        for i in 1..(len.saturating_sub(2)) {
            if self.messages[i].role == ChatRole::Tool && self.messages[i].content.len() > 300 {
                let note = "\n[Tool output condensed to safeguard context window]";
                let head: String = self.messages[i].content.chars().take(200).collect();
                self.messages[i].content = format!("{head}{note}");
                if self.estimate_token_count() <= max_tokens {
                    return;
                }
            }
        }

        // Sliding window eviction if still over budget: drop oldest non-system message
        while self.estimate_token_count() > max_tokens && self.messages.len() > 2 {
            self.messages.remove(1);
        }
    }
}

/// Abstract local LLM client for sending prompts / messages and receiving completions.
pub trait LlmClient: Send + Sync {
    fn chat_complete<'a>(
        &'a self,
        messages: &'a [ChatMessage],
        cancel: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>>;
}

/// Real HTTP local inference client connecting to Ollama, LM Studio, Colibrì, Rabbit, or llama.cpp.
pub struct HttpLlmClient {
    pub base_url: String,
    pub provider: ProviderType,
    pub model: String,
    pub client: Client,
    pub event_tx: Option<tokio::sync::broadcast::Sender<AgentEvent>>,
    pub stream: bool,
}

impl HttpLlmClient {
    pub fn new(base_url: impl Into<String>, provider: ProviderType, model: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            provider,
            model: model.into(),
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_default(),
            event_tx: None,
            stream: true,
        }
    }

    pub fn with_event_sender(mut self, tx: tokio::sync::broadcast::Sender<AgentEvent>) -> Self {
        self.event_tx = Some(tx);
        self
    }

    pub fn with_streaming(mut self, stream: bool) -> Self {
        self.stream = stream;
        self
    }

    fn build_openai_messages(&self, messages: &[ChatMessage]) -> serde_json::Value {
        let list: Vec<serde_json::Value> = messages
            .iter()
            .map(|m| {
                let role_str = match m.role {
                    ChatRole::System => "system",
                    ChatRole::User => "user",
                    ChatRole::Assistant => "assistant",
                    ChatRole::Tool => "tool",
                };
                let mut obj = serde_json::json!({
                    "role": role_str,
                    "content": m.content,
                });
                if let Some(name) = &m.tool_name {
                    obj["name"] = serde_json::Value::String(name.clone());
                    if m.role == ChatRole::Tool {
                        obj["tool_call_id"] = serde_json::Value::String(format!("call_{name}"));
                    }
                } else if m.role == ChatRole::Tool {
                    obj["tool_call_id"] = serde_json::Value::String("call_tool_0".into());
                }

                if let Some(calls) = &m.tool_calls {
                    let formatted_calls: Vec<serde_json::Value> = calls
                        .iter()
                        .enumerate()
                        .map(|(i, c)| {
                            serde_json::json!({
                                "id": format!("call_{}_{i}", c.name),
                                "type": "function",
                                "function": {
                                    "name": c.name,
                                    "arguments": if c.arguments.is_string() {
                                        c.arguments.as_str().unwrap_or_default().to_string()
                                    } else {
                                        serde_json::to_string(&c.arguments).unwrap_or_default()
                                    }
                                }
                            })
                        })
                        .collect();
                    obj["tool_calls"] = serde_json::Value::Array(formatted_calls);
                }
                obj
            })
            .collect();
        serde_json::Value::Array(list)
    }
}

impl LlmClient for HttpLlmClient {
    fn chat_complete<'a>(
        &'a self,
        messages: &'a [ChatMessage],
        cancel: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
        Box::pin(async move {
            let base = self.base_url.trim_end_matches('/');

            let (url, format) = match self.provider {
                ProviderType::Ollama => (format!("{base}/api/chat"), SseFormat::Ollama),
                _ => (format!("{base}/v1/chat/completions"), SseFormat::OpenAi),
            };

            if self.stream {
                let body = serde_json::json!({
                    "model": self.model,
                    "messages": self.build_openai_messages(messages),
                    "stream": true,
                });

                let req = self.client.post(&url).json(&body);
                let (token_tx, mut token_rx) = tokio::sync::mpsc::channel(128);

                let stream_cancel = cancel.clone();
                let client_ref = self.client.clone();
                let stream_task = tokio::spawn(async move {
                    stream_sse(&client_ref, req, format, token_tx, stream_cancel).await
                });

                let mut full_text = String::new();
                while let Some(token) = token_rx.recv().await {
                    match token {
                        SseToken::Delta(delta) => {
                            if let Some(tx) = &self.event_tx {
                                let _ = tx.send(AgentEvent::ThinkingDelta {
                                    delta: delta.clone(),
                                });
                            }
                            full_text.push_str(&delta);
                        }
                        SseToken::Done => break,
                    }
                }

                match stream_task.await {
                    Ok(Ok(())) if !full_text.trim().is_empty() => return Ok(full_text),
                    Ok(Err(_)) | Err(_) if !full_text.trim().is_empty() => return Ok(full_text),
                    _ => {} // Fallback to non-streaming if stream yielded no text (e.g. tool_calls in message)
                }
            }

            // Non-streaming fallback
            let body = serde_json::json!({
                "model": self.model,
                "messages": self.build_openai_messages(messages),
                "stream": false,
            });

            let req = self.client.post(&url).json(&body);
            let resp = tokio::select! {
                _ = cancel.cancelled() => return Err(RideError::Cancelled),
                res = req.send() => res.map_err(|e| RideError::Agent(format!("Request failed: {e}")))?,
            };

            if !resp.status().is_success() {
                return Err(RideError::Agent(format!("Provider HTTP {}", resp.status())));
            }

            let json: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| RideError::Agent(format!("JSON parse error: {e}")))?;

            let content = match self.provider {
                ProviderType::Ollama => {
                    if let Some(content_str) = json.pointer("/message/content").and_then(serde_json::Value::as_str) {
                        if !content_str.is_empty() {
                            content_str.to_string()
                        } else if let Some(tool_calls) = json.pointer("/message/tool_calls") {
                            serde_json::json!({ "tool_calls": tool_calls }).to_string()
                        } else {
                            String::new()
                        }
                    } else if let Some(tool_calls) = json.pointer("/message/tool_calls") {
                        serde_json::json!({ "tool_calls": tool_calls }).to_string()
                    } else {
                        String::new()
                    }
                }
                _ => {
                    let c_opt = json
                        .pointer("/choices/0/message/content")
                        .or_else(|| json.pointer("/choices/0/text"))
                        .and_then(serde_json::Value::as_str);

                    match c_opt {
                        Some(s) if !s.is_empty() => s.to_string(),
                        _ => {
                            if let Some(tool_calls) = json.pointer("/choices/0/message/tool_calls") {
                                serde_json::json!({ "tool_calls": tool_calls }).to_string()
                            } else {
                                c_opt.unwrap_or_default().to_string()
                            }
                        }
                    }
                }
            };

            Ok(content)
        })
    }
}

/// Mock LLM client for unit and integration testing without requiring local models.
pub struct MockLlmClient {
    responses: Arc<Mutex<Vec<String>>>,
    handler: Option<Arc<dyn Fn(&[ChatMessage]) -> String + Send + Sync>>,
}

impl MockLlmClient {
    pub fn new_with_queue(responses: Vec<String>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses)),
            handler: None,
        }
    }

    pub fn new_with_handler(handler: impl Fn(&[ChatMessage]) -> String + Send + Sync + 'static) -> Self {
        Self {
            responses: Arc::new(Mutex::new(Vec::new())),
            handler: Some(Arc::new(handler)),
        }
    }
}

impl LlmClient for MockLlmClient {
    fn chat_complete<'a>(
        &'a self,
        messages: &'a [ChatMessage],
        cancel: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
        Box::pin(async move {
            if cancel.is_cancelled() {
                return Err(RideError::Cancelled);
            }

            if let Some(handler) = &self.handler {
                return Ok(handler(messages));
            }

            let mut queue = self.responses.lock().unwrap();
            if queue.is_empty() {
                Ok("Mock LLM: No more queued responses.".into())
            } else {
                Ok(queue.remove(0))
            }
        })
    }
}

/// Recorded execution of a single tool call in a turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutedToolCall {
    pub call: ParsedToolCall,
    pub success: bool,
    pub output: String,
    pub diff: Option<UnifiedDiff>,
    pub snapshot: Option<BackupSnapshot>,
}

/// Result of an agent turn loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTurnResult {
    pub turn_count: usize,
    pub final_response: String,
    pub executed_tools: Vec<ExecutedToolCall>,
    pub diffs: Vec<UnifiedDiff>,
    pub completed: bool,
    pub hit_turn_limit: bool,
    pub cancelled: bool,
}

/// Configuration options for an AgentSession.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionConfig {
    pub max_turns: usize,
    pub max_context_tokens: usize,
    pub model_name: String,
    pub working_dir: Option<PathBuf>,
    pub approval_policy: ApprovalPolicy,
    pub tool_budget: ToolOutputBudget,
}

impl Default for AgentSessionConfig {
    fn default() -> Self {
        Self {
            max_turns: 10,
            max_context_tokens: 8192,
            model_name: "local-model".into(),
            working_dir: None,
            approval_policy: ApprovalPolicy::AlwaysAllow,
            tool_budget: ToolOutputBudget::default(),
        }
    }
}

/// Multi-Turn Agent Execution Session managing conversation history, local LLM turn loops,
/// skills injection, approval gates, unified diffs, and rollback snapshots.
pub struct AgentSession {
    config: AgentSessionConfig,
    history: ConversationHistory,
    llm_client: Arc<dyn LlmClient>,
    approval_gate: Arc<TerminalApprovalGate>,
    diff_manager: Arc<DiffManager>,
    dynamic_tools: Option<DynamicToolRegistry>,
    skills: Vec<SkillManifest>,
    event_tx: Option<tokio::sync::broadcast::Sender<AgentEvent>>,
    session_id: String,
    active_turn_count: usize,
}

impl AgentSession {
    pub fn new(config: AgentSessionConfig, llm_client: Arc<dyn LlmClient>) -> Self {
        let approval_gate = Arc::new(TerminalApprovalGate::new(config.approval_policy));
        let diff_manager = Arc::new(DiffManager::default());

        Self {
            config,
            history: ConversationHistory::new(),
            llm_client,
            approval_gate,
            diff_manager,
            dynamic_tools: None,
            skills: Vec::new(),
            event_tx: None,
            session_id: format!("session_{}", current_timestamp()),
            active_turn_count: 0,
        }
    }

    pub fn with_skills(mut self, skills: Vec<SkillManifest>) -> Self {
        self.skills = skills;
        self
    }

    pub fn with_approval_gate(mut self, gate: Arc<TerminalApprovalGate>) -> Self {
        self.approval_gate = gate;
        self
    }

    pub fn with_diff_manager(mut self, diff_manager: Arc<DiffManager>) -> Self {
        self.diff_manager = diff_manager;
        self
    }

    pub fn with_dynamic_tools(mut self, dynamic_tools: DynamicToolRegistry) -> Self {
        self.dynamic_tools = Some(dynamic_tools);
        self
    }

    pub fn with_event_sender(mut self, event_tx: tokio::sync::broadcast::Sender<AgentEvent>) -> Self {
        self.approval_gate = Arc::new(
            TerminalApprovalGate::new(self.config.approval_policy)
                .with_event_sender(event_tx.clone()),
        );
        self.event_tx = Some(event_tx);
        self
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn config(&self) -> &AgentSessionConfig {
        &self.config
    }

    pub fn history(&self) -> &[ChatMessage] {
        self.history.messages()
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
        self.active_turn_count = 0;
    }

    pub fn diff_manager(&self) -> &Arc<DiffManager> {
        &self.diff_manager
    }

    pub fn approval_gate(&self) -> &Arc<TerminalApprovalGate> {
        &self.approval_gate
    }

    /// Single-click rollback of the latest change to `path`.
    pub fn rollback_latest(&self, path: impl AsRef<Path>) -> Result<BackupSnapshot> {
        self.diff_manager.rollback_latest_for_path(path)
    }

    /// Executes a single multi-turn interaction initiated by the user.
    pub async fn execute_turn(
        &mut self,
        user_input: &str,
        cancel: CancellationToken,
    ) -> Result<AgentTurnResult> {
        if cancel.is_cancelled() {
            if let Some(tx) = &self.event_tx {
                let _ = tx.send(AgentEvent::TurnCancelled);
            }
            return Ok(AgentTurnResult {
                turn_count: 0,
                final_response: "Turn cancelled before start.".into(),
                executed_tools: Vec::new(),
                diffs: Vec::new(),
                completed: false,
                hit_turn_limit: false,
                cancelled: true,
            });
        }

        self.active_turn_count += 1;
        let turn_idx = self.active_turn_count;

        if let Some(tx) = &self.event_tx {
            let _ = tx.send(AgentEvent::TurnStarted {
                turn_index: turn_idx,
                max_turns: self.config.max_turns,
            });
        }

        // 1. Build composite system prompt including active skills & dynamic tools
        let skill_refs: Vec<&SkillManifest> = self.skills.iter().collect();
        let prompt_synth = AgentExecutor::build_system_prompt(
            &skill_refs,
            self.dynamic_tools.as_ref(),
            user_input,
            Some(&self.config.model_name),
        );

        self.history.insert_or_update_system(&prompt_synth.system_prompt);
        self.history.push(ChatMessage::user(user_input));

        // 2. Multi-turn execution loop
        let mut loop_count = 0;
        let mut executed_tools = Vec::new();
        let mut diffs = Vec::new();

        while loop_count < self.config.max_turns {
            loop_count += 1;

            if cancel.is_cancelled() {
                if let Some(tx) = &self.event_tx {
                    let _ = tx.send(AgentEvent::TurnCancelled);
                }
                return Ok(AgentTurnResult {
                    turn_count: loop_count,
                    final_response: "Execution cancelled by user.".into(),
                    executed_tools,
                    diffs,
                    completed: false,
                    hit_turn_limit: false,
                    cancelled: true,
                });
            }

            // Context window safeguarding
            self.history.prune_if_exceeds(self.config.max_context_tokens);

            // Call local LLM
            let model_output = match self
                .llm_client
                .chat_complete(self.history.messages(), cancel.clone())
                .await
            {
                Ok(resp) => resp,
                Err(RideError::Cancelled) => {
                    if let Some(tx) = &self.event_tx {
                        let _ = tx.send(AgentEvent::TurnCancelled);
                    }
                    return Ok(AgentTurnResult {
                        turn_count: loop_count,
                        final_response: "Execution cancelled during LLM inference.".into(),
                        executed_tools,
                        diffs,
                        completed: false,
                        hit_turn_limit: false,
                        cancelled: true,
                    });
                }
                Err(e) => {
                    if let Some(tx) = &self.event_tx {
                        let _ = tx.send(AgentEvent::TurnError {
                            error: e.to_string(),
                        });
                    }
                    return Err(e);
                }
            };

            // Parse tool calls via dual-parser (JSON + XML fallback)
            let tool_calls = AgentToolParser::parse(&model_output);

            if tool_calls.is_empty() {
                // Model reached final response
                self.history.push(ChatMessage::assistant(&model_output));

                if let Some(tx) = &self.event_tx {
                    let _ = tx.send(AgentEvent::TurnCompleted {
                        turn_index: loop_count,
                        response: model_output.clone(),
                    });
                }

                return Ok(AgentTurnResult {
                    turn_count: loop_count,
                    final_response: model_output,
                    executed_tools,
                    diffs,
                    completed: true,
                    hit_turn_limit: false,
                    cancelled: false,
                });
            }

            // Record assistant tool calls in history
            self.history
                .push(ChatMessage::assistant_with_tools(&model_output, tool_calls.clone()));

            // Execute each requested tool call
            for call in tool_calls {
                if cancel.is_cancelled() {
                    if let Some(tx) = &self.event_tx {
                        let _ = tx.send(AgentEvent::TurnCancelled);
                    }
                    return Ok(AgentTurnResult {
                        turn_count: loop_count,
                        final_response: "Execution cancelled by user.".into(),
                        executed_tools,
                        diffs,
                        completed: false,
                        hit_turn_limit: false,
                        cancelled: true,
                    });
                }

                if let Some(tx) = &self.event_tx {
                    let _ = tx.send(AgentEvent::ToolCallRequested {
                        tool_name: call.name.clone(),
                        arguments: call.arguments.clone(),
                    });
                    let _ = tx.send(AgentEvent::ToolExecutionStarted {
                        tool_name: call.name.clone(),
                    });
                }

                let outcome = AgentExecutor::execute_tool_full(
                    &call,
                    self.dynamic_tools.as_ref(),
                    self.config.working_dir.as_deref(),
                    Some(&self.approval_gate),
                    Some(&self.diff_manager),
                    &self.config.tool_budget,
                )
                .await;

                tokio::task::yield_now().await;

                if let Some(diff) = &outcome.diff {
                    if let Some(tx) = &self.event_tx {
                        let _ = tx.send(AgentEvent::DiffGenerated {
                            path: diff.path.to_string_lossy().to_string(),
                            diff: diff.diff_text.clone(),
                        });
                    }
                    diffs.push(diff.clone());
                }

                if let Some(tx) = &self.event_tx {
                    let _ = tx.send(AgentEvent::ToolExecuted {
                        tool_name: call.name.clone(),
                        success: outcome.success,
                        output_preview: outcome
                            .output
                            .lines()
                            .take(5)
                            .collect::<Vec<_>>()
                            .join("\n"),
                    });
                }

                // Add tool result to conversation history
                self.history.push(ChatMessage::tool(&call.name, &outcome.output));

                executed_tools.push(ExecutedToolCall {
                    call,
                    success: outcome.success,
                    output: outcome.output,
                    diff: outcome.diff,
                    snapshot: outcome.snapshot,
                });
            }

            // Continue loop for next turn iteration
        }

        // Maximum turns limit reached
        let limit_message = format!(
            "Maximum turn limit ({}) reached to prevent infinite loops.",
            self.config.max_turns
        );
        self.history.push(ChatMessage::assistant(&limit_message));

        if let Some(tx) = &self.event_tx {
            let _ = tx.send(AgentEvent::TurnCompleted {
                turn_index: loop_count,
                response: limit_message.clone(),
            });
        }

        Ok(AgentTurnResult {
            turn_count: loop_count,
            final_response: limit_message,
            executed_tools,
            diffs,
            completed: false,
            hit_turn_limit: true,
            cancelled: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::approval::ApprovalPolicy;

    #[tokio::test]
    async fn test_agent_session_simple_final_response() {
        let mock_client = Arc::new(MockLlmClient::new_with_queue(vec![
            "Hello! I am ready to help you code.".into(),
        ]));

        let config = AgentSessionConfig::default();
        let mut session = AgentSession::new(config, mock_client);

        let cancel = CancellationToken::new();
        let result = session.execute_turn("Hi there", cancel).await.unwrap();

        assert_eq!(result.turn_count, 1);
        assert!(result.completed);
        assert!(!result.hit_turn_limit);
        assert_eq!(result.final_response, "Hello! I am ready to help you code.");
        assert_eq!(session.history().len(), 3); // System, User, Assistant
    }

    #[tokio::test]
    async fn test_agent_session_multi_turn_tool_loop() {
        let tmp_dir = std::env::temp_dir().join("ride_session_test");
        let _ = std::fs::create_dir_all(&tmp_dir);
        let target_file = tmp_dir.join("test_file.rs");
        std::fs::write(&target_file, "fn old_func() {}\n").unwrap();

        // 1st LLM call: emits tool call to edit_file
        // 2nd LLM call: emits final answer
        let responses = vec![
            format!(
                "<tool_call name=\"edit_file\"><path>{}</path><target>old_func</target><replacement>new_func</replacement></tool_call>",
                target_file.to_string_lossy().replace('\\', "/")
            ),
            "I have updated old_func to new_func successfully.".into(),
        ];

        let mock_client = Arc::new(MockLlmClient::new_with_queue(responses));
        let config = AgentSessionConfig {
            working_dir: Some(tmp_dir.clone()),
            ..Default::default()
        };

        let mut session = AgentSession::new(config, mock_client);
        let cancel = CancellationToken::new();

        let result = session
            .execute_turn("Please rename old_func to new_func", cancel)
            .await
            .unwrap();

        assert_eq!(result.turn_count, 2);
        assert!(result.completed);
        assert_eq!(result.executed_tools.len(), 1);
        assert!(result.executed_tools[0].success);
        assert_eq!(result.diffs.len(), 1);
        assert!(result.diffs[0].diff_text.contains("+fn new_func() {}"));

        // File on disk was modified
        let updated = std::fs::read_to_string(&target_file).unwrap();
        assert!(updated.contains("new_func"));

        // Single-click rollback works
        let rolled_back = session.rollback_latest(&target_file).unwrap();
        assert_eq!(
            rolled_back.previous_content,
            Some("fn old_func() {}\n".into())
        );
        let restored = std::fs::read_to_string(&target_file).unwrap();
        assert!(restored.contains("old_func"));

        let _ = std::fs::remove_dir_all(tmp_dir);
    }

    #[tokio::test]
    async fn test_agent_session_max_turn_limit_guard() {
        // Queue an infinite loop of tool calls
        let infinite_tool = "<tool_call name=\"list_dir\"><path>.</path></tool_call>".to_string();
        let responses = vec![infinite_tool; 15];

        let mock_client = Arc::new(MockLlmClient::new_with_queue(responses));
        let config = AgentSessionConfig {
            max_turns: 3,
            ..Default::default()
        };

        let mut session = AgentSession::new(config, mock_client);
        let cancel = CancellationToken::new();

        let result = session.execute_turn("Run endlessly", cancel).await.unwrap();

        assert_eq!(result.turn_count, 3);
        assert!(!result.completed);
        assert!(result.hit_turn_limit);
        assert!(result.final_response.contains("Maximum turn limit (3) reached"));
    }

    #[tokio::test]
    async fn test_agent_session_cancellation() {
        let cancel = CancellationToken::new();
        cancel.cancel(); // Cancel immediately

        let mock_client = Arc::new(MockLlmClient::new_with_queue(vec!["ignored".into()]));
        let config = AgentSessionConfig::default();
        let mut session = AgentSession::new(config, mock_client);

        let result = session.execute_turn("Should cancel", cancel).await.unwrap();
        assert!(result.cancelled);
        assert!(!result.completed);
    }

    #[tokio::test]
    async fn test_agent_session_security_denial() {
        let destructive_call = "<tool_call name=\"run_command\"><cmd>rm -rf /</cmd></tool_call>".to_string();
        let responses = vec![
            destructive_call,
            "Command failed to run.".into(),
        ];

        let mock_client = Arc::new(MockLlmClient::new_with_queue(responses));
        let config = AgentSessionConfig {
            approval_policy: ApprovalPolicy::Deny,
            ..Default::default()
        };

        let mut session = AgentSession::new(config, mock_client);
        let cancel = CancellationToken::new();

        let result = session.execute_turn("Delete root", cancel).await.unwrap();
        assert_eq!(result.executed_tools.len(), 1);
        assert!(!result.executed_tools[0].success);
        assert!(result.executed_tools[0].output.contains("denied by Deny policy"));
    }

    #[test]
    fn test_sliding_window_prunes_conversation_history() {
        let mut history = ConversationHistory::new();
        history.insert_or_update_system("System instructions");

        // Add 20 turns of messages
        for i in 1..=20 {
            history.push(ChatMessage::user(format!("Question {i} with lots of context details... {}", "x".repeat(100))));
            history.push(ChatMessage::assistant(format!("Answer {i} with code snippets... {}", "y".repeat(100))));
        }

        let initial_tokens = history.estimate_token_count();
        assert!(initial_tokens > 500);

        // Prune with a tight 200 token budget
        history.prune_if_exceeds(200);

        let pruned_tokens = history.estimate_token_count();
        assert!(pruned_tokens <= 200 || history.messages().len() <= 2);
        // System prompt at index 0 must ALWAYS be preserved
        assert_eq!(history.messages()[0].role, ChatRole::System);
        assert_eq!(history.messages()[0].content, "System instructions");
    }

    #[tokio::test]
    async fn test_cancellation_during_multi_tool_execution() {
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();

        let (event_tx, mut event_rx) = tokio::sync::broadcast::channel(16);

        // Cancel token deterministically as soon as the first tool starts executing
        tokio::spawn(async move {
            while let Ok(evt) = event_rx.recv().await {
                if let AgentEvent::ToolExecutionStarted { tool_name } = evt {
                    if tool_name == "list_dir" {
                        cancel_clone.cancel();
                        break;
                    }
                }
            }
        });

        let tmp_dir = std::env::temp_dir().join("ride_cancel_test");
        let _ = std::fs::remove_dir_all(&tmp_dir);
        let _ = std::fs::create_dir_all(&tmp_dir);

        // Model returns two tool calls
        let responses = vec![
            "<tool_call name=\"list_dir\"><path>.</path></tool_call>\n<tool_call name=\"write_file\"><path>should_not_write.txt</path><content>data</content></tool_call>".into(),
        ];

        let mock_client = Arc::new(MockLlmClient::new_with_queue(responses));
        let config = AgentSessionConfig {
            working_dir: Some(tmp_dir.clone()),
            ..Default::default()
        };
        let mut session = AgentSession::new(config, mock_client).with_event_sender(event_tx);

        let result = session.execute_turn("List and write", cancel).await.unwrap();
        assert!(result.cancelled);
        assert_eq!(result.executed_tools.len(), 1);
        assert_eq!(result.executed_tools[0].call.name, "list_dir");
        assert!(!tmp_dir.join("should_not_write.txt").exists());

        let _ = std::fs::remove_dir_all(tmp_dir);
    }

    #[tokio::test]
    async fn test_multi_turn_conversation_context_retention() {
        let responses = vec![
            "Turn 1 response: I know about project X.".into(),
            "Turn 2 response: Yes, we were discussing project X.".into(),
        ];

        let mock_client = Arc::new(MockLlmClient::new_with_queue(responses));
        let config = AgentSessionConfig::default();
        let mut session = AgentSession::new(config, mock_client);

        let r1 = session.execute_turn("Let's talk about project X", CancellationToken::new()).await.unwrap();
        assert!(r1.completed);

        let r2 = session.execute_turn("What were we talking about?", CancellationToken::new()).await.unwrap();
        assert!(r2.completed);

        // History now has: System, User1, Assistant1, User2, Assistant2
        let history = session.history();
        assert_eq!(history.len(), 5);
        assert_eq!(history[1].role, ChatRole::User);
        assert!(history[1].content.contains("project X"));
        assert_eq!(history[3].role, ChatRole::User);
        assert!(history[3].content.contains("talking about"));
    }
}
