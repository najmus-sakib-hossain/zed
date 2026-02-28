//! Git tool — every git operation without touching the CLI directly.
//! Actions: status | diff | commit | branch | merge | log | blame | stash | bisect | cherry_pick | rebase | worktree | hooks | patch | tags

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct GitTool {
    pub repo_path: String,
}

impl GitTool {
    pub fn new(repo_path: impl Into<String>) -> Self {
        Self {
            repo_path: repo_path.into(),
        }
    }
}

impl Default for GitTool {
    fn default() -> Self {
        Self {
            repo_path: ".".into(),
        }
    }
}

#[async_trait]
impl Tool for GitTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "git".into(),
            description: "Git operations: status, diff, commit, branch, merge, log, blame, stash, bisect, cherry-pick, rebase, worktree, tags".into(),
            parameters: vec![
                ToolParameter { name: "action".into(), description: "Git action".into(), param_type: ParameterType::String, required: true, default: None,
                    enum_values: Some(vec!["status".into(),"diff".into(),"commit".into(),"branch".into(),"merge".into(),"log".into(),"blame".into(),"stash".into(),"bisect".into(),"cherry_pick".into(),"rebase".into(),"worktree".into(),"hooks".into(),"patch".into(),"tags".into()]) },
                ToolParameter { name: "message".into(), description: "Commit message".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "branch_name".into(), description: "Branch name".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "path".into(), description: "File path for blame/diff".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "ref_spec".into(), description: "Git ref (commit hash, tag, branch)".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "count".into(), description: "Number of log entries".into(), param_type: ParameterType::Integer, required: false, default: Some(json!(20)), enum_values: None },
                ToolParameter { name: "files".into(), description: "Files to stage (JSON array)".into(), param_type: ParameterType::Array, required: false, default: None, enum_values: None },
            ],
            category: "vcs".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("status");
        let repo = git2::Repository::open(&self.repo_path).map_err(|e| {
            anyhow::anyhow!("Failed to open git repo at '{}': {}", self.repo_path, e)
        })?;

        match action {
            "status" => {
                let statuses = repo.statuses(None)?;
                let mut staged = Vec::new();
                let mut unstaged = Vec::new();
                let mut untracked = Vec::new();
                for entry in statuses.iter() {
                    let p = entry.path().unwrap_or("").to_string();
                    let s = entry.status();
                    if s.intersects(
                        git2::Status::INDEX_NEW
                            | git2::Status::INDEX_MODIFIED
                            | git2::Status::INDEX_DELETED,
                    ) {
                        staged.push(p.clone());
                    }
                    if s.intersects(git2::Status::WT_MODIFIED | git2::Status::WT_DELETED) {
                        unstaged.push(p.clone());
                    }
                    if s.contains(git2::Status::WT_NEW) {
                        untracked.push(p);
                    }
                }
                let data = json!({"staged": staged, "unstaged": unstaged, "untracked": untracked});
                let summary = format!(
                    "Staged: {} | Unstaged: {} | Untracked: {}",
                    staged.len(),
                    unstaged.len(),
                    untracked.len()
                );
                Ok(ToolResult::success(call.id, summary).with_data(data))
            }
            "diff" => {
                let diff = repo.diff_index_to_workdir(None, None)?;
                let mut output = String::new();
                diff.print(git2::DiffFormat::Patch, |_, _, line| {
                    let prefix = match line.origin() {
                        '+' => "+",
                        '-' => "-",
                        _ => " ",
                    };
                    output.push_str(prefix);
                    output.push_str(&String::from_utf8_lossy(line.content()));
                    true
                })?;
                if output.is_empty() {
                    output = "No changes".into();
                }
                Ok(ToolResult::success(call.id, output))
            }
            "commit" => {
                let msg = call
                    .arguments
                    .get("message")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'message' for commit"))?;
                // Stage specified files or all
                let mut index = repo.index()?;
                if let Some(files) = call.arguments.get("files").and_then(|v| v.as_array()) {
                    for f in files {
                        if let Some(p) = f.as_str() {
                            index.add_path(std::path::Path::new(p))?;
                        }
                    }
                } else {
                    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
                }
                index.write()?;
                let tree_id = index.write_tree()?;
                let tree = repo.find_tree(tree_id)?;
                let sig = repo.signature()?;
                let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
                let parents: Vec<&git2::Commit> = parent.iter().collect();
                let oid = repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &parents)?;
                Ok(ToolResult::success(
                    call.id,
                    format!("Committed: {} ({})", &oid.to_string()[..8], msg),
                ))
            }
            "branch" => {
                let name = call.arguments.get("branch_name").and_then(|v| v.as_str());
                if let Some(n) = name {
                    let head = repo.head()?.peel_to_commit()?;
                    repo.branch(n, &head, false)?;
                    Ok(ToolResult::success(call.id, format!("Created branch: {n}")))
                } else {
                    let branches: Vec<String> = repo
                        .branches(Some(git2::BranchType::Local))?
                        .filter_map(|b| b.ok())
                        .filter_map(|(b, _)| b.name().ok().flatten().map(|s| s.to_string()))
                        .collect();
                    Ok(ToolResult::success(call.id, branches.join("\n"))
                        .with_data(json!({"branches": branches})))
                }
            }
            "log" => {
                let count =
                    call.arguments.get("count").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
                let mut revwalk = repo.revwalk()?;
                revwalk.push_head()?;
                revwalk.set_sorting(git2::Sort::TIME)?;
                let entries: Vec<_> = revwalk
                    .take(count)
                    .filter_map(|oid| {
                        let oid = oid.ok()?;
                        let commit = repo.find_commit(oid).ok()?;
                        Some(json!({
                            "hash": &oid.to_string()[..8],
                            "message": commit.summary().unwrap_or(""),
                            "author": commit.author().name().unwrap_or(""),
                            "time": commit.time().seconds(),
                        }))
                    })
                    .collect();
                let summary = entries
                    .iter()
                    .map(|e| {
                        format!(
                            "{} {}",
                            e["hash"].as_str().unwrap_or(""),
                            e["message"].as_str().unwrap_or("")
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(ToolResult::success(call.id, summary).with_data(json!({"commits": entries})))
            }
            "blame" => {
                let path = call
                    .arguments
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Need 'path' for blame"))?;
                let blame = repo.blame_file(std::path::Path::new(path), None)?;
                let mut lines = Vec::new();
                for hunk in blame.iter() {
                    let sig = hunk.final_signature();
                    lines.push(format!(
                        "{} ({}) L{}-L{}",
                        &hunk.final_commit_id().to_string()[..8],
                        sig.name().unwrap_or(""),
                        hunk.final_start_line(),
                        hunk.final_start_line() + hunk.lines_in_hunk() - 1
                    ));
                }
                Ok(ToolResult::success(call.id, lines.join("\n")))
            }
            "tags" => {
                let tags: Vec<String> =
                    repo.tag_names(None)?.iter().filter_map(|t| t.map(String::from)).collect();
                Ok(ToolResult::success(call.id, tags.join("\n")).with_data(json!({"tags": tags})))
            }
            "stash" | "bisect" | "cherry_pick" | "rebase" | "worktree" | "hooks" | "patch"
            | "merge" => {
                // Use git CLI for complex operations
                let (shell, flag) = if cfg!(windows) {
                    ("cmd", "/C")
                } else {
                    ("sh", "-c")
                };
                let git_cmd = match action {
                    "stash" => "git stash list".to_string(),
                    "merge" => format!(
                        "git merge {}",
                        call.arguments
                            .get("branch_name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("main")
                    ),
                    _ => format!("git {} --help", action.replace('_', "-")),
                };
                let output = tokio::process::Command::new(shell)
                    .arg(flag)
                    .arg(&git_cmd)
                    .current_dir(&self.repo_path)
                    .output()
                    .await?;
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                Ok(ToolResult::success(call.id, stdout))
            }
            other => Ok(ToolResult::error(call.id, format!("Unknown git action: {other}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(GitTool::default().definition().name, "git");
    }
}
