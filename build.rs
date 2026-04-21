use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

/// Run a git command with a wall-clock timeout. Returns `None` on timeout or failure.
/// On timeout the child is orphaned and reaped when the thread completes.
fn git_output(args: &[&str], timeout: Duration) -> Option<String> {
    let child = Command::new("git")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });

    match rx.recv_timeout(timeout) {
        Ok(Ok(output)) if output.status.success() => String::from_utf8(output.stdout)
            .ok()
            .map(|s| s.trim().to_string()),
        Ok(_) => None,
        Err(_) => {
            // Timeout — kill the child. The thread owns it now, so we can't kill
            // directly. The child will be reaped when the thread eventually returns.
            None
        }
    }
}

fn main() {
    let timeout = Duration::from_secs(5);

    // Check if we're in a git repository
    let is_git_repo = git_output(&["rev-parse", "--git-dir"], timeout).is_some();

    let (git_hash, git_describe, git_dirty) = if is_git_repo {
        let hash = git_output(&["rev-parse", "--short", "HEAD"], timeout).unwrap_or_default();
        let tag = git_output(&["describe", "--tags", "--exact-match", "HEAD"], timeout);

        let describe = if let Some(t) = tag {
            t
        } else {
            let branch =
                git_output(&["rev-parse", "--abbrev-ref", "HEAD"], timeout).unwrap_or_default();

            if branch == "HEAD" || branch.is_empty() {
                git_output(&["describe", "--tags", "--always", "--dirty"], timeout)
                    .unwrap_or_else(|| hash.clone())
            } else {
                branch
            }
        };

        let dirty = git_output(&["status", "--porcelain"], timeout)
            .map(|s| !s.is_empty())
            .unwrap_or(false);

        (hash, describe, if dirty { "-dirty" } else { "" })
    } else {
        (String::new(), String::new(), "")
    };

    // Set environment variables for use in the crate
    println!("cargo:rustc-env=GIT_HASH={git_hash}");
    println!("cargo:rustc-env=GIT_DESCRIBE={git_describe}");
    println!("cargo:rustc-env=GIT_DIRTY={git_dirty}");

    // Rerun build.rs if git changes
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/index");
}
