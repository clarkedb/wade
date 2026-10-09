mod versions;

use std::{fs, path::Path};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(about = "Check Wade product versions")]
struct Cli {
    #[command(subcommand)]
    command: Task,
}

#[derive(Debug, Subcommand)]
enum Task {
    /// Check that manifests and lockfiles share the product version.
    CheckVersion,
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
    }
    Ok(())
}

fn read_toml(path: &Path) -> Result<toml::Value> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("parse {}", path.display()))
}
