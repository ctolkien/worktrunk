use anyhow::Context;
use std::path::PathBuf;
use worktrunk::config::WorktrunkConfig;
use worktrunk::git::{Repository, detached_head};

use super::command_executor::CommandContext;

/// Shared execution context for command handlers that operate on the current worktree.
///
/// Centralizes the common "repo + branch + config + cwd" setup so individual handlers
/// can focus on their core logic while sharing consistent error messaging.
///
/// This helper is used for commands that explicitly act on "where the user is standing"
/// (e.g., `beta` and `merge`) and therefore need all of these pieces together. Commands that
/// inspect multiple worktrees or run without a config/branch requirement (`list`, `select`,
/// some `worktree` helpers) still call `Repository::current()` directly so they can operate in
/// broader contexts without forcing config loads or branch resolution.
pub struct CommandEnv {
    pub repo: Repository,
    pub branch: String,
    pub config: WorktrunkConfig,
    pub worktree_path: PathBuf,
    pub repo_root: PathBuf,
}

impl CommandEnv {
    /// Load the command environment from the current process context.
    pub fn current() -> anyhow::Result<Self> {
        let repo = Repository::current();
        let worktree_path = std::env::current_dir()
            .map_err(|e| anyhow::anyhow!("Failed to get current directory: {}", e))?;
        let branch = repo
            .current_branch()
            .context("Failed to get current branch")?
            .ok_or_else(|| anyhow::anyhow!("{}", detached_head()))?;
        let config = WorktrunkConfig::load().context("Failed to load config")?;
        let repo_root = repo.worktree_base()?;

        Ok(Self {
            repo,
            branch,
            config,
            worktree_path,
            repo_root,
        })
    }

    /// Build a `CommandContext` tied to this environment.
    pub fn context(&self, force: bool) -> CommandContext<'_> {
        CommandContext::new(
            &self.repo,
            &self.config,
            &self.branch,
            &self.worktree_path,
            &self.repo_root,
            force,
        )
    }
}
