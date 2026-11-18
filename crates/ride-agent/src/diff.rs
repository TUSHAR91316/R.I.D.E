use ride_core::error::{Result, RideError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// A single hunk within a unified diff.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiffHunk {
    pub old_start: usize,
    pub old_lines: usize,
    pub new_start: usize,
    pub new_lines: usize,
    /// Lines in the hunk, including prefix indicator (' ', '-', or '+').
    pub lines: Vec<String>,
}

impl DiffHunk {
    pub fn header(&self) -> String {
        let old_part = if self.old_lines == 1 {
            format!("-{}", self.old_start)
        } else if self.old_lines == 0 {
            format!("-{},0", self.old_start)
        } else {
            format!("-{},{}", self.old_start, self.old_lines)
        };

        let new_part = if self.new_lines == 1 {
            format!("+{}", self.new_start)
        } else if self.new_lines == 0 {
            format!("+{},0", self.new_start)
        } else {
            format!("+{},{}", self.new_start, self.new_lines)
        };

        format!("@@ {old_part} {new_part} @@")
    }
}

/// Standard Unified Diff structure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnifiedDiff {
    pub path: PathBuf,
    pub original_header: String,
    pub modified_header: String,
    pub hunks: Vec<DiffHunk>,
    pub diff_text: String,
    pub additions: usize,
    pub deletions: usize,
}

impl UnifiedDiff {
    pub fn has_changes(&self) -> bool {
        self.additions > 0 || self.deletions > 0
    }
}

/// Preview of a proposed file change before applying.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffPreview {
    pub path: PathBuf,
    pub exists_before: bool,
    pub diff: UnifiedDiff,
    pub proposed_content: String,
}

/// In-memory snapshot of a file prior to modification, enabling single-click rollback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupSnapshot {
    pub id: u64,
    pub path: PathBuf,
    /// None if the file was newly created, Some(content) if it previously existed.
    pub previous_content: Option<String>,
    pub timestamp_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DiffOp<'a> {
    Keep(&'a str),
    Delete(&'a str),
    Insert(&'a str),
}

/// Core Unified Diff Engine.
pub struct DiffEngine;

impl DiffEngine {
    /// Generates a standard unified diff between `original` and `modified` contents.
    pub fn create_diff(
        path: impl AsRef<Path>,
        original: &str,
        modified: &str,
        context_lines: usize,
    ) -> UnifiedDiff {
        let path_buf = path.as_ref().to_path_buf();
        let path_str = path_buf.to_string_lossy().replace('\\', "/");

        let orig_is_empty = original.is_empty();
        let mod_is_empty = modified.is_empty();

        let original_header = if orig_is_empty {
            "--- /dev/null".to_string()
        } else {
            format!("--- a/{path_str}")
        };

        let modified_header = if mod_is_empty && !orig_is_empty {
            "+++ /dev/null".to_string()
        } else {
            format!("+++ b/{path_str}")
        };

        if original == modified {
            return UnifiedDiff {
                path: path_buf,
                original_header,
                modified_header,
                hunks: Vec::new(),
                diff_text: String::new(),
                additions: 0,
                deletions: 0,
            };
        }

        let orig_lines: Vec<&str> = if original.is_empty() {
            Vec::new()
        } else {
            original.lines().collect()
        };

        let mod_lines: Vec<&str> = if modified.is_empty() {
            Vec::new()
        } else {
            modified.lines().collect()
        };

        let ops = Self::compute_diff_ops(&orig_lines, &mod_lines);

        let mut additions = 0;
        let mut deletions = 0;
        for op in &ops {
            match op {
                DiffOp::Insert(_) => additions += 1,
                DiffOp::Delete(_) => deletions += 1,
                DiffOp::Keep(_) => {}
            }
        }

        let hunks = Self::build_hunks(&ops, context_lines, orig_is_empty, mod_is_empty);

        let mut diff_text = String::new();
        if !hunks.is_empty() {
            diff_text.push_str(&original_header);
            diff_text.push('\n');
            diff_text.push_str(&modified_header);
            diff_text.push('\n');

            for hunk in &hunks {
                diff_text.push_str(&hunk.header());
                diff_text.push('\n');
                for line in &hunk.lines {
                    diff_text.push_str(line);
                    diff_text.push('\n');
                }
            }
        }

        UnifiedDiff {
            path: path_buf,
            original_header,
            modified_header,
            hunks,
            diff_text,
            additions,
            deletions,
        }
    }

    /// Optimized diff operations using prefix/suffix trimming followed by LCS.
    fn compute_diff_ops<'a>(old: &[&'a str], new: &[&'a str]) -> Vec<DiffOp<'a>> {
        let mut prefix_len = 0;
        while prefix_len < old.len() && prefix_len < new.len() && old[prefix_len] == new[prefix_len] {
            prefix_len += 1;
        }

        let mut suffix_len = 0;
        while suffix_len < (old.len() - prefix_len)
            && suffix_len < (new.len() - prefix_len)
            && old[old.len() - 1 - suffix_len] == new[new.len() - 1 - suffix_len]
        {
            suffix_len += 1;
        }

        let old_mid = &old[prefix_len..old.len() - suffix_len];
        let new_mid = &new[prefix_len..new.len() - suffix_len];

        let mut ops = Vec::with_capacity(old.len() + new.len());

        for line in &old[..prefix_len] {
            ops.push(DiffOp::Keep(line));
        }

        let mid_ops = Self::lcs_diff(old_mid, new_mid);
        ops.extend(mid_ops);

        for line in &old[old.len() - suffix_len..] {
            ops.push(DiffOp::Keep(line));
        }

        ops
    }

    /// Computes diff for the changed middle block using Longest Common Subsequence.
    fn lcs_diff<'a>(old: &[&'a str], new: &[&'a str]) -> Vec<DiffOp<'a>> {
        let m = old.len();
        let n = new.len();

        if m == 0 {
            return new.iter().map(|&s| DiffOp::Insert(s)).collect();
        }
        if n == 0 {
            return old.iter().map(|&s| DiffOp::Delete(s)).collect();
        }

        if m * n > 2_000_000 {
            let mut ops = Vec::with_capacity(m + n);
            for &s in old { ops.push(DiffOp::Delete(s)); }
            for &s in new { ops.push(DiffOp::Insert(s)); }
            return ops;
        }

        let stride = n + 1;
        let mut dp = vec![0u32; (m + 1) * stride];

        for i in 0..m {
            let row = i * stride;
            let next_row = (i + 1) * stride;
            for j in 0..n {
                if old[i] == new[j] {
                    dp[next_row + j + 1] = dp[row + j] + 1;
                } else {
                    dp[next_row + j + 1] = dp[row + j + 1].max(dp[next_row + j]);
                }
            }
        }

        let mut ops = Vec::new();
        let mut i = m;
        let mut j = n;

        while i > 0 || j > 0 {
            let cur_row = i * stride;
            let prev_row = if i > 0 { (i - 1) * stride } else { 0 };

            if i > 0 && j > 0 && old[i - 1] == new[j - 1] {
                ops.push(DiffOp::Keep(old[i - 1]));
                i -= 1;
                j -= 1;
            } else if j > 0 && (i == 0 || dp[cur_row + j - 1] >= dp[prev_row + j]) {
                ops.push(DiffOp::Insert(new[j - 1]));
                j -= 1;
            } else if i > 0 {
                ops.push(DiffOp::Delete(old[i - 1]));
                i -= 1;
            }
        }

        ops.reverse();
        ops
    }

    /// Groups diff operations into contextual hunks.
    fn build_hunks(
        ops: &[DiffOp<'_>],
        context_size: usize,
        orig_is_empty: bool,
        mod_is_empty: bool,
    ) -> Vec<DiffHunk> {
        let mut change_indices = Vec::new();
        for (idx, op) in ops.iter().enumerate() {
            if !matches!(op, DiffOp::Keep(_)) {
                change_indices.push(idx);
            }
        }

        if change_indices.is_empty() {
            return Vec::new();
        }

        // Group changes into clusters separated by more than 2 * context_size
        let mut groups: Vec<(usize, usize)> = Vec::new();
        let mut group_start = change_indices[0];
        let mut group_end = change_indices[0];

        for &idx in &change_indices[1..] {
            if idx <= group_end + 2 * context_size + 1 {
                group_end = idx;
            } else {
                groups.push((group_start, group_end));
                group_start = idx;
                group_end = idx;
            }
        }
        groups.push((group_start, group_end));

        let mut hunks = Vec::new();

        // Track running line numbers for old and new files
        // Precompute line offsets for each op
        let mut old_line_at = Vec::with_capacity(ops.len() + 1);
        let mut new_line_at = Vec::with_capacity(ops.len() + 1);
        let mut cur_old = 1usize;
        let mut cur_new = 1usize;

        for op in ops {
            old_line_at.push(cur_old);
            new_line_at.push(cur_new);
            match op {
                DiffOp::Keep(_) => {
                    cur_old += 1;
                    cur_new += 1;
                }
                DiffOp::Delete(_) => {
                    cur_old += 1;
                }
                DiffOp::Insert(_) => {
                    cur_new += 1;
                }
            }
        }
        old_line_at.push(cur_old);
        new_line_at.push(cur_new);

        for (start_idx, end_idx) in groups {
            let hunk_start = start_idx.saturating_sub(context_size);
            let hunk_end = (end_idx + context_size + 1).min(ops.len());

            let mut hunk_lines = Vec::new();
            let mut old_count = 0;
            let mut new_count = 0;

            for op in &ops[hunk_start..hunk_end] {
                match op {
                    DiffOp::Keep(s) => {
                        hunk_lines.push(format!(" {s}"));
                        old_count += 1;
                        new_count += 1;
                    }
                    DiffOp::Delete(s) => {
                        hunk_lines.push(format!("-{s}"));
                        old_count += 1;
                    }
                    DiffOp::Insert(s) => {
                        hunk_lines.push(format!("+{s}"));
                        new_count += 1;
                    }
                }
            }

            let old_start = if orig_is_empty {
                0
            } else {
                old_line_at[hunk_start]
            };

            let new_start = if mod_is_empty {
                0
            } else {
                new_line_at[hunk_start]
            };

            hunks.push(DiffHunk {
                old_start,
                old_lines: old_count,
                new_start,
                new_lines: new_count,
                lines: hunk_lines,
            });
        }

        hunks
    }
}

/// Thread-safe File Backup and Rollback Manager.
#[derive(Debug, Clone)]
pub struct DiffManager {
    snapshots: Arc<Mutex<Vec<BackupSnapshot>>>,
    next_id: Arc<AtomicU64>,
    context_lines: usize,
}

impl Default for DiffManager {
    fn default() -> Self {
        Self::new(3)
    }
}

impl DiffManager {
    pub fn new(context_lines: usize) -> Self {
        Self {
            snapshots: Arc::new(Mutex::new(Vec::new())),
            next_id: Arc::new(AtomicU64::new(1)),
            context_lines,
        }
    }

    /// Previews writing `new_content` to `path` without modifying disk.
    pub fn preview_write(&self, path: impl AsRef<Path>, new_content: &str) -> Result<DiffPreview> {
        let path = path.as_ref();
        let (original, exists) = if path.exists() {
            (std::fs::read_to_string(path)?, true)
        } else {
            (String::new(), false)
        };

        let diff = DiffEngine::create_diff(path, &original, new_content, self.context_lines);
        Ok(DiffPreview {
            path: path.to_path_buf(),
            exists_before: exists,
            diff,
            proposed_content: new_content.to_string(),
        })
    }

    /// Previews editing `path` by replacing `target` with `replacement` without modifying disk.
    pub fn preview_edit(
        &self,
        path: impl AsRef<Path>,
        target: &str,
        replacement: &str,
    ) -> Result<DiffPreview> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(RideError::Agent(format!(
                "Cannot edit non-existent file: {}",
                path.display()
            )));
        }

        let original = std::fs::read_to_string(path)?;
        let modified = compute_replacement(&original, target, replacement, path)?;

        let diff = DiffEngine::create_diff(path, &original, &modified, self.context_lines);
        Ok(DiffPreview {
            path: path.to_path_buf(),
            exists_before: true,
            diff,
            proposed_content: modified,
        })
    }

    /// Applies `write_file` to disk, records an in-memory backup snapshot, and returns the diff.
    pub fn apply_write_with_backup(
        &self,
        path: impl AsRef<Path>,
        new_content: &str,
    ) -> Result<(UnifiedDiff, BackupSnapshot)> {
        let path = path.as_ref();
        let (original, previous_content) = if path.exists() {
            let content = std::fs::read_to_string(path)?;
            (content.clone(), Some(content))
        } else {
            (String::new(), None)
        };

        let diff = DiffEngine::create_diff(path, &original, new_content, self.context_lines);

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::write(path, new_content)?;

        let snapshot = BackupSnapshot {
            id: self.next_id.fetch_add(1, Ordering::SeqCst),
            path: path.to_path_buf(),
            previous_content,
            timestamp_secs: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        let mut guard = self.snapshots.lock().unwrap();
        guard.push(snapshot.clone());

        Ok((diff, snapshot))
    }

    /// Applies `edit_file` to disk, records an in-memory backup snapshot, and returns the diff.
    pub fn apply_edit_with_backup(
        &self,
        path: impl AsRef<Path>,
        target: &str,
        replacement: &str,
    ) -> Result<(UnifiedDiff, BackupSnapshot)> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(RideError::Agent(format!(
                "File not found to edit: {}",
                path.display()
            )));
        }

        let original = std::fs::read_to_string(path)?;
        let modified = compute_replacement(&original, target, replacement, path)?;

        let diff = DiffEngine::create_diff(path, &original, &modified, self.context_lines);
        std::fs::write(path, &modified)?;

        let snapshot = BackupSnapshot {
            id: self.next_id.fetch_add(1, Ordering::SeqCst),
            path: path.to_path_buf(),
            previous_content: Some(original),
            timestamp_secs: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        let mut guard = self.snapshots.lock().unwrap();
        guard.push(snapshot.clone());

        Ok((diff, snapshot))
    }

    /// Rolls back the latest snapshot for a specific path.
    pub fn rollback_latest_for_path(&self, path: impl AsRef<Path>) -> Result<BackupSnapshot> {
        let path = path.as_ref();
        let mut guard = self.snapshots.lock().unwrap();

        let idx = guard
            .iter()
            .rposition(|s| paths_match(&s.path, path))
            .ok_or_else(|| RideError::Agent(format!("No snapshot found for {}", path.display())))?;

        let snapshot = guard.remove(idx);
        drop(guard);

        Self::restore_snapshot(&snapshot)?;
        Ok(snapshot)
    }

    /// Rolls back a snapshot by its unique ID.
    pub fn rollback_snapshot_id(&self, snapshot_id: u64) -> Result<BackupSnapshot> {
        let mut guard = self.snapshots.lock().unwrap();

        let idx = guard
            .iter()
            .position(|s| s.id == snapshot_id)
            .ok_or_else(|| RideError::Agent(format!("Snapshot #{snapshot_id} not found")))?;

        let snapshot = guard.remove(idx);
        drop(guard);

        Self::restore_snapshot(&snapshot)?;
        Ok(snapshot)
    }

    fn restore_snapshot(snapshot: &BackupSnapshot) -> Result<()> {
        match &snapshot.previous_content {
            Some(content) => {
                if let Some(parent) = snapshot.path.parent() {
                    if !parent.as_os_str().is_empty() {
                        std::fs::create_dir_all(parent)?;
                    }
                }
                std::fs::write(&snapshot.path, content)?;
            }
            None => {
                // File did not exist before this write; rollback removes it
                if snapshot.path.exists() {
                    std::fs::remove_file(&snapshot.path)?;
                }
            }
        }
        Ok(())
    }

    pub fn list_snapshots(&self) -> Vec<BackupSnapshot> {
        self.snapshots.lock().unwrap().clone()
    }

    pub fn can_rollback(&self, path: impl AsRef<Path>) -> bool {
        let path = path.as_ref();
        self.snapshots.lock().unwrap().iter().any(|s| paths_match(&s.path, path))
    }

    pub fn clear_snapshots(&self) {
        self.snapshots.lock().unwrap().clear();
    }
}

pub fn paths_match(stored: &Path, requested: &Path) -> bool {
    if stored == requested {
        return true;
    }
    if let (Ok(ca), Ok(cb)) = (std::fs::canonicalize(stored), std::fs::canonicalize(requested)) {
        if ca == cb {
            return true;
        }
    }
    let s_norm = stored.to_string_lossy().replace('\\', "/").to_lowercase();
    let r_norm = requested.to_string_lossy().replace('\\', "/").to_lowercase();
    if s_norm == r_norm {
        return true;
    }
    let s_clean = s_norm.trim_start_matches("./");
    let r_clean = r_norm.trim_start_matches("./");
    if s_clean == r_clean {
        return true;
    }
    if s_clean.ends_with(&format!("/{r_clean}")) || r_clean.ends_with(&format!("/{s_clean}")) {
        return true;
    }
    false
}

fn compute_replacement(
    content: &str,
    target: &str,
    replacement: &str,
    path: &Path,
) -> Result<String> {
    if content.contains(target) {
        Ok(content.replace(target, replacement))
    } else {
        // CRLF tolerance checks
        let target_crlf = target.replace("\r\n", "\n").replace('\n', "\r\n");
        let replacement_crlf = replacement.replace("\r\n", "\n").replace('\n', "\r\n");
        if content.contains(&target_crlf) {
            Ok(content.replace(&target_crlf, &replacement_crlf))
        } else {
            let target_lf = target.replace("\r\n", "\n");
            let content_lf = content.replace("\r\n", "\n");
            if content_lf.contains(&target_lf) {
                let res = content_lf.replace(&target_lf, replacement);
                if content.contains("\r\n") {
                    Ok(res.replace("\r\n", "\n").replace('\n', "\r\n"))
                } else {
                    Ok(res)
                }
            } else {
                Err(RideError::Agent(format!(
                    "Target string '{}' not found in {}",
                    target,
                    path.display()
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_identical_strings() {
        let diff = DiffEngine::create_diff("test.txt", "abc\ndef\n", "abc\ndef\n", 3);
        assert!(!diff.has_changes());
        assert_eq!(diff.additions, 0);
        assert_eq!(diff.deletions, 0);
        assert!(diff.hunks.is_empty());
    }

    #[test]
    fn test_diff_new_file() {
        let diff = DiffEngine::create_diff("new_file.rs", "", "fn main() {}\n", 3);
        assert!(diff.has_changes());
        assert_eq!(diff.additions, 1);
        assert_eq!(diff.deletions, 0);
        assert!(diff.original_header.contains("/dev/null"));
        assert!(diff.diff_text.contains("+fn main() {}"));
    }

    #[test]
    fn test_diff_edit_middle_line() {
        let original = "line1\nline2\nline3\nline4\nline5\n";
        let modified = "line1\nline2\nline_edited\nline4\nline5\n";
        let diff = DiffEngine::create_diff("sample.txt", original, modified, 1);
        assert!(diff.has_changes());
        assert_eq!(diff.additions, 1);
        assert_eq!(diff.deletions, 1);
        assert_eq!(diff.hunks.len(), 1);
        assert!(diff.diff_text.contains("-line3"));
        assert!(diff.diff_text.contains("+line_edited"));
    }

    #[test]
    fn test_preview_and_apply_with_rollback() {
        let tmp_dir = std::env::temp_dir().join("ride_diff_test");
        let _ = std::fs::create_dir_all(&tmp_dir);
        let test_file = tmp_dir.join("test_rollback.txt");

        let manager = DiffManager::new(3);

        // 1. Preview write for new file
        let preview = manager
            .preview_write(&test_file, "initial content\n")
            .unwrap();
        assert!(!preview.exists_before);
        assert!(preview.diff.has_changes());
        assert!(!test_file.exists()); // Disk not touched yet

        // 2. Apply write with backup
        let (diff, snap1) = manager
            .apply_write_with_backup(&test_file, "initial content\n")
            .unwrap();
        assert!(test_file.exists());
        assert!(diff.has_changes());
        assert_eq!(snap1.previous_content, None);

        // 3. Apply edit with backup
        let (diff2, snap2) = manager
            .apply_edit_with_backup(&test_file, "initial", "updated")
            .unwrap();
        assert_eq!(diff2.additions, 1);
        assert_eq!(diff2.deletions, 1);
        assert_eq!(
            std::fs::read_to_string(&test_file).unwrap(),
            "updated content\n"
        );
        assert_eq!(snap2.previous_content, Some("initial content\n".to_string()));

        // 4. Rollback the edit -> restores "initial content\n"
        let rolled_back = manager.rollback_latest_for_path(&test_file).unwrap();
        assert_eq!(rolled_back.id, snap2.id);
        assert_eq!(
            std::fs::read_to_string(&test_file).unwrap(),
            "initial content\n"
        );

        // 5. Rollback the initial write -> deletes test_file since it was created
        let rolled_back2 = manager.rollback_latest_for_path(&test_file).unwrap();
        assert_eq!(rolled_back2.id, snap1.id);
        assert!(!test_file.exists());

        let _ = std::fs::remove_dir_all(tmp_dir);
    }

    #[test]
    fn test_diff_pure_insertion_in_existing_file_has_correct_old_line_start() {
        let original = "line1\nline2\nline3\n";
        let modified = "line1\nline_inserted\nline2\nline3\n";
        // 0 context lines: pure insertion at line 2
        let diff = DiffEngine::create_diff("test.txt", original, modified, 0);
        assert_eq!(diff.hunks.len(), 1);
        let hunk = &diff.hunks[0];
        // Must NOT be -0,0 because file is not empty!
        assert_ne!(hunk.old_start, 0);
        assert_eq!(hunk.new_lines, 1);
        assert_eq!(hunk.old_lines, 0);
        assert!(hunk.header().contains("+2"));
    }

    #[test]
    fn test_rollback_cross_format_paths() {
        let tmp_dir = std::env::temp_dir().join("ride_diff_cross_path_test");
        let _ = std::fs::create_dir_all(&tmp_dir);
        let test_file = tmp_dir.join("sub").join("nested.rs");

        let manager = DiffManager::new(3);
        let (_, snap) = manager
            .apply_write_with_backup(&test_file, "original content\n")
            .unwrap();

        let (_, _) = manager
            .apply_write_with_backup(&test_file, "modified content\n")
            .unwrap();

        // Query with forward slashes on Windows
        let query_path_fwd = test_file.to_string_lossy().replace('\\', "/");
        assert!(manager.can_rollback(&query_path_fwd));

        let rolled_back = manager.rollback_latest_for_path(&query_path_fwd).unwrap();
        assert_eq!(rolled_back.id, snap.id + 1);
        assert_eq!(
            std::fs::read_to_string(&test_file).unwrap(),
            "original content\n"
        );

        let _ = std::fs::remove_dir_all(tmp_dir);
    }
}
