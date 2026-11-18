use ride_core::error::{Result, RideError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DynamicToolDefinition {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_object_schema")]
    pub parameters: serde_json::Value,
    pub script: String,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_object_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {}
    })
}

fn default_timeout_ms() -> u64 {
    30_000
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DynamicToolOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone)]
pub struct DynamicSkillTool {
    pub definition: DynamicToolDefinition,
    pub skill_dir: PathBuf,
}

impl DynamicSkillTool {
    pub fn new(definition: DynamicToolDefinition, skill_dir: impl AsRef<Path>) -> Self {
        Self {
            definition,
            skill_dir: skill_dir.as_ref().to_path_buf(),
        }
    }

    /// Resolve the script path safely within skill_dir, protecting against path traversal.
    pub fn resolve_script_path(&self) -> Result<PathBuf> {
        let script_rel = Path::new(&self.definition.script);

        // Security check: reject explicit directory traversal sequences
        for comp in script_rel.components() {
            if comp == std::path::Component::ParentDir {
                return Err(RideError::Skill(format!(
                    "Path traversal detected in tool script: {}",
                    self.definition.script
                )));
            }
        }

        // Try candidate paths:
        // 1. skill_dir / script
        // 2. skill_dir / scripts / script
        let candidates = [
            self.skill_dir.join(script_rel),
            self.skill_dir.join("scripts").join(script_rel),
        ];

        for cand in &candidates {
            if cand.exists() && cand.is_file() {
                // Canonicalize if possible to prevent symlink escape
                if let (Ok(can_cand), Ok(can_dir)) = (cand.canonicalize(), self.skill_dir.canonicalize()) {
                    if !can_cand.starts_with(&can_dir) {
                        return Err(RideError::Skill(format!(
                            "Security error: Script escapes skill directory: {}",
                            cand.display()
                        )));
                    }
                    return Ok(strip_unc_prefix(&can_cand));
                }
                return Ok(strip_unc_prefix(cand));
            }
        }

        Err(RideError::Skill(format!(
            "Script '{}' for tool '{}' not found in '{}'",
            self.definition.script,
            self.definition.name,
            self.skill_dir.display()
        )))
    }

    /// Execute the tool script synchronously with parameter serialization
    pub fn execute_sync(
        &self,
        args: &serde_json::Value,
        working_dir: Option<&Path>,
    ) -> Result<DynamicToolOutput> {
        let script_path = self.resolve_script_path()?;
        let ext = script_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let json_str = serde_json::to_string(args).unwrap_or_else(|_| "{}".to_string());

        let mut cmd = Self::build_command(&ext, &script_path, &json_str, args)?;

        let run_dir = working_dir.unwrap_or(&self.skill_dir);
        let run_dir_clean = strip_unc_prefix(run_dir);
        cmd.current_dir(&run_dir_clean);
        cmd.env("RIDE_TOOL_PARAMS", &json_str);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| {
            RideError::Skill(format!(
                "Failed to spawn tool script {}: {e}",
                script_path.display()
            ))
        })?;

        // Write JSON string to stdin if available
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            let _ = stdin.write_all(json_str.as_bytes());
        }

        // Wait with timeout
        let timeout = Duration::from_millis(self.definition.timeout_ms);
        let start = std::time::Instant::now();

        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let mut stdout = Vec::new();
                    let mut stderr = Vec::new();
                    if let Some(mut out) = child.stdout.take() {
                        use std::io::Read;
                        let _ = out.read_to_end(&mut stdout);
                    }
                    if let Some(mut err) = child.stderr.take() {
                        use std::io::Read;
                        let _ = err.read_to_end(&mut stderr);
                    }

                    // Cap output size at 512KB to protect RAM
                    const MAX_BYTES: usize = 512 * 1024;
                    let stdout_truncated = if stdout.len() > MAX_BYTES {
                        stdout.truncate(MAX_BYTES);
                        true
                    } else {
                        false
                    };
                    let stderr_truncated = if stderr.len() > MAX_BYTES {
                        stderr.truncate(MAX_BYTES);
                        true
                    } else {
                        false
                    };

                    let mut stdout_str = String::from_utf8_lossy(&stdout).to_string();
                    let mut stderr_str = String::from_utf8_lossy(&stderr).to_string();

                    if stdout_truncated {
                        stdout_str.push_str("\n... [stdout truncated at 512KB]");
                    }
                    if stderr_truncated {
                        stderr_str.push_str("\n... [stderr truncated at 512KB]");
                    }

                    return Ok(DynamicToolOutput {
                        exit_code: status.code().unwrap_or(-1),
                        stdout: stdout_str,
                        stderr: stderr_str,
                    });
                }
                Ok(None) => {
                    if start.elapsed() > timeout {
                        let _ = child.kill();
                        return Err(RideError::Skill(format!(
                            "Tool '{}' timed out after {}ms",
                            self.definition.name, self.definition.timeout_ms
                        )));
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(e) => {
                    return Err(RideError::Skill(format!(
                        "Error waiting for tool '{}': {e}",
                        self.definition.name
                    )));
                }
            }
        }
    }

    /// Execute the tool script asynchronously
    pub async fn execute_async(
        &self,
        args: &serde_json::Value,
        working_dir: Option<PathBuf>,
    ) -> Result<DynamicToolOutput> {
        let tool = self.clone();
        let args_owned = args.clone();
        tokio::task::spawn_blocking(move || tool.execute_sync(&args_owned, working_dir.as_deref()))
            .await
            .map_err(|e| RideError::Skill(format!("Async tool task join error: {e}")))?
    }

    fn build_command(
        ext: &str,
        script_path: &Path,
        json_str: &str,
        args: &serde_json::Value,
    ) -> Result<std::process::Command> {
        let mut cmd = match ext {
            "bat" | "cmd" => {
                let mut c = std::process::Command::new("cmd.exe");
                c.arg("/C").arg(script_path);
                c
            }
            "ps1" => {
                let mut c = std::process::Command::new("powershell.exe");
                c.args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-File",
                ])
                .arg(script_path);
                c
            }
            "py" => {
                let py_bin = if cfg!(target_os = "windows") {
                    "python"
                } else {
                    "python3"
                };
                let mut c = std::process::Command::new(py_bin);
                c.arg(script_path);
                c
            }
            "sh" | "bash" => {
                let mut c = std::process::Command::new("sh");
                c.arg(script_path);
                c
            }
            "js" | "mjs" => {
                let mut c = std::process::Command::new("node");
                c.arg(script_path);
                c
            }
            _ => {
                // Executable binary (.exe or unix binary)
                std::process::Command::new(script_path)
            }
        };

        // Pass JSON string via command line argument
        cmd.arg("--json-args").arg(json_str);

        // Also pass individual named arguments for convenient script access
        if let Some(obj) = args.as_object() {
            for (key, val) in obj {
                let flag = format!("--{}", key);
                match val {
                    serde_json::Value::String(s) => {
                        cmd.arg(flag).arg(s);
                    }
                    serde_json::Value::Number(n) => {
                        cmd.arg(flag).arg(n.to_string());
                    }
                    serde_json::Value::Bool(b) => {
                        cmd.arg(flag).arg(b.to_string());
                    }
                    serde_json::Value::Null => {}
                    _ => {
                        if let Ok(serialized) = serde_json::to_string(val) {
                            cmd.arg(flag).arg(serialized);
                        }
                    }
                }
            }
        }

        Ok(cmd)
    }
}

fn strip_unc_prefix(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(stripped) = s.strip_prefix(r"\\?\") {
        PathBuf::from(stripped)
    } else {
        path.to_path_buf()
    }
}

/// Registry of dynamic tools made available across loaded skills
#[derive(Debug, Default, Clone)]
pub struct DynamicToolRegistry {
    tools: HashMap<String, DynamicSkillTool>,
}

impl DynamicToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: DynamicSkillTool) {
        self.tools.insert(tool.definition.name.clone(), tool);
    }

    pub fn get(&self, name: &str) -> Option<&DynamicSkillTool> {
        self.tools.get(name)
    }

    pub fn list(&self) -> Vec<&DynamicSkillTool> {
        self.tools.values().collect()
    }

    pub fn definitions(&self) -> Vec<&DynamicToolDefinition> {
        self.tools.values().map(|t| &t.definition).collect()
    }

    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Converts all registered tools into OpenAI-compatible tool specifications
    pub fn to_openai_tools(&self) -> serde_json::Value {
        let tools: Vec<serde_json::Value> = self
            .tools
            .values()
            .map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.definition.name,
                        "description": t.definition.description,
                        "parameters": t.definition.parameters,
                    }
                })
            })
            .collect();

        serde_json::Value::Array(tools)
    }

    /// Generates markdown XML schema documentation for local models
    pub fn to_xml_schemas(&self) -> String {
        if self.tools.is_empty() {
            return String::new();
        }

        let mut out = String::from("\n## Custom Skill Tools\n");
        for tool in self.tools.values() {
            out.push_str(&format!(
                "- `{}`: {}\n  Parameters: {}\n",
                tool.definition.name,
                tool.definition.description,
                serde_json::to_string(&tool.definition.parameters).unwrap_or_default()
            ));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_tool_definition_defaults() {
        let def: DynamicToolDefinition = serde_json::from_str(
            r#"{
                "name": "lint",
                "description": "Run linter",
                "script": "lint.py"
            }"#,
        )
        .unwrap();

        assert_eq!(def.name, "lint");
        assert_eq!(def.timeout_ms, 30_000);
        assert!(def.parameters.is_object());
    }

    #[test]
    fn test_path_traversal_protection() {
        let def = DynamicToolDefinition {
            name: "malicious".into(),
            description: "Try to escape".into(),
            parameters: serde_json::json!({}),
            script: "../../windows/system32/cmd.exe".into(),
            timeout_ms: 1000,
        };

        let tool = DynamicSkillTool::new(def, std::env::temp_dir());
        let res = tool.resolve_script_path();
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("Path traversal"));
    }

    #[test]
    fn test_missing_script_error() {
        let def = DynamicToolDefinition {
            name: "missing".into(),
            description: "Script does not exist".into(),
            parameters: serde_json::json!({}),
            script: "nonexistent_script_xyz.bat".into(),
            timeout_ms: 1000,
        };

        let tool = DynamicSkillTool::new(def, std::env::temp_dir());
        let res = tool.resolve_script_path();
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_dynamic_tool_execution_batch() {
        let temp_dir = std::env::temp_dir().join("ride_test_dynamic_tool");
        let _ = std::fs::create_dir_all(&temp_dir);
        let script_file = temp_dir.join("test_tool.bat");

        if cfg!(target_os = "windows") {
            // Write a simple bat file that prints parameters and succeeds
            std::fs::write(&script_file, "@echo off\r\necho TEST_PARAM=%RIDE_TOOL_PARAMS%\r\n").unwrap();

            let def = DynamicToolDefinition {
                name: "test_bat".into(),
                description: "Test batch tool".into(),
                parameters: serde_json::json!({"type": "object", "properties": {"msg": {"type": "string"}}}),
                script: "test_tool.bat".into(),
                timeout_ms: 5000,
            };

            let tool = DynamicSkillTool::new(def, &temp_dir);
            let args = serde_json::json!({ "msg": "hello_ride" });
            let output = tool.execute_sync(&args, None).unwrap();

            assert_eq!(output.exit_code, 0);
            assert!(output.stdout.contains("hello_ride"));
        }

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_registry_openai_and_xml_generation() {
        let mut registry = DynamicToolRegistry::new();
        let def = DynamicToolDefinition {
            name: "calc".into(),
            description: "Math calculator".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "expr": { "type": "string" }
                },
                "required": ["expr"]
            }),
            script: "calc.py".into(),
            timeout_ms: 5000,
        };

        registry.register(DynamicSkillTool::new(def, PathBuf::from(".")));
        assert_eq!(registry.len(), 1);

        let openai_tools = registry.to_openai_tools();
        assert!(openai_tools.is_array());
        let arr = openai_tools.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["function"]["name"], "calc");

        let xml = registry.to_xml_schemas();
        assert!(xml.contains("calc"));
        assert!(xml.contains("Math calculator"));
    }
}
