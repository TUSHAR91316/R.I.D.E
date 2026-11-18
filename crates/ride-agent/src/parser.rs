use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParsedToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

pub struct AgentToolParser;

impl AgentToolParser {
    /// Attempts to parse tool calls from both standard JSON function calls, markdown fences,
    /// embedded JSON, and XML fallback tags.
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

        // 2b. Embedded JSON check (e.g. conversational text followed by raw JSON object)
        let mut scan_pos = 0;
        while let Some(start) = text[scan_pos..].find('{') {
            let actual_start = scan_pos + start;
            let mut depth = 0;
            let mut end_pos = None;
            let mut in_str = false;
            let mut escape = false;

            for (offset, c) in text[actual_start..].char_indices() {
                if escape {
                    escape = false;
                    continue;
                }
                if c == '\\' && in_str {
                    escape = true;
                    continue;
                }
                if c == '"' {
                    in_str = !in_str;
                    continue;
                }
                if !in_str {
                    if c == '{' {
                        depth += 1;
                    } else if c == '}' {
                        depth -= 1;
                        if depth == 0 {
                            end_pos = Some(actual_start + offset + 1);
                            break;
                        }
                    }
                }
            }

            if let Some(end) = end_pos {
                let json_candidate = &text[actual_start..end];
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_candidate) {
                    let before_len = results.len();
                    Self::extract_from_json_value(&val, &mut results);
                    if results.len() > before_len {
                        scan_pos = end;
                        continue;
                    }
                }
                scan_pos = actual_start + 1;
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
                let is_self_closing = header.trim_end().ends_with('/');
                let mut tool_name = Self::extract_attribute(header, "name")
                    .or_else(|| Self::extract_attribute(header, "tool"))
                    .or_else(|| Self::extract_attribute(header, "function"));

                if is_self_closing {
                    if let Some(name) = tool_name {
                        results.push(ParsedToolCall {
                            name,
                            arguments: serde_json::Value::Object(serde_json::Map::new()),
                        });
                    }
                    search_idx = actual_start + tag_close + 1;
                    continue;
                }

                let body_start = actual_start + tag_close + 1;
                let next_tc_opt = text[body_start..].find("<tool_call");
                let end_tag_opt = text[body_start..].find("</tool_call>");

                match (end_tag_opt, next_tc_opt) {
                    (Some(end_tag), None) => {
                        let content = &text[body_start..body_start + end_tag];
                        if tool_name.is_none() {
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
                    (Some(end_tag), Some(next_tc)) if end_tag < next_tc => {
                        let content = &text[body_start..body_start + end_tag];
                        if tool_name.is_none() {
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
                    (_, Some(next_tc)) => {
                        let content = &text[body_start..body_start + next_tc];
                        if let Some(name) = tool_name.or_else(|| Self::extract_inner_tag(content, "name")) {
                            let args = Self::parse_tool_body(content);
                            if args.as_object().map_or(false, |m| !m.is_empty()) {
                                results.push(ParsedToolCall {
                                    name,
                                    arguments: args,
                                });
                            }
                        }
                        search_idx = body_start + next_tc;
                        continue;
                    }
                    (None, None) => {
                        let content = &text[body_start..];
                        if let Some(name) = tool_name.or_else(|| Self::extract_inner_tag(content, "name")) {
                            let args = Self::parse_tool_body(content);
                            if args.as_object().map_or(false, |m| !m.is_empty()) {
                                results.push(ParsedToolCall {
                                    name,
                                    arguments: args,
                                });
                            }
                        }
                        break;
                    }
                }
            }
            search_idx = actual_start + 10;
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
                    let body = &text[body_start..body_start + end];
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
                            .unwrap_or_else(|_| serde_json::Value::String(s.clone()))
                    }
                    Some(other) => other.clone(),
                    None => serde_json::Value::Object(serde_json::Map::new()),
                };
                results.push(ParsedToolCall {
                    name: name.to_string(),
                    arguments: args,
                });
            }
        } else if let Some(name) = val
            .get("name")
            .or_else(|| val.get("tool"))
            .or_else(|| val.get("tool_name"))
            .and_then(|n| n.as_str())
        {
            let raw_args = val
                .get("arguments")
                .or_else(|| val.get("parameters"))
                .or_else(|| val.get("args"));

            let args = match raw_args {
                Some(serde_json::Value::String(s)) => {
                    serde_json::from_str::<serde_json::Value>(s)
                        .unwrap_or_else(|_| serde_json::Value::String(s.clone()))
                }
                Some(other) => other.clone(),
                None => serde_json::Value::Object(serde_json::Map::new()),
            };
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

    fn strip_tag_wrapping_newlines(s: &str) -> &str {
        let mut res = s;
        if res.starts_with("\r\n") {
            res = &res[2..];
        } else if res.starts_with('\n') {
            res = &res[1..];
        }
        if let Some(last_nl) = res.rfind('\n') {
            if res[last_nl + 1..].chars().all(|c| c == ' ' || c == '\t' || c == '\r') {
                let end = if last_nl > 0 && res.as_bytes()[last_nl - 1] == b'\r' {
                    last_nl - 1
                } else {
                    last_nl
                };
                res = &res[..end];
            }
        } else if res.ends_with("\r\n") {
            res = &res[..res.len() - 2];
        } else if res.ends_with('\n') {
            res = &res[..res.len() - 1];
        }
        res
    }

    fn parse_tool_body(body: &str) -> serde_json::Value {
        // Try parsing body as json first
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(body.trim()) {
            return val;
        }

        // Otherwise parse simple key tags: <param>value</param>
        let mut map = serde_json::Map::new();
        let mut idx = 0;
        while let Some(open) = body[idx..].find('<') {
            let tag_start = idx + open;
            if let Some(close) = body[tag_start..].find('>') {
                let tag_raw = &body[tag_start + 1..tag_start + close];
                if tag_raw.starts_with('/') {
                    idx = tag_start + close + 1;
                    continue;
                }
                let clean_tag_name = tag_raw
                    .split_whitespace()
                    .next()
                    .unwrap_or(tag_raw)
                    .trim_end_matches('/');
                if clean_tag_name.is_empty() {
                    idx = tag_start + close + 1;
                    continue;
                }
                let end_tag = format!("</{clean_tag_name}>");
                let val_start = tag_start + close + 1;
                if let Some(end_pos) = body[val_start..].find(&end_tag) {
                    let raw_val = &body[val_start..val_start + end_pos];
                    match clean_tag_name {
                        "target" | "replacement" | "content" => {
                            // Preserve indentation, only strip single wrapping newline if on its own line
                            let clean = Self::strip_tag_wrapping_newlines(raw_val).to_string();
                            map.insert(clean_tag_name.to_string(), serde_json::Value::String(clean));
                        }
                        _ => {
                            let trimmed = raw_val.trim();
                            if trimmed.eq_ignore_ascii_case("true") {
                                map.insert(clean_tag_name.to_string(), serde_json::Value::Bool(true));
                            } else if trimmed.eq_ignore_ascii_case("false") {
                                map.insert(clean_tag_name.to_string(), serde_json::Value::Bool(false));
                            } else if let Ok(num) = trimmed.parse::<i64>() {
                                map.insert(clean_tag_name.to_string(), serde_json::Value::Number(num.into()));
                            } else {
                                map.insert(clean_tag_name.to_string(), serde_json::Value::String(trimmed.to_string()));
                            }
                        }
                    }
                    idx = val_start + end_pos + end_tag.len();
                    continue;
                } else {
                    idx = tag_start + 1;
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

    #[test]
    fn test_embedded_json_in_conversational_text() {
        let input = "Sure! I will read that file for you:\n{\"name\": \"read_file\", \"arguments\": {\"path\": \"src/main.rs\"}}\nLet me know what else you need.";
        let calls = AgentToolParser::parse(input);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "read_file");
        assert_eq!(calls[0].arguments["path"], "src/main.rs");
    }

    #[test]
    fn test_json_tool_and_parameters_syntax() {
        let input = r#"{"tool": "read_file", "parameters": {"path": "src/lib.rs"}}"#;
        let calls = AgentToolParser::parse(input);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "read_file");
        assert_eq!(calls[0].arguments["path"], "src/lib.rs");
    }

    #[test]
    fn test_xml_types_and_indentation_preservation() {
        let input = r#"
<tool_call name="read_file">
  <path>src/main.rs</path>
  <start_line>50</start_line>
  <end_line>100</end_line>
</tool_call>
"#;
        let calls = AgentToolParser::parse(input);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "read_file");
        assert_eq!(calls[0].arguments["start_line"], 50);
        assert_eq!(calls[0].arguments["end_line"], 100);

        // Multiline code with indentation
        let edit_input = r#"
<tool_call name="edit_file">
  <path>src/main.rs</path>
  <target>
    let x = 1;
    let y = 2;
  </target>
  <replacement>
    let x = 10;
    let y = 20;
  </replacement>
</tool_call>
"#;
        let edit_calls = AgentToolParser::parse(edit_input);
        assert_eq!(edit_calls.len(), 1);
        assert_eq!(
            edit_calls[0].arguments["target"],
            "    let x = 1;\n    let y = 2;"
        );
        assert_eq!(
            edit_calls[0].arguments["replacement"],
            "    let x = 10;\n    let y = 20;"
        );
    }

    #[test]
    fn test_xml_self_closing_and_recovery() {
        let input = "<tool_call name=\"list_dir\" />";
        let calls = AgentToolParser::parse(input);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "list_dir");

        // Malformed/unclosed XML tag followed by valid tag
        let input_recovery = "<tool_call name=\"broken\"><path>missing_end\n<tool_call name=\"read_file\"><path>good.rs</path></tool_call>";
        let recovery_calls = AgentToolParser::parse(input_recovery);
        assert!(recovery_calls.iter().any(|c| c.name == "read_file"));
    }
}
