use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParsedToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

pub struct AgentToolParser;

impl AgentToolParser {
    /// Attempts to parse tool calls from both standard JSON function calls, markdown fences,
    /// and XML fallback tags.
    pub fn parse(text: &str) -> Vec<ParsedToolCall> {
        let trimmed = text.trim();
        let mut results = Vec::new();

        // 1. Direct JSON check (raw text is JSON)
        if (trimmed.starts_with('{') && trimmed.ends_with('}'))
            || (trimmed.starts_with('[') && trimmed.ends_with(']'))
        {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
                Self::extract_from_json_value(&val, &mut results);
                if !results.is_empty() {
                    return results;
                }
            }
        }

        // 2. Markdown fenced code blocks: ```json ... ``` or ``` ... ```
        let mut fence_idx = 0;
        while let Some(start_fence) = text[fence_idx..].find("```") {
            let actual_start = fence_idx + start_fence + 3;
            let block_after = &text[actual_start..];
            // Skip optional language identifier like "json"
            let content_start = if let Some(first_nl) = block_after.find('\n') {
                actual_start + first_nl + 1
            } else {
                actual_start
            };

            if let Some(end_fence) = text[content_start..].find("```") {
                let code_content = text[content_start..content_start + end_fence].trim();
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(code_content) {
                    Self::extract_from_json_value(&val, &mut results);
                }
                fence_idx = content_start + end_fence + 3;
            } else {
                break;
            }
        }

        if !results.is_empty() {
            return results;
        }

        // 3. XML fallback: <tool_call ...>...</tool_call>
        let mut search_idx = 0;
        while let Some(start_tag) = text[search_idx..].find("<tool_call") {
            let actual_start = search_idx + start_tag;
            if let Some(tag_close) = text[actual_start..].find('>') {
                let header = &text[actual_start..actual_start + tag_close];
                let mut tool_name = Self::extract_attribute(header, "name")
                    .or_else(|| Self::extract_attribute(header, "tool"))
                    .or_else(|| Self::extract_attribute(header, "function"));

                let body_start = actual_start + tag_close + 1;
                if let Some(end_tag) = text[body_start..].find("</tool_call>") {
                    let content = text[body_start..body_start + end_tag].trim();

                    if tool_name.is_none() {
                        // Check if body has <name>tool</name>
                        tool_name = Self::extract_inner_tag(content, "name");
                    }

                    let name = tool_name.unwrap_or_else(|| "unknown".into());
                    let args = Self::parse_tool_body(content);
                    results.push(ParsedToolCall {
                        name,
                        arguments: args,
                    });
                    search_idx = body_start + end_tag + 12;
                    continue;
                }
            }
            break;
        }

        if !results.is_empty() {
            return results;
        }

        // 4. Direct tool tags: e.g. <edit_file>...</edit_file>, <read_file>...</read_file>
        let known_tools = [
            "read_file",
            "write_file",
            "edit_file",
            "list_dir",
            "search_code",
            "run_command",
        ];
        for tool in known_tools {
            let open_tag = format!("<{tool}>");
            let close_tag = format!("</{tool}>");
            let mut search_pos = 0;
            while let Some(start) = text[search_pos..].find(&open_tag) {
                let body_start = search_pos + start + open_tag.len();
                if let Some(end) = text[body_start..].find(&close_tag) {
                    let body = text[body_start..body_start + end].trim();
                    let args = Self::parse_tool_body(body);
                    results.push(ParsedToolCall {
                        name: tool.to_string(),
                        arguments: args,
                    });
                    search_pos = body_start + end + close_tag.len();
                } else {
                    break;
                }
            }
        }

        results
    }

    fn extract_from_json_value(val: &serde_json::Value, results: &mut Vec<ParsedToolCall>) {
        if let Some(arr) = val.as_array() {
            for item in arr {
                Self::extract_single_json_call(item, results);
            }
        } else if let Some(obj) = val.as_object() {
            if let Some(calls) = obj.get("tool_calls").and_then(|v| v.as_array()) {
                for item in calls {
                    Self::extract_single_json_call(item, results);
                }
            } else {
                Self::extract_single_json_call(val, results);
            }
        }
    }

    fn extract_single_json_call(val: &serde_json::Value, results: &mut Vec<ParsedToolCall>) {
        if let Some(func) = val.get("function") {
            // OpenAI format: { "function": { "name": "...", "arguments": "..." } }
            if let Some(name) = func.get("name").and_then(|n| n.as_str()) {
                let args = match func.get("arguments") {
                    Some(serde_json::Value::String(s)) => {
                        serde_json::from_str::<serde_json::Value>(s)
                            .unwrap_or(serde_json::Value::String(s.clone()))
                    }
                    Some(other) => other.clone(),
                    None => serde_json::Value::Object(serde_json::Map::new()),
                };
                results.push(ParsedToolCall {
                    name: name.to_string(),
                    arguments: args,
                });
            }
        } else if let Some(name) = val.get("name").and_then(|n| n.as_str()) {
            let args = val.get("arguments").cloned().unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
            results.push(ParsedToolCall {
                name: name.to_string(),
                arguments: args,
            });
        }
    }

    fn extract_attribute(header: &str, attr: &str) -> Option<String> {
        for quote in ['"', '\''] {
            let pattern = format!("{attr}={quote}");
            let pattern_spaced = format!("{attr} = {quote}");
            for pat in [&pattern, &pattern_spaced] {
                if let Some(pos) = header.find(pat.as_str()) {
                    let after = &header[pos + pat.len()..];
                    if let Some(end) = after.find(quote) {
                        return Some(after[..end].to_string());
                    }
                }
            }
        }
        None
    }

    fn extract_inner_tag(body: &str, tag: &str) -> Option<String> {
        let open = format!("<{tag}>");
        let close = format!("</{tag}>");
        if let Some(start) = body.find(&open) {
            let after = &body[start + open.len()..];
            if let Some(end) = after.find(&close) {
                return Some(after[..end].trim().to_string());
            }
        }
        None
    }

    fn parse_tool_body(body: &str) -> serde_json::Value {
        // Try parsing body as json first
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(body) {
            return val;
        }

        // Otherwise parse simple key tags: <param>value</param>
        let mut map = serde_json::Map::new();
        let mut idx = 0;
        while let Some(open) = body[idx..].find('<') {
            let tag_start = idx + open;
            if let Some(close) = body[tag_start..].find('>') {
                let tag_name = &body[tag_start + 1..tag_start + close];
                if tag_name.starts_with('/') {
                    idx = tag_start + close + 1;
                    continue;
                }
                let end_tag = format!("</{tag_name}>");
                let val_start = tag_start + close + 1;
                if let Some(end_pos) = body[val_start..].find(&end_tag) {
                    let val = &body[val_start..val_start + end_pos].trim();
                    map.insert((*tag_name).to_string(), serde_json::Value::String((*val).to_string()));
                    idx = val_start + end_pos + end_tag.len();
                    continue;
                }
            }
            break;
        }

        serde_json::Value::Object(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xml_tool_call_parsing() {
        let input = r#"
I will now edit the file:
<tool_call name="edit_file">
  <path>src/main.rs</path>
  <target>fn old()</target>
  <replacement>fn new()</replacement>
</tool_call>
"#;
        let calls = AgentToolParser::parse(input);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "edit_file");
        assert_eq!(calls[0].arguments["path"], "src/main.rs");
        assert_eq!(calls[0].arguments["target"], "fn old()");
        assert_eq!(calls[0].arguments["replacement"], "fn new()");
    }

    #[test]
    fn test_xml_single_quote_and_inner_name() {
        let input = "<tool_call name='read_file'><path>src/lib.rs</path></tool_call>";
        let calls = AgentToolParser::parse(input);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "read_file");
        assert_eq!(calls[0].arguments["path"], "src/lib.rs");

        let input_inner = "<tool_call><name>list_dir</name><path>.</path></tool_call>";
        let calls_inner = AgentToolParser::parse(input_inner);
        assert_eq!(calls_inner.len(), 1);
        assert_eq!(calls_inner[0].name, "list_dir");
    }

    #[test]
    fn test_direct_tool_tag_parsing() {
        let input = "Let me list the files:\n<list_dir><path>src</path></list_dir>";
        let calls = AgentToolParser::parse(input);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "list_dir");
        assert_eq!(calls[0].arguments["path"], "src");
    }

    #[test]
    fn test_raw_json_tool_call() {
        let input = r#"{"name": "read_file", "arguments": {"path": "Cargo.toml"}}"#;
        let calls = AgentToolParser::parse(input);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "read_file");
        assert_eq!(calls[0].arguments["path"], "Cargo.toml");
    }

    #[test]
    fn test_markdown_fence_json_tool_call() {
        let input = r#"
Here is the tool call:
```json
{
  "name": "write_file",
  "arguments": {
    "path": "hello.txt",
    "content": "world"
  }
}
```
"#;
        let calls = AgentToolParser::parse(input);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "write_file");
        assert_eq!(calls[0].arguments["path"], "hello.txt");
    }

    #[test]
    fn test_openai_tool_calls_format() {
        let input = r#"{
            "tool_calls": [
                {
                    "id": "call_abc",
                    "type": "function",
                    "function": {
                        "name": "edit_file",
                        "arguments": "{\"path\":\"a.txt\",\"target\":\"x\",\"replacement\":\"y\"}"
                    }
                }
            ]
        }"#;
        let calls = AgentToolParser::parse(input);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "edit_file");
        assert_eq!(calls[0].arguments["path"], "a.txt");
        assert_eq!(calls[0].arguments["target"], "x");
        assert_eq!(calls[0].arguments["replacement"], "y");
    }
}
