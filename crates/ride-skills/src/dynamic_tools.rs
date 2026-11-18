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

        // Security check 1: reject absolute paths, root prefixes, and drive letters
        if script_rel.is_absolute() || script_rel.has_root() {
            return Err(RideError::Skill(format!(
                "Absolute paths not permitted in tool script: {}",
                self.definition.script
            )));
        }

        // Security check 2: reject explicit directory traversal and prefix sequences
        for comp in script_rel.components() {
            if comp == std::path::Component::ParentDir || matches!(comp, std::path::Component::Prefix(_)) {
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

        // Write JSON string to stdin and close pipe so child does not block on stdin
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            let _ = stdin.write_all(json_str.as_bytes());
            let _ = stdin.flush();
        }

        // Drain stdout and stderr in background reader threads to avoid OS pipe buffer exhaustion deadlock
        const MAX_BYTES: usize = 512 * 1024;
        let stdout_pipe = child.stdout.take();
        let stderr_pipe = child.stderr.take();

        let stdout_thread = std::thread::spawn(move || {
            let mut data = Vec::new();
            let mut truncated = false;
            if let Some(mut pipe) = stdout_pipe {
                use std::io::Read;
                let mut chunk = [0u8; 8192];
                loop {
                    match pipe.read(&mut chunk) {
                        Ok(0) => break,
                        Ok(n) => {
                            if data.len() < MAX_BYTES {
                                let to_copy = n.min(MAX_BYTES - data.len());
                                data.extend_from_slice(&chunk[..to_copy]);
                                if to_copy < n {
                                    truncated = true;
                                }
                            } else {
                                truncated = true;
                            }
                        }
                        Err(_) => break,
                    }
                }
            }
            (data, truncated)
        });

        let stderr_thread = std::thread::spawn(move || {
            let mut data = Vec::new();
            let mut truncated = false;
            if let Some(mut pipe) = stderr_pipe {
                use std::io::Read;
                let mut chunk = [0u8; 8192];
                loop {
                    match pipe.read(&mut chunk) {
                        Ok(0) => break,
                        Ok(n) => {
                            if data.len() < MAX_BYTES {
                                let to_copy = n.min(MAX_BYTES - data.len());
                                data.extend_from_slice(&chunk[..to_copy]);
                                if to_copy < n {
                                    truncated = true;
                                }
                            } else {
                                truncated = true;
                            }
                        }
                        Err(_) => break,
                    }
                }
            }
            (data, truncated)
        });

        // Wait with timeout
        let timeout = Duration::from_millis(self.definition.timeout_ms);
        let start = std::time::Instant::now();

        let exit_code = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status.code().unwrap_or(-1),
                Ok(None) => {
                    if start.elapsed() > timeout {
                        let _ = child.kill();
                        let _ = child.wait();
                        let _ = stdout_thread.join();
                        let _ = stderr_thread.join();
                        return Err(RideError::Skill(format!(
                            "Tool '{}' timed out after {}ms",
                            self.definition.name, self.definition.timeout_ms
                        )));
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(e) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = stdout_thread.join();
                    let _ = stderr_thread.join();
                    return Err(RideError::Skill(format!(
                        "Error waiting for tool '{}': {e}",
                        self.definition.name
                    )));
                }
            }
        };

        let (stdout_raw, stdout_truncated) = stdout_thread.join().unwrap_or_default();
        let (stderr_raw, stderr_truncated) = stderr_thread.join().unwrap_or_default();

        let mut stdout_str = String::from_utf8_lossy(&stdout_raw).to_string();
        let mut stderr_str = String::from_utf8_lossy(&stderr_raw).to_string();

        if stdout_truncated {
            stdout_str.push_str("\n... [stdout truncated at 512KB]");
        }
        if stderr_truncated {
            stderr_str.push_str("\n... [stderr truncated at 512KB]");
        }

        Ok(DynamicToolOutput {
            exit_code,
            stdout: stdout_str,
            stderr: stderr_str,
        })
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
                if cfg!(target_os = "windows") {
                    let mut c = std::process::Command::new("cmd.exe");
                    c.arg("/C").arg(script_path);
                    c
                } else {
                    let mut c = std::process::Command::new("sh");
                    c.arg(script_path);
                    c
                }
            }
            "ps1" => {
                let ps_bin = if cfg!(target_os = "windows") {
                    "powershell.exe"
                } else {
                    "pwsh"
                };
                let mut c = std::process::Command::new(ps_bin);
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
                let sh_bin = if cfg!(target_os = "windows") {
                    if Path::new(r"C:\Program Files\Git\bin\sh.exe").exists() {
                        r"C:\Program Files\Git\bin\sh.exe"
                    } else if Path::new(r"C:\Program Files\Git\usr\bin\sh.exe").exists() {
                        r"C:\Program Files\Git\usr\bin\sh.exe"
                    } else {
                        "sh"
                    }
                } else {
                    "sh"
                };
                let mut c = std::process::Command::new(sh_bin);
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
    if let Some(stripped) = s.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{}", stripped))
    } else if let Some(stripped) = s.strip_prefix(r"\\?\") {
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

    /// Returns list of dynamic tools deterministically sorted by tool name
    pub fn list(&self) -> Vec<&DynamicSkillTool> {
        let mut list: Vec<&DynamicSkillTool> = self.tools.values().collect();
        list.sort_by(|a, b| a.definition.name.cmp(&b.definition.name));
        list
    }

    /// Returns list of tool definitions deterministically sorted by name
    pub fn definitions(&self) -> Vec<&DynamicToolDefinition> {
        let mut list: Vec<&DynamicToolDefinition> = self.tools.values().map(|t| &t.definition).collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
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
    fn test_absolute_path_security_rejection() {
        let abs_scripts = [
            "C:\\Windows\\System32\\cmd.exe",
            "/bin/sh",
            "\\\\server\\share\\script.bat",
        ];

        for script in abs_scripts {
            let def = DynamicToolDefinition {
                name: "malicious_abs".into(),
                description: "Try absolute path".into(),
                parameters: serde_json::json!({}),
                script: script.into(),
                timeout_ms: 1000,
            };

            let tool = DynamicSkillTool::new(def, std::env::temp_dir());
            let res = tool.resolve_script_path();
            assert!(res.is_err(), "Expected error for absolute path: {}", script);
            let err_msg = res.unwrap_err().to_string();
            assert!(
                err_msg.contains("Absolute paths not permitted") || err_msg.contains("Path traversal"),
                "Unexpected error message: {}",
                err_msg
            );
        }
    }

    #[test]
    fn test_dynamic_tool_large_output_no_deadlock() {
        let temp_dir = std::env::temp_dir().join("ride_test_large_output");
        let _ = std::fs::create_dir_all(&temp_dir);

        if cfg!(target_os = "windows") {
            let script_file = temp_dir.join("large_output.bat");
            // Outputs ~20KB of data, far exceeding standard Windows 4KB pipe buffer
            std::fs::write(
                &script_file,
                "@echo off\r\nfor /L %%i in (1,1,400) do echo [LINE_%%i] 1234567890123456789012345678901234567890\r\n",
            )
            .unwrap();

            let def = DynamicToolDefinition {
                name: "large_tool".into(),
                description: "Produces large output".into(),
                parameters: serde_json::json!({}),
                script: "large_output.bat".into(),
                timeout_ms: 10000,
            };

            let tool = DynamicSkillTool::new(def, &temp_dir);
            let output = tool.execute_sync(&serde_json::json!({}), None).unwrap();

            assert_eq!(output.exit_code, 0);
            assert!(output.stdout.contains("[LINE_1]"));
            assert!(output.stdout.contains("[LINE_400]"));
            assert!(output.stdout.len() > 10_000);
        }

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_dynamic_tool_execution_powershell() {
        let temp_dir = std::env::temp_dir().join("ride_test_ps_tool");
        let _ = std::fs::create_dir_all(&temp_dir);

        if cfg!(target_os = "windows") {
            let script_file = temp_dir.join("test_script.ps1");
            std::fs::write(
                &script_file,
                "Write-Host \"PS_OUTPUT: $env:RIDE_TOOL_PARAMS\"\n",
            )
            .unwrap();

            let def = DynamicToolDefinition {
                name: "ps_tool".into(),
                description: "Test PowerShell tool".into(),
                parameters: serde_json::json!({"type": "object", "properties": {"val": {"type": "string"}}}),
                script: "test_script.ps1".into(),
                timeout_ms: 8000,
            };

            let tool = DynamicSkillTool::new(def, &temp_dir);
            let output = tool.execute_sync(&serde_json::json!({"val": "powershell_rocks"}), None).unwrap();

            assert_eq!(output.exit_code, 0);
            assert!(output.stdout.contains("PS_OUTPUT:"));
            assert!(output.stdout.contains("powershell_rocks"));
        }

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_dynamic_tool_execution_python() {
        let temp_dir = std::env::temp_dir().join("ride_test_py_tool");
        let _ = std::fs::create_dir_all(&temp_dir);

        let script_file = temp_dir.join("test_script.py");
        std::fs::write(
            &script_file,
            "import os, sys, json\nparams = os.environ.get('RIDE_TOOL_PARAMS', '{}')\ndata = json.loads(params)\nprint(f\"PY_SUCCESS:{data.get('lang', '')}\")\n",
        )
        .unwrap();

        let def = DynamicToolDefinition {
            name: "py_tool".into(),
            description: "Test Python tool".into(),
            parameters: serde_json::json!({"type": "object", "properties": {"lang": {"type": "string"}}}),
            script: "test_script.py".into(),
            timeout_ms: 8000,
        };

        let tool = DynamicSkillTool::new(def, &temp_dir);
        // Only run if python command succeeds
        if let Ok(output) = tool.execute_sync(&serde_json::json!({"lang": "python3"}), None) {
            assert_eq!(output.exit_code, 0);
            assert!(output.stdout.contains("PY_SUCCESS:python3"));
        }

        let _ = std::fs::remove_dir_all(temp_dir);
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
