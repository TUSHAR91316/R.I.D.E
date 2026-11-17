use ride_core::error::{Result, RideError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchMatch {
    pub file: PathBuf,
    pub line_number: usize,
    pub line_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub struct Toolset;

impl Toolset {
    pub fn read_file(path: impl AsRef<Path>, start_line: Option<usize>, end_line: Option<usize>) -> Result<String> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| RideError::Agent(format!("Cannot read file: {e}")))?;

        let lines: Vec<&str> = content.lines().collect();
        if lines.is_empty() {
            return Ok(String::new());
        }

        let start = start_line.unwrap_or(1).saturating_sub(1);
        let end = end_line.unwrap_or(lines.len()).min(lines.len());

        if start >= lines.len() || start >= end {
            return Ok(String::new());
        }

        let slice = &lines[start..end];
        Ok(slice.join("\n"))
    }

    pub fn write_file(path: impl AsRef<Path>, content: &str) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn edit_file(path: impl AsRef<Path>, target: &str, replacement: &str) -> Result<()> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .map_err(|e| RideError::Agent(format!("Cannot read file to edit: {e}")))?;

        let new_content = if content.contains(target) {
            content.replace(target, replacement)
        } else {
            // Check if file uses CRLF while target uses LF
            let target_crlf = target.replace("\r\n", "\n").replace('\n', "\r\n");
            let replacement_crlf = replacement.replace("\r\n", "\n").replace('\n', "\r\n");
            if content.contains(&target_crlf) {
                content.replace(&target_crlf, &replacement_crlf)
            } else {
                // Check if file uses LF while target uses CRLF
                let target_lf = target.replace("\r\n", "\n");
                let content_lf = content.replace("\r\n", "\n");
                if content_lf.contains(&target_lf) {
                    content_lf.replace(&target_lf, replacement)
                } else {
                    return Err(RideError::Agent(format!(
                        "Target string '{}' not found in {}",
                        target,
                        path.display()
                    )));
                }
            }
        };

        std::fs::write(path, new_content)?;
        Ok(())
    }

    pub fn list_dir(path: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
        let path = path.as_ref();
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            entries.push(entry.path());
        }
        entries.sort();
        Ok(entries)
    }

    pub fn search_code(dir: impl AsRef<Path>, query: &str, case_sensitive: bool) -> Result<Vec<SearchMatch>> {
        let dir = dir.as_ref();
        let mut matches = Vec::new();
        let query_normalized = if case_sensitive {
            query.to_string()
        } else {
            query.to_lowercase()
        };

        Self::search_dir_recursive(dir, &query_normalized, case_sensitive, &mut matches)?;
        Ok(matches)
    }

    fn search_dir_recursive(
        dir: &Path,
        query: &str,
        case_sensitive: bool,
        matches: &mut Vec<SearchMatch>,
    ) -> Result<()> {
        if !dir.exists() || !dir.is_dir() {
            return Ok(());
        }

        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return Ok(()),
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if file_name.starts_with('.') || file_name == "target" || file_name == "node_modules" {
                continue;
            }

            if path.is_dir() {
                let _ = Self::search_dir_recursive(&path, query, case_sensitive, matches);
            } else if path.is_file() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    for (line_idx, line) in content.lines().enumerate() {
                        let matched = if case_sensitive {
                            line.contains(query)
                        } else {
                            line.to_lowercase().contains(query)
                        };

                        if matched {
                            matches.push(SearchMatch {
                                file: path.clone(),
                                line_number: line_idx + 1,
                                line_text: line.to_string(),
                            });
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn run_command(cmd: &str, working_dir: Option<&Path>) -> Result<CommandOutput> {
        let mut command = if cfg!(target_os = "windows") {
            let mut c = std::process::Command::new("powershell");
            c.args(["-NoProfile", "-Command", cmd]);
            c
        } else {
            let mut c = std::process::Command::new("sh");
            c.args(["-c", cmd]);
            c
        };

        if let Some(dir) = working_dir {
            command.current_dir(dir);
        }

        let output = command
            .output()
            .map_err(|e| RideError::Agent(format!("Failed to execute command '{cmd}': {e}")))?;

        Ok(CommandOutput {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_tools_in_temp() {
        let tmp = std::env::temp_dir().join("ride_test_file.txt");
        Toolset::write_file(&tmp, "line1\nline2\nline3\n").unwrap();
        let content = Toolset::read_file(&tmp, Some(1), Some(2)).unwrap();
        assert_eq!(content, "line1\nline2");

        Toolset::edit_file(&tmp, "line2", "replaced_line").unwrap();
        let updated = Toolset::read_file(&tmp, None, None).unwrap();
        assert!(updated.contains("replaced_line"));

        let _ = std::fs::remove_file(tmp);
    }

    #[test]
    fn test_read_file_inverted_bounds() {
        let tmp = std::env::temp_dir().join("ride_bounds_test.txt");
        Toolset::write_file(&tmp, "line1\nline2\nline3\n").unwrap();
        let content = Toolset::read_file(&tmp, Some(5), Some(2)).unwrap();
        assert_eq!(content, "");
        let _ = std::fs::remove_file(tmp);
    }

    #[test]
    fn test_edit_file_crlf_tolerance() {
        let tmp = std::env::temp_dir().join("ride_crlf_test.txt");
        std::fs::write(&tmp, "fn main() {\r\n    println!(\"hello\");\r\n}\r\n").unwrap();
        // Target uses LF instead of CRLF
        Toolset::edit_file(&tmp, "fn main() {\n    println!(\"hello\");\n}", "fn main() {\n    println!(\"world\");\n}").unwrap();
        let updated = std::fs::read_to_string(&tmp).unwrap();
        assert!(updated.contains("world"));
        let _ = std::fs::remove_file(tmp);
    }

    #[test]
    fn test_run_command_echo() {
        let out = Toolset::run_command("Write-Output 'ride_agent_ok'", None).unwrap();
        assert_eq!(out.exit_code, 0);
        assert!(out.stdout.contains("ride_agent_ok"));
    }
}
