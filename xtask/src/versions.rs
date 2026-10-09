use std::{collections::BTreeSet, fs, path::Path};

use anyhow::{Context, Result, ensure};

use crate::read_toml;

const BOARDS: [&str; 2] = ["wade-cyd", "wade-cores3"];

pub fn check(root: &Path) -> Result<String> {
    let version = fs::read_to_string(root.join("version.txt"))?
        .trim()
        .to_owned();
    let parsed =
        semver::Version::parse(&version).context("version.txt must contain a semantic version")?;
    ensure!(
        parsed.build.is_empty(),
        "product versions cannot contain build metadata"
    );

    let workspace = read_toml(&root.join("Cargo.toml"))?;
    ensure!(
        workspace
            .get("workspace")
            .and_then(|value| value.get("package"))
            .and_then(|value| value.get("version"))
            .and_then(toml::Value::as_str)
            == Some(&version),
        "workspace version differs from {version}"
    );
    let members = workspace
        .get("workspace")
        .and_then(|value| value.get("members"))
        .and_then(toml::Value::as_array)
        .context("missing workspace members")?;
    let mut names = BTreeSet::new();
    for member in members {
        let member = member.as_str().context("workspace member must be a path")?;
        let manifest = read_toml(&root.join(member).join("Cargo.toml"))?;
        let package = manifest.get("package").context("missing package")?;
        ensure!(
            package
                .get("version")
                .and_then(|value| value.get("workspace"))
                .and_then(toml::Value::as_bool)
                == Some(true),
            "{member} must inherit the workspace version"
        );
        names.insert(
            package
                .get("name")
                .and_then(toml::Value::as_str)
                .context("missing package name")?
                .to_owned(),
        );
    }
    let workspace_names = names.clone();
    for board in BOARDS {
        let manifest = read_toml(&root.join(board).join("Cargo.toml"))?;
        ensure!(
            manifest
                .get("package")
                .and_then(|value| value.get("version"))
                .and_then(toml::Value::as_str)
                == Some(&version),
            "{board} version differs from {version}"
        );
        names.insert(board.to_owned());
    }
    check_lock(root, "Cargo.lock", &version, &names, &workspace_names)?;
    for board in BOARDS {
        let expected = ["wade-core", "wade-firmware", board]
            .map(str::to_owned)
            .into();
        check_lock(
            root,
            &format!("{board}/Cargo.lock"),
            &version,
            &names,
            &expected,
        )?;
    }
    Ok(version)
}

fn check_lock(
    root: &Path,
    path: &str,
    version: &str,
    names: &BTreeSet<String>,
    expected: &BTreeSet<String>,
) -> Result<()> {
    let lock = read_toml(&root.join(path))?;
    let packages = lock
        .get("package")
        .and_then(toml::Value::as_array)
        .context("missing lockfile packages")?;
    let mut found = BTreeSet::new();
    for package in packages {
        let name = package
            .get("name")
            .and_then(toml::Value::as_str)
            .context("missing locked package name")?;
        if names.contains(name) {
            ensure!(
                package.get("version").and_then(toml::Value::as_str) == Some(version),
                "{path}: {name} version differs from {version}"
            );
            found.insert(name.to_owned());
        }
    }
    ensure!(
        expected.is_subset(&found),
        "{path}: missing Wade packages: {:?}",
        expected.difference(&found).collect::<Vec<_>>()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fmt::Write;

    use super::*;

    fn repository() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("version.txt"), "0.1.0\n").unwrap();
        fs::write(root.join("Cargo.toml"), "[workspace]\nmembers = ['wade-core', 'wade-firmware', 'xtask']\n[workspace.package]\nversion = '0.1.0'\n").unwrap();
        for (member, name) in [
            ("wade-core", "wade-core"),
            ("wade-firmware", "wade-firmware"),
            ("xtask", "wade-xtask"),
        ] {
            fs::create_dir(root.join(member)).unwrap();
            fs::write(
                root.join(member).join("Cargo.toml"),
                format!("[package]\nname = '{name}'\nversion.workspace = true\n"),
            )
            .unwrap();
        }
        for board in BOARDS {
            fs::create_dir(root.join(board)).unwrap();
            fs::write(
                root.join(board).join("Cargo.toml"),
                format!("[package]\nname = '{board}'\nversion = '0.1.0'\n"),
            )
            .unwrap();
            write_lock(
                &root.join(board).join("Cargo.lock"),
                &["wade-core", "wade-firmware", board],
            );
        }
        write_lock(
            &root.join("Cargo.lock"),
            &["wade-core", "wade-firmware", "wade-xtask"],
        );
        dir
    }

    fn write_lock(path: &Path, names: &[&str]) {
        let mut text = String::new();
        for name in names {
            writeln!(text, "[[package]]\nname = '{name}'\nversion = '0.1.0'").unwrap();
        }
        fs::write(path, text).unwrap();
    }

    #[test]
    fn accepts_consistent_versions() {
        assert_eq!(check(repository().path()).unwrap(), "0.1.0");
    }

    #[test]
    fn rejects_drift_in_each_lockfile() {
        for path in [
            "Cargo.lock",
            "wade-cyd/Cargo.lock",
            "wade-cores3/Cargo.lock",
        ] {
            let dir = repository();
            let file = dir.path().join(path);
            let contents = fs::read_to_string(&file).unwrap().replace("0.1.0", "0.2.0");
            fs::write(file, contents).unwrap();
            assert!(check(dir.path()).unwrap_err().to_string().contains(path));
        }
    }

    #[test]
    fn rejects_a_missing_locked_workspace_member() {
        let dir = repository();
        write_lock(
            &dir.path().join("Cargo.lock"),
            &["wade-core", "wade-firmware"],
        );
        assert!(
            check(dir.path())
                .unwrap_err()
                .to_string()
                .contains("wade-xtask")
        );
    }

    #[test]
    fn requires_workspace_inheritance_and_board_versions() {
        for path in [
            "xtask/Cargo.toml",
            "wade-cyd/Cargo.toml",
            "wade-cores3/Cargo.toml",
        ] {
            let dir = repository();
            fs::write(
                dir.path().join(path),
                "[package]\nname = 'wade'\nversion = '0.2.0'\n",
            )
            .unwrap();
            assert!(check(dir.path()).is_err());
        }
    }

    #[test]
    fn rejects_invalid_product_versions() {
        for version in ["01.1.0", "0.1", "0.1.0+local", "0.1.0-!"] {
            let dir = repository();
            fs::write(dir.path().join("version.txt"), version).unwrap();
            assert!(check(dir.path()).is_err());
        }
    }
}
