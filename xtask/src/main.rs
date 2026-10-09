mod firmware;
mod layout;
mod versions;

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, ensure};
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(about = "Check and package Wade releases")]
struct Cli {
    #[command(subcommand)]
    command: Task,
}

#[derive(Debug, Subcommand)]
enum Task {
    /// Check that manifests and lockfiles share the product version.
    CheckVersion,
    /// Package a built release ELF for USB installation and updates.
    PackageFirmware {
        #[arg(value_enum)]
        board: firmware::Board,
        /// Destination for the firmware ZIP (defaults to the repository's dist directory).
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .context("missing repository root")?;
    match Cli::parse().command {
        Task::CheckVersion => {
            let version = versions::check(root)?;
            println!("Wade {version}: manifests and lockfiles agree");
        }
        Task::PackageFirmware { board, output } => {
            let output = output.unwrap_or_else(|| root.join("dist"));
            let archive = firmware::package(root, board, &output)?;
            println!("{}", archive.display());
        }
    }
    Ok(())
}

fn read_toml(path: &Path) -> Result<toml::Value> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("parse {}", path.display()))
}

fn command_output(command: &mut Command) -> Result<String> {
    let output = command
        .output()
        .with_context(|| format!("run {command:?}"))?;
    ensure!(
        output.status.success(),
        "{command:?} failed: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}
