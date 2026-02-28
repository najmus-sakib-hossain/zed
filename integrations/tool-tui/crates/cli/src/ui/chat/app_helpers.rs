//! Helper functions for chat app

use super::app_data::{Agent, AgentStatus, ChangeType, GitChange, Task, TaskPriority, TaskStatus};
use std::fs;
use std::process::Command;
use std::time::Duration;

pub fn fetch_git_changes() -> (Vec<GitChange>, usize) {
    let mut git_changes = Vec::new();
    let mut changes_count = 0;

    let status_output = Command::new("git").args(["status", "--porcelain"]).output();

    if let Ok(output) = status_output
        && output.status.success()
    {
        let status_str = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = status_str.lines().collect();
        changes_count = lines.len();

        for line in lines {
            if line.len() < 4 {
                continue;
            }

            let status_code = &line[0..2];
            let file_path = line[3..].trim().to_string();

            let change_type = match status_code.trim() {
                "M" | "MM" => ChangeType::Modified,
                "A" | "AM" => ChangeType::Added,
                "D" => ChangeType::Deleted,
                "R" => ChangeType::Renamed,
                _ => ChangeType::Modified,
            };

            let diff_output = if change_type == ChangeType::Added {
                Command::new("git").args(["diff", "--cached", "--", &file_path]).output()
            } else {
                Command::new("git").args(["diff", "HEAD", "--", &file_path]).output()
            };

            let (diff, additions, deletions) = if let Ok(diff_out) = diff_output {
                if diff_out.status.success() {
                    let diff_str = String::from_utf8_lossy(&diff_out.stdout).to_string();
                    let (add, del) = count_diff_lines(&diff_str);
                    (diff_str, add, del)
                } else {
                    (String::new(), 0, 0)
                }
            } else {
                (String::new(), 0, 0)
            };

            git_changes.push(GitChange {
                file_path,
                change_type,
                diff,
                additions,
                deletions,
            });
        }
    }

    (git_changes, changes_count)
}

pub fn count_diff_lines(diff: &str) -> (usize, usize) {
    let mut additions = 0;
    let mut deletions = 0;

    for line in diff.lines() {
        if line.starts_with('+') && !line.starts_with("+++") {
            additions += 1;
        } else if line.starts_with('-') && !line.starts_with("---") {
            deletions += 1;
        }
    }

    (additions, deletions)
}

pub fn fetch_tasks() -> (Vec<Task>, usize) {
    let mut tasks = Vec::new();
    let patterns = ["TODO", "FIXME", "HACK", "NOTE"];

    let files_output = Command::new("git").args(["ls-files"]).output();

    if let Ok(output) = files_output
        && output.status.success()
    {
        let files_str = String::from_utf8_lossy(&output.stdout);

        for file_path in files_str.lines() {
            if file_path.ends_with(".lock")
                || file_path.ends_with(".png")
                || file_path.ends_with(".jpg")
                || file_path.ends_with(".svg")
                || file_path.ends_with(".woff")
                || file_path.ends_with(".woff2")
                || file_path.ends_with(".ttf")
            {
                continue;
            }

            if let Ok(content) = fs::read_to_string(file_path) {
                for (line_num, line) in content.lines().enumerate() {
                    for pattern in &patterns {
                        if let Some(idx) = line.find(pattern) {
                            let after_pattern = &line[idx + pattern.len()..].trim();
                            let description =
                                if let Some(stripped) = after_pattern.strip_prefix(':') {
                                    stripped.trim()
                                } else {
                                    after_pattern
                                };

                            let priority = match *pattern {
                                "FIXME" => TaskPriority::High,
                                "TODO" | "HACK" => TaskPriority::Medium,
                                _ => TaskPriority::Low,
                            };

                            let status = if line.contains("DONE") || line.contains("FIXED") {
                                TaskStatus::Done
                            } else if line.contains("WIP") || line.contains("IN PROGRESS") {
                                TaskStatus::InProgress
                            } else {
                                TaskStatus::Todo
                            };

                            tasks.push(Task {
                                title: pattern.to_string(),
                                description: description.to_string(),
                                priority,
                                status,
                                file_path: Some(file_path.to_string()),
                                line_number: Some(line_num + 1),
                            });

                            break;
                        }
                    }

                    if tasks.len() >= 4 {
                        break;
                    }
                }

                if tasks.len() >= 4 {
                    break;
                }
            }
        }
    }

    let tasks_count = tasks.len();
    (tasks, tasks_count)
}

pub fn play_sound(_sound_type: &str) {
    // Audio disabled
}

pub fn fetch_agents() -> (Vec<Agent>, usize) {
    // Simulated agents for demo - in production this would query actual running agents
    let agents = vec![
        Agent {
            name: "Code Analyzer".to_string(),
            status: AgentStatus::Running,
            model: "Claude-3.5".to_string(),
            task: "Analyzing codebase structure and dependencies".to_string(),
            progress: 0.65,
            tokens_used: 12_450,
            duration: Duration::from_secs(45),
        },
        Agent {
            name: "Test Generator".to_string(),
            status: AgentStatus::Running,
            model: "GPT-4".to_string(),
            task: "Generating unit tests for utils module".to_string(),
            progress: 0.42,
            tokens_used: 8_230,
            duration: Duration::from_secs(28),
        },
        Agent {
            name: "Doc Writer".to_string(),
            status: AgentStatus::Completed,
            model: "Gemini-Pro".to_string(),
            task: "Writing API documentation".to_string(),
            progress: 1.0,
            tokens_used: 15_890,
            duration: Duration::from_secs(120),
        },
        Agent {
            name: "Refactorer".to_string(),
            status: AgentStatus::Paused,
            model: "Claude-3.5".to_string(),
            task: "Refactoring legacy authentication code".to_string(),
            progress: 0.28,
            tokens_used: 5_120,
            duration: Duration::from_secs(15),
        },
    ];

    let agents_count = agents.len();
    (agents, agents_count)
}
