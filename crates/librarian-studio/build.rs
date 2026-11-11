//! The identity of the Librarian code this binary is built from: the full commit of the repository
//! the code is in, and whether that code differs from the commit. It is not the analyzed project's
//! commit, which an edition records on its own.
//!
//! `dirty` says that, at build time, something under the code paths below differs from the commit:
//! a tracked file changed and not staged, a change staged in the index, or a file Git does not
//! track and does not ignore. Ignored files do not count; neither does anything outside these
//! paths (notes, documents, `.studio`), since none of it is compiled into the binary.
//!
//! Anything that cannot be told for sure gives `unknown`: no Git, no repository, a repository
//! that does not track this crate, a commit or a status Git cannot give. A failure is never read
//! as "clean". Nothing here stops the build.
//!
//! Cargo runs this again when the commit, the index or the code may have changed: `HEAD`, the
//! branch references, `packed-refs`, the index, and every file and folder under the code paths
//! (Cargo compares the newest time in a folder, which a new or removed file also changes). The
//! paths are given relative to this package, so that they are the files of the copy being built,
//! and the workspace's own are watched even when Git cannot be asked.
use std::{
    path::{Path, PathBuf},
    process::Command,
};

/// What is compiled into the binary, from the repository's root. Only these decide `dirty`.
const CODE: [&str; 3] = ["crates", "Cargo.toml", "Cargo.lock"];

/// What the binary reads back: `unknown`, or `recorded:<commit>:<dirty>`.
const VARIABLE: &str = "LIBRARIAN_BUILDER";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    // Git reads these; a different value can mean another repository or another index.
    for name in ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE", "GIT_COMMON_DIR", "GIT_CEILING_DIRECTORIES", "GIT_DISCOVERY_ACROSS_FILESYSTEM"] {
        println!("cargo:rerun-if-env-changed={name}");
    }
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    watch_without_git(&manifest);
    let identity = identify(&manifest).unwrap_or_else(|| "unknown".to_owned());
    println!("cargo:rustc-env={VARIABLE}={identity}");
}

/// Git's answer, or nothing when Git is missing or says it could not answer. `--no-optional-locks`
/// keeps it from refreshing the index, which would make Cargo run this again.
fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git").arg("--no-optional-locks").arg("-C").arg(dir).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

/// Asks Cargo to run this again when `path` changes, only if it exists: a path that does not exist
/// would make Cargo run this on every build. The path is written relative to this package, which
/// is how Cargo reads it: a build of another copy that shares the target folder then watches its
/// own files, and one whose repository is not there runs this again and finds none.
fn watch(manifest: &Path, path: &Path) {
    if path.exists() {
        println!("cargo:rerun-if-changed={}", relative(manifest, path).display());
    }
}

/// `to` as seen from `from`, through their common folders; as it is when they have none.
fn relative(from: &Path, to: &Path) -> PathBuf {
    let (Ok(from), Ok(to)) = (from.canonicalize(), to.canonicalize()) else { return to.to_path_buf() };
    let (a, b): (Vec<_>, Vec<_>) = (from.components().collect(), to.components().collect());
    let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    if common == 0 {
        return to;
    }
    let mut path: PathBuf = std::iter::repeat_n("..", a.len() - common).collect();
    path.extend(&b[common..]);
    path
}

/// What is watched even when Git cannot be asked: the workspace (the nearest folder above with a
/// `Cargo.lock`), its code and its `.git`. A build that found no identity, or no Git, is run
/// again once any of these changes, and does not keep `unknown` after Git is back.
fn watch_without_git(manifest: &Path) {
    let Some(root) = manifest.ancestors().find(|dir| dir.join("Cargo.lock").is_file()) else { return };
    for path in CODE {
        watch(manifest, &root.join(path));
    }
    let git = root.join(".git");
    if git.is_dir() {
        for path in ["HEAD", "index", "packed-refs"] {
            watch(manifest, &git.join(path));
        }
        watch(manifest, &git.join("refs").join("heads"));
    } else {
        // A worktree: `.git` is a file that names where its repository is.
        watch(manifest, &git);
    }
}

fn identify(manifest: &Path) -> Option<String> {
    let lines = git(manifest, &["rev-parse", "--path-format=absolute", "--show-toplevel", "--git-dir", "--git-common-dir"])?;
    let [top, git_dir, common]: [PathBuf; 3] = lines.lines().map(PathBuf::from).collect::<Vec<_>>().try_into().ok()?;
    // Watched first: a repository that cannot be read now is read again once these change.
    watch(manifest, &git_dir.join("HEAD"));
    watch(manifest, &git_dir.join("index"));
    watch(manifest, &common.join("packed-refs"));
    watch(manifest, &common.join("refs").join("heads"));
    for path in CODE {
        watch(manifest, &top.join(path));
    }
    // The repository found has to be the one that holds this crate, not one around a copy of it.
    git(manifest, &["ls-files", "--error-unmatch", "--", "Cargo.toml"])?;
    let commit = git(&top, &["rev-parse", "--verify", "--end-of-options", "HEAD^{commit}"])?.trim().to_owned();
    let full = (commit.len() == 40 || commit.len() == 64) && commit.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
    if !full {
        return None;
    }
    let mut status = vec!["status", "--porcelain=v1", "-z", "--untracked-files=normal", "--ignore-submodules=none", "--"];
    status.extend(CODE);
    let changes = git(&top, &status)?;
    Some(format!("recorded:{commit}:{}", !changes.is_empty()))
}
