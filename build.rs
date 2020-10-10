use std::process::Command;

fn main() {
    // Add `GIT_HASH` environment variable which is the hash of the HEAD commit.
    // This environment variable is used to define the release version of the project,
    // primarily for providing context for errors/panics reported to Sentry.
    // The hash will have the suffix `-dirty` if the workspace is not clean. This is
    // intentional so that we can distinguish specific releases from WIP builds.
    let output = Command::new("git")
        .args(&["describe", "--always", "--abbrev=40"])
        .output()
        .expect("failed to get commit hash");
    let git_hash = String::from_utf8(output.stdout).expect("failed to parse commit hash as utf8");
    if git_hash.is_empty() {
        panic!("failed to parse commit hash: git command output was empty");
    }
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
}
