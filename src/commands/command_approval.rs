//! Command approval and execution utilities
//!
//! This module provides shared functionality for approving and executing commands
//! across different worktrunk operations (post-create, post-start, pre-merge).

use worktrunk::config::{ApprovedCommand, CommandConfig, WorktrunkConfig};
use worktrunk::git::GitError;
use worktrunk::styling::{
    AnstyleStyle, HINT_EMOJI, WARNING, WARNING_EMOJI, eprintln, format_with_gutter,
};

/// Convert CommandConfig to a vector of (name, command) pairs
///
/// # Arguments
/// * `config` - The command configuration to convert
/// * `default_prefix` - Prefix for unnamed commands (typically "cmd")
///
/// # Naming Behavior
/// - **Single string**: Uses the exact prefix without numbering
///   - `pre-merge-check = "exit 0"` → `("cmd", "exit 0")`
/// - **Array (even single-element)**: Appends 1-based index to prefix
///   - `pre-merge-check = ["exit 0"]` → `("cmd-1", "exit 0")`
///   - `pre-merge-check = ["a", "b"]` → `("cmd-1", "a"), ("cmd-2", "b")`
/// - **Named table**: Uses the key names directly (sorted alphabetically)
///   - `[pre-merge-check]` `foo="a"` `bar="b"` → `("bar", "b"), ("foo", "a")`
pub fn command_config_to_vec(
    config: &CommandConfig,
    default_prefix: &str,
) -> Vec<(String, String)> {
    match config {
        CommandConfig::Single(cmd) => vec![(default_prefix.to_string(), cmd.clone())],
        CommandConfig::Multiple(cmds) => cmds
            .iter()
            .enumerate()
            .map(|(i, cmd)| (format!("{}-{}", default_prefix, i + 1), cmd.clone()))
            .collect(),
        CommandConfig::Named(map) => {
            let mut pairs: Vec<_> = map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            // Sort by name for deterministic iteration order
            pairs.sort_by(|a, b| a.0.cmp(&b.0));
            pairs
        }
    }
}

/// Check if commands need approval and handle the approval flow
///
/// Returns `Ok(true)` if all commands are approved (or force-approved),
/// `Ok(false)` if user declined approval.
/// Automatically saves approvals when granted.
pub fn approve_command_batch(
    commands: &[(String, String)],
    project_id: &str,
    config: &WorktrunkConfig,
    force: bool,
    context: &str,
) -> Result<bool, GitError> {
    // Find commands that need approval
    let needs_approval: Vec<(&str, &str)> = commands
        .iter()
        .filter(|(_, cmd)| !config.is_command_approved(project_id, cmd))
        .map(|(name, cmd)| (name.as_str(), cmd.as_str()))
        .collect();

    if needs_approval.is_empty() {
        return Ok(true);
    }

    // Prompt or force-approve
    let should_approve = if force {
        true
    } else {
        prompt_for_batch_approval(&needs_approval, project_id)
            .map_err(|e| GitError::CommandFailed(format!("Failed to read user input: {}", e)))?
    };

    if !should_approve {
        let dim = AnstyleStyle::new().dimmed();
        eprintln!("{dim}{context} declined{dim:#}");
        return Ok(false);
    }

    // Save all approvals to config
    let mut fresh_config = WorktrunkConfig::load()
        .map_err(|e| GitError::CommandFailed(format!("Failed to reload config: {}", e)))?;

    // Add each command to approved list
    for (_, command) in &needs_approval {
        if !fresh_config.is_command_approved(project_id, command) {
            fresh_config.approved_commands.push(ApprovedCommand {
                project: project_id.to_string(),
                command: command.to_string(),
            });
        }
    }

    // Save all approvals at once
    if let Err(e) = fresh_config.save() {
        eprintln!("{WARNING_EMOJI} {WARNING}Failed to save command approvals: {e}{WARNING:#}");
        eprintln!("You will be prompted again next time.");
    }

    Ok(true)
}

/// Prompt the user to approve multiple commands for execution
///
/// Displays a formatted prompt asking the user to approve a batch of commands,
/// showing both the project and all commands being requested.
pub fn prompt_for_batch_approval(
    commands: &[(&str, &str)],
    project_id: &str,
) -> std::io::Result<bool> {
    use std::io::{self, Write};
    use worktrunk::styling::eprintln;

    debug_assert!(
        !commands.is_empty(),
        "prompt_for_batch_approval called with empty commands list"
    );

    // Extract just the project name for cleaner display
    let project_name = project_id.split('/').next_back().unwrap_or(project_id);

    let bold = AnstyleStyle::new().bold();
    let dim = AnstyleStyle::new().dimmed();

    eprintln!();
    eprintln!("{WARNING_EMOJI} {WARNING}Permission required to execute in worktree{WARNING:#}");
    eprintln!();
    eprintln!("{bold}{project_name}{bold:#} ({dim}{project_id}{dim:#}) wants to execute:");
    eprintln!();

    // Show each command with its name
    for (name, command) in commands {
        eprintln!("{bold}{name}:{bold:#}");
        eprint!("{}", format_with_gutter(command));
        eprintln!();
    }

    eprint!("{HINT_EMOJI} Allow and remember? {bold}[y/N]{bold:#} ");
    io::stderr().flush()?;

    let mut response = String::new();
    io::stdin().read_line(&mut response)?;

    Ok(response.trim().eq_ignore_ascii_case("y"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_command_config_to_vec_single() {
        let config = CommandConfig::Single("echo test".to_string());
        let result = command_config_to_vec(&config, "cmd");
        assert_eq!(result, vec![("cmd".to_string(), "echo test".to_string())]);
    }

    #[test]
    fn test_command_config_to_vec_multiple() {
        let config = CommandConfig::Multiple(vec!["cmd1".to_string(), "cmd2".to_string()]);
        let result = command_config_to_vec(&config, "check");
        assert_eq!(
            result,
            vec![
                ("check-1".to_string(), "cmd1".to_string()),
                ("check-2".to_string(), "cmd2".to_string())
            ]
        );
    }

    #[test]
    fn test_command_config_to_vec_named() {
        let mut map = HashMap::new();
        map.insert("zebra".to_string(), "z".to_string());
        map.insert("alpha".to_string(), "a".to_string());
        let config = CommandConfig::Named(map);
        let result = command_config_to_vec(&config, "cmd");
        // Should be sorted alphabetically
        assert_eq!(
            result,
            vec![
                ("alpha".to_string(), "a".to_string()),
                ("zebra".to_string(), "z".to_string())
            ]
        );
    }

    #[test]
    fn test_command_config_to_vec_different_prefix() {
        let config = CommandConfig::Single("test".to_string());
        let result1 = command_config_to_vec(&config, "cmd");
        let result2 = command_config_to_vec(&config, "check");
        assert_eq!(result1[0].0, "cmd");
        assert_eq!(result2[0].0, "check");
    }
}
