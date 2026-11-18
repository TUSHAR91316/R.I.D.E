pub mod approval;
pub mod budget;
pub mod diff;
pub mod executor;
pub mod parser;
pub mod session;
pub mod tools;

pub use approval::{
    ApprovalDecision, ApprovalPolicy, ApprovalRequest, CommandSafetyClassifier,
    CommandSafetyLevel, TerminalApprovalGate,
};
pub use budget::{ToolOutputBudget, ToolOutputBudgeter};
pub use diff::{BackupSnapshot, DiffEngine, DiffHunk, DiffManager, DiffPreview, UnifiedDiff};
pub use executor::{AgentExecutor, ToolExecutionOutcome};
pub use parser::{AgentToolParser, ParsedToolCall};
pub use session::{
    AgentSession, AgentSessionConfig, AgentTurnResult, ChatMessage, ChatRole,
    ConversationHistory, ExecutedToolCall, HttpLlmClient, LlmClient, MockLlmClient,
};
pub use tools::{CommandOutput, SearchMatch, ToolResult, Toolset};
