use insta::assert_snapshot;
use worktrunk::git::{push_failed, worktree_removal_failed};

#[test]
fn display_worktree_removal_failed() {
    let msg = worktree_removal_failed(
        "feature-x",
        std::path::Path::new("/tmp/repo.feature-x"),
        "fatal: worktree is dirty\nerror: could not remove worktree",
    );

    assert_snapshot!("worktree_removal_failed", msg);
}

#[test]
fn display_push_failed() {
    let msg = push_failed(
        "To /Users/user/workspace/repo/.git\n ! [remote rejected] HEAD -> main (Up-to-date check failed)\nerror: failed to push some refs to '/Users/user/workspace/repo/.git'",
    );

    assert_snapshot!("push_failed", msg);
}
