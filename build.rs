use std::process::Command;

fn main() {
    // Check if we're in a git repository
    let is_git_repo = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .ok()
        .map(|output| output.status.success())
        .unwrap_or(false);

    let (git_hash, git_describe, git_dirty) = if is_git_repo {
        // Get git commit hash (short)
        let hash = Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    String::from_utf8(output.stdout).ok()
                } else {
                    None
                }
            })
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        // Try to get the exact tag for this commit (e.g., "v0.1.0")
        let tag = Command::new("git")
            .args(["describe", "--tags", "--exact-match", "HEAD"])
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    String::from_utf8(output.stdout).ok()
                } else {
                    None
                }
            })
            .map(|s| s.trim().to_string());

        // Get branch or fallback to describe (e.g., "v0.1.0-5-gabc123")
        let describe = if let Some(t) = tag {
            // On an exact tag - use the tag name
            t
        } else {
            // Not on a tag - use branch name or fallback to describe
            let branch = Command::new("git")
                .args(["rev-parse", "--abbrev-ref", "HEAD"])
                .output()
                .ok()
                .and_then(|output| {
                    if output.status.success() {
                        String::from_utf8(output.stdout).ok()
                    } else {
                        None
                    }
                })
                .map(|s| s.trim().to_string())
                .unwrap_or_default();

            if branch == "HEAD" || branch.is_empty() {
                // Detached HEAD - try to get a describe string
                Command::new("git")
                    .args(["describe", "--tags", "--always", "--dirty"])
                    .output()
                    .ok()
                    .and_then(|output| {
                        if output.status.success() {
                            String::from_utf8(output.stdout).ok()
                        } else {
                            None
                        }
                    })
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| hash.clone())
            } else {
                branch
            }
        };

        // Check if working directory is dirty
        let dirty = Command::new("git")
            .args(["status", "--porcelain"])
            .output()
            .ok()
            .map(|output| !output.stdout.is_empty())
            .unwrap_or(false);

        (hash, describe, if dirty { "-dirty" } else { "" })
    } else {
        // Not in a git repo - use empty values
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
