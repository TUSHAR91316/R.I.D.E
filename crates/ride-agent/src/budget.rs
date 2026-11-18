use serde::{Deserialize, Serialize};

/// Context budgeting configuration for agent tool outputs to prevent blowing local LLM context windows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutputBudget {
    /// Maximum character length for any tool output (default 4,000)
    pub max_chars: usize,
    /// Maximum line count for tool output (default 200)
    pub max_lines: usize,
    /// Maximum search results to return in `search_code` (default 50)
    pub max_search_matches: usize,
    /// Maximum directory entries in `list_dir` (default 100)
    pub max_dir_entries: usize,
    /// Maximum lines to return in `read_file` when unbounded (default 300)
    pub max_file_read_lines: usize,
}

impl Default for ToolOutputBudget {
    fn default() -> Self {
        Self {
            max_chars: 4000,
            max_lines: 200,
            max_search_matches: 50,
            max_dir_entries: 100,
            max_file_read_lines: 300,
        }
    }
}

/// Hardened output truncation logic.
pub struct ToolOutputBudgeter;

impl ToolOutputBudgeter {
    /// Truncates string output according to budget rules and appends clear warning text.
    pub fn truncate(output: &str, budget: &ToolOutputBudget) -> String {
        Self::truncate_explicit(output, budget.max_lines, budget.max_chars)
    }

    /// Truncates output based on explicit line and character limits.
    pub fn truncate_explicit(output: &str, max_lines: usize, max_chars: usize) -> String {
        let total_chars = output.chars().count();
        let lines: Vec<&str> = output.lines().collect();
        let total_lines = lines.len();

        let exceeds_lines = total_lines > max_lines;
        let exceeds_chars = total_chars > max_chars;

        if !exceeds_lines && !exceeds_chars {
            return output.to_string();
        }

        // Take up to max_lines lines
        let candidate_lines = if exceeds_lines {
            &lines[..max_lines]
        } else {
            &lines[..]
        };

        let mut truncated_text = candidate_lines.join("\n");

        // Further check character limit
        if truncated_text.chars().count() > max_chars {
            // Cut safely at char boundary
            let safe_slice: String = truncated_text.chars().take(max_chars).collect();
            truncated_text = safe_slice;
        }

        let shown_chars = truncated_text.chars().count();
        let shown_lines = truncated_text.lines().count();

        format!(
            "{truncated_text}\n\n[Output truncated: showing {shown_lines} of {total_lines} lines ({shown_chars} of {total_chars} chars). Specify narrower line numbers or search filters to see more.]"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_truncation_under_limits() {
        let text = "Line 1\nLine 2\nLine 3\n";
        let budget = ToolOutputBudget::default();
        let res = ToolOutputBudgeter::truncate(text, &budget);
        assert_eq!(res, text);
    }

    #[test]
    fn test_truncate_line_limit() {
        let lines: Vec<String> = (1..=300).map(|i| format!("Line {i}")).collect();
        let text = lines.join("\n");

        let mut budget = ToolOutputBudget::default();
        budget.max_lines = 10;
        budget.max_chars = 50000;

        let res = ToolOutputBudgeter::truncate(&text, &budget);
        assert!(res.contains("Line 10"));
        assert!(!res.contains("Line 11\n"));
        assert!(res.contains("[Output truncated: showing 10 of 300 lines"));
    }

    #[test]
    fn test_truncate_char_limit_single_long_line() {
        let long_line: String = "a".repeat(1000);
        let mut budget = ToolOutputBudget::default();
        budget.max_chars = 100;
        budget.max_lines = 10;

        let res = ToolOutputBudgeter::truncate(&long_line, &budget);
        assert!(res.contains("[Output truncated: showing 1 of 1 lines (100 of 1000 chars)"));
        assert!(res.starts_with(&"a".repeat(100)));
    }
}
