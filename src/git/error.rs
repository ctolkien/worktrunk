//! Worktrunk error types and formatting helpers
//!
//! Uses anyhow for error propagation. WorktrunkError is a minimal enum for
//! semantic errors that need special handling (exit codes, silent errors).

use std::path::Path;

use super::HookType;
use crate::path::format_path_for_display;
use crate::styling::{ERROR, ERROR_BOLD, ERROR_EMOJI, HINT, HINT_EMOJI, format_with_gutter};

/// Semantic errors that require special handling in main.rs
///
/// Most errors use anyhow::bail! with formatted messages. This enum is only
/// for cases that need exit code extraction or special handling.
#[derive(Debug)]
pub enum WorktrunkError {
    /// Child process exited with non-zero code (preserves exit code for signals)
    ChildProcessExited { code: i32, message: String },
    /// Hook command failed
    HookCommandFailed {
        hook_type: HookType,
        command_name: Option<String>,
        error: String,
        exit_code: Option<i32>,
    },
    /// Command was not approved by user (silent error)
    CommandNotApproved,
}

impl std::fmt::Display for WorktrunkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorktrunkError::ChildProcessExited { message, .. } => {
                write!(f, "{ERROR_EMOJI} {ERROR}{message}{ERROR:#}")
            }
            WorktrunkError::HookCommandFailed {
                hook_type,
                command_name,
                error,
                ..
            } => {
                let name_suffix = command_name
                    .as_ref()
                    .map(|n| format!(": {ERROR_BOLD}{n}{ERROR_BOLD:#}"))
                    .unwrap_or_default();

                write!(
                    f,
                    "{ERROR_EMOJI} {ERROR}{hook_type} command failed{name_suffix}: {error}{ERROR:#}\n\n{HINT_EMOJI} {HINT}Use --no-verify to skip {hook_type} commands{HINT:#}"
                )
            }
            WorktrunkError::CommandNotApproved => {
                Ok(()) // on_skip callback handles the printing
            }
        }
    }
}

impl std::error::Error for WorktrunkError {}

/// Extract exit code from WorktrunkError, if applicable
pub fn exit_code(err: &anyhow::Error) -> Option<i32> {
    err.downcast_ref::<WorktrunkError>().and_then(|e| match e {
        WorktrunkError::ChildProcessExited { code, .. } => Some(*code),
        WorktrunkError::HookCommandFailed { exit_code, .. } => *exit_code,
        WorktrunkError::CommandNotApproved => None,
    })
}

/// Check if error is CommandNotApproved (silent error)
pub fn is_command_not_approved(err: &anyhow::Error) -> bool {
    err.downcast_ref::<WorktrunkError>()
        .is_some_and(|e| matches!(e, WorktrunkError::CommandNotApproved))
}

// =============================================================================
// Error formatting helpers
// =============================================================================

/// Format an error with header and gutter content
fn format_error_block(header: String, error: &str) -> String {
    let trimmed = error.trim();
    if trimmed.is_empty() {
        header
    } else {
        format!("{header}\n{}", format_with_gutter(trimmed, "", None))
    }
}

/// Generic formatted error message
pub fn error_message(msg: impl std::fmt::Display) -> String {
    format!("{ERROR_EMOJI} {ERROR}{msg}{ERROR:#}")
}

/// Parse error
pub fn parse_error(msg: impl std::fmt::Display) -> String {
    format!("{ERROR_EMOJI} {ERROR}{msg}{ERROR:#}")
}

/// Detached HEAD error
pub fn detached_head() -> String {
    format!(
        "{ERROR_EMOJI} {ERROR}Not on a branch (detached HEAD){ERROR:#}\n\n{HINT_EMOJI} {HINT}You are in detached HEAD state{HINT:#}"
    )
}

/// Untracked files error
pub fn untracked_files() -> String {
    format!(
        "{ERROR_EMOJI} {ERROR}Working tree has untracked files{ERROR:#}\n\n{HINT_EMOJI} {HINT}Add them with 'git add' and try again{HINT:#}"
    )
}

/// Uncommitted changes error
pub fn uncommitted_changes() -> String {
    format!(
        "{ERROR_EMOJI} {ERROR}Working tree has uncommitted changes{ERROR:#}\n\n{HINT_EMOJI} {HINT}Commit or stash them first{HINT:#}"
    )
}

/// Branch already exists error
pub fn branch_already_exists(branch: &str) -> String {
    format!(
        "{ERROR_EMOJI} {ERROR}Branch {ERROR_BOLD}{branch}{ERROR_BOLD:#}{ERROR} already exists{ERROR:#}\n\n{HINT_EMOJI} {HINT}Remove --create flag to switch to it{HINT:#}"
    )
}

/// Worktree missing error
pub fn worktree_missing(branch: &str) -> String {
    format!(
        "{ERROR_EMOJI} {ERROR}Worktree directory missing for {ERROR_BOLD}{branch}{ERROR_BOLD:#}{ERROR:#}\n\n{HINT_EMOJI} {HINT}Run 'git worktree prune' to clean up{HINT:#}"
    )
}

/// No worktree found error
pub fn no_worktree_found(branch: &str) -> String {
    format!(
        "{ERROR_EMOJI} {ERROR}No worktree found for branch {ERROR_BOLD}{branch}{ERROR_BOLD:#}{ERROR:#}"
    )
}

/// Worktree path occupied error
pub fn worktree_path_occupied(branch: &str, path: &Path, occupant: Option<&str>) -> String {
    let occupant_note = occupant
        .map(|b| format!(" (currently on {ERROR_BOLD}{b}{ERROR_BOLD:#})"))
        .unwrap_or_default();
    format!(
        "{ERROR_EMOJI} {ERROR}Cannot create worktree for {ERROR_BOLD}{branch}{ERROR_BOLD:#}{ERROR}: target path already exists{ERROR:#}\n\n{HINT_EMOJI} {HINT}Reuse the existing worktree at {}{} or remove it before retrying{HINT:#}",
        format_path_for_display(path),
        occupant_note
    )
}

/// Conflicting changes error
pub fn conflicting_changes(files: &[String], worktree_path: &Path) -> String {
    let mut msg = format!(
        "{ERROR_EMOJI} {ERROR}Cannot push: conflicting uncommitted changes in:{ERROR:#}\n\n"
    );
    if !files.is_empty() {
        let joined_files = files.join("\n");
        msg.push_str(&format_with_gutter(&joined_files, "", None));
    }
    msg.push_str(&format!(
        "\n{HINT_EMOJI} {HINT}Commit or stash these changes in {} first{HINT:#}",
        format_path_for_display(worktree_path)
    ));
    msg
}

/// Not fast-forward error
pub fn not_fast_forward(
    target_branch: &str,
    commits_formatted: &str,
    files_formatted: &str,
) -> String {
    let mut msg = format!(
        "{ERROR_EMOJI} {ERROR}Can't push to local {ERROR_BOLD}{target_branch}{ERROR_BOLD:#} branch: it has newer commits{ERROR:#}\n"
    );

    if !commits_formatted.is_empty() {
        msg.push('\n');
        msg.push_str(commits_formatted);
    }

    if !files_formatted.is_empty() {
        msg.push('\n');
        msg.push_str(files_formatted);
    }

    msg.push_str(&format!(
        "\n{HINT_EMOJI} {HINT}Use 'wt merge' to rebase your changes onto {target_branch}{HINT:#}"
    ));
    msg
}

/// Merge commits found error
pub fn merge_commits_found() -> String {
    format!(
        "{ERROR_EMOJI} {ERROR}Found merge commits in push range{ERROR:#}\n\n{HINT_EMOJI} {HINT}Use --allow-merge-commits to push non-linear history{HINT:#}"
    )
}

/// Not interactive error
pub fn not_interactive() -> String {
    format!(
        "{ERROR_EMOJI} {ERROR}Cannot prompt for approval in non-interactive environment{ERROR:#}\n\n{HINT_EMOJI} {HINT}In CI/CD, use --force to skip prompts. To pre-approve commands, use 'wt beta ask-approvals'{HINT:#}"
    )
}

/// Push failed error
pub fn push_failed(error: &str) -> String {
    let header = format!("{ERROR_EMOJI} {ERROR}Push failed{ERROR:#}");
    format_error_block(header, error)
}

/// Rebase conflict error
pub fn rebase_conflict(target_branch: &str, git_output: &str) -> String {
    let mut msg = format!(
        "{ERROR_EMOJI} {ERROR}Rebase onto {ERROR_BOLD}{target_branch}{ERROR_BOLD:#}{ERROR} incomplete{ERROR:#}"
    );

    if !git_output.is_empty() {
        msg.push('\n');
        msg.push_str(&format_with_gutter(git_output, "", None));
    } else {
        msg.push_str(&format!(
            "\n\n{HINT_EMOJI} {HINT}Resolve conflicts and run 'git rebase --continue'{HINT:#}\n{HINT_EMOJI} {HINT}Or abort with 'git rebase --abort'{HINT:#}"
        ));
    }

    msg
}

/// Worktree path exists error
pub fn worktree_path_exists(path: &Path) -> String {
    format!(
        "{ERROR_EMOJI} {ERROR}Directory already exists: {ERROR_BOLD}{}{ERROR_BOLD:#}{ERROR:#}\n\n{HINT_EMOJI} {HINT}Remove the directory or use a different branch name{HINT:#}",
        format_path_for_display(path)
    )
}

/// Worktree creation failed error
pub fn worktree_creation_failed(branch: &str, base_branch: Option<&str>, error: &str) -> String {
    let base_suffix = base_branch
        .map(|base| format!("{ERROR} from base {ERROR_BOLD}{base}{ERROR_BOLD:#}"))
        .unwrap_or_default();

    let header = format!(
        "{ERROR_EMOJI} {ERROR}Failed to create worktree for {ERROR_BOLD}{branch}{ERROR_BOLD:#}{base_suffix}{ERROR:#}"
    );
    format_error_block(header, error)
}

/// Switch failed error
pub fn switch_failed(branch: &str, error: &str) -> String {
    let header = format!(
        "{ERROR_EMOJI} {ERROR}Failed to switch to {ERROR_BOLD}{branch}{ERROR_BOLD:#}{ERROR:#}"
    );
    format_error_block(header, error)
}

/// Worktree removal failed error
pub fn worktree_removal_failed(branch: &str, path: &Path, error: &str) -> String {
    let header = format!(
        "{ERROR_EMOJI} {ERROR}Failed to remove worktree for {ERROR_BOLD}{branch}{ERROR_BOLD:#}{ERROR} at {ERROR_BOLD}{}{ERROR_BOLD:#}{ERROR:#}",
        format_path_for_display(path)
    );
    format_error_block(header, error)
}

/// Branch deletion failed error
pub fn branch_deletion_failed(branch: &str, error: &str) -> String {
    let header = format!(
        "{ERROR_EMOJI} {ERROR}Failed to delete branch {ERROR_BOLD}{branch}{ERROR_BOLD:#}{ERROR:#}"
    );
    format_error_block(header, error)
}

/// Cannot remove main worktree error
pub fn cannot_remove_main_worktree() -> String {
    format!(
        "{ERROR_EMOJI} {ERROR}Cannot remove main worktree{ERROR:#}\n\n{HINT_EMOJI} {HINT}Use 'wt remove' from a secondary worktree, or 'wt switch' to change branches{HINT:#}"
    )
}

// =============================================================================
// Conversions
// =============================================================================

/// Convert io::Error to anyhow::Error, parsing exit codes from error messages
///
/// Protocol: execute_streaming() embeds exit codes in error messages as:
///   "CHILD_EXIT_CODE:{code} {original_message}"
/// This allows passing exit codes through io::Error while preserving context.
pub fn from_io_error(e: std::io::Error) -> anyhow::Error {
    let msg = e.to_string();
    // Parse exit code from error message (format: "CHILD_EXIT_CODE:130 Command failed...")
    if let Some(rest) = msg.strip_prefix("CHILD_EXIT_CODE:")
        && let Some(space_idx) = rest.find(' ')
        && let Ok(code) = rest[..space_idx].parse::<i32>()
    {
        let message = rest[space_idx + 1..].to_string();
        return WorktrunkError::ChildProcessExited { code, message }.into();
    }
    anyhow::Error::from(e)
}
