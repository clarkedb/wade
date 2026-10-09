use std::path::Path;
use std::process::Command;

fn git(root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

pub fn emit() {
    let manifest =
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the manifest directory");
    let root = Path::new(&manifest)
        .parent()
        .expect("platform is under the repository root");
    // Directory watching observes unstaged edits too; Cargo ignores build outputs.
    println!("cargo:rerun-if-changed={}", root.display());
    for name in ["HEAD", "index", "packed-refs"] {
        if let Some(path) = git(root, &["rev-parse", "--git-path", name]) {
            println!("cargo:rerun-if-changed={}", root.join(path).display());
        }
    }
    if let Some(reference) = git(root, &["symbolic-ref", "-q", "HEAD"])
        && let Some(path) = git(root, &["rev-parse", "--git-path", &reference])
    {
        println!("cargo:rerun-if-changed={}", root.join(path).display());
    }
    let version = std::env::var("CARGO_PKG_VERSION").expect("Cargo supplies the package version");
    let tag = format!("refs/tags/v{version}");
    if let Some(path) = git(root, &["rev-parse", "--git-path", &tag]) {
        println!("cargo:rerun-if-changed={}", root.join(path).display());
    }
    let head = git(root, &["rev-parse", "HEAD"]);
    let revision =
        git(root, &["rev-parse", "--short=7", "HEAD"]).unwrap_or_else(|| "unknown".into());
    let dirty = git(root, &["status", "--porcelain"]).is_some_and(|status| !status.is_empty());
    let tagged = head.is_some()
        && git(
            root,
            &["rev-parse", "--verify", &format!("{tag}^{{commit}}")],
        ) == head;
    let development = !tagged || dirty;
    let label = if development {
        format!(
            "{version}-dev+{revision}{}",
            if dirty { ".dirty" } else { "" }
        )
    } else {
        version.clone()
    };
    for (key, value) in [
        ("WADE_VERSION", version),
        ("WADE_BUILD_VERSION", label),
        ("WADE_REVISION", revision),
        ("WADE_DEVELOPMENT", development.to_string()),
        ("WADE_DIRTY", dirty.to_string()),
    ] {
        println!("cargo:rustc-env={key}={value}");
    }
}
