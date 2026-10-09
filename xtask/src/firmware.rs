use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, ensure};
use clap::ValueEnum;
use serde_json::json;
use sha2::{Digest, Sha256};
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

use crate::{
    command_output,
    layout::{APP_OFFSET, Layout, SETTINGS_OFFSET, TABLE_OFFSET},
    versions,
};

const ESPFLASH_VERSION: &str = "4.6.0";

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Board {
    Cyd,
    Cores3,
}

impl Board {
    pub fn id(self) -> &'static str {
        match self {
            Self::Cyd => "cyd",
            Self::Cores3 => "cores3",
        }
    }

    fn chip(self) -> &'static str {
        match self {
            Self::Cyd => "esp32",
            Self::Cores3 => "esp32s3",
        }
    }

    fn family(self) -> &'static str {
        match self {
            Self::Cyd => "ESP32",
            Self::Cores3 => "ESP32-S3",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::Cyd => "CYD (ESP32-2432S028R)",
            Self::Cores3 => "M5Stack CoreS3 Lite",
        }
    }

    pub fn boot_offset(self) -> usize {
        match self {
            Self::Cyd => 0x1000,
            Self::Cores3 => 0,
        }
    }

    pub fn flash_bytes(self) -> usize {
        match self {
            Self::Cyd => 4 * 1024 * 1024,
            Self::Cores3 => 16 * 1024 * 1024,
        }
    }

    pub fn flash_header(self) -> u8 {
        match self {
            Self::Cyd => 0x20,
            Self::Cores3 => 0x40,
        }
    }
}

pub fn package(root: &Path, board: Board, output: &Path) -> Result<PathBuf> {
    let version = versions::check(root)?;
    let actual = command_output(Command::new("espflash").arg("--version"))?;
    ensure!(
        actual == format!("espflash {ESPFLASH_VERSION}"),
        "install espflash {ESPFLASH_VERSION}; found {actual}"
    );
    let crate_path = root.join(format!("wade-{}", board.id()));
    let layout = Layout::load(&crate_path, board)?;
    let elf = crate_path
        .join("target")
        .join(format!("xtensa-{}-none-elf", board.chip()))
        .join("release")
        .join(format!("wade-{}", board.id()));
    ensure!(
        elf.is_file(),
        "build the board in release mode first: {}",
        elf.display()
    );
    let tmp = tempfile::Builder::new().prefix("wade-package-").tempdir()?;
    let path = tmp.path();
    save_image(&crate_path, board, &elf, &path.join("install.bin"), true)?;
    save_image(&crate_path, board, &elf, &path.join("app.bin"), false)?;
    let install = fs::read(path.join("install.bin"))?;
    let app = fs::read(path.join("app.bin"))?;
    layout.validate_images(board, &install, &app)?;
    fs::write(
        path.join("bootloader.bin"),
        &install[board.boot_offset()..TABLE_OFFSET],
    )?;
    fs::write(
        path.join("partitions.bin"),
        &install[TABLE_OFFSET..SETTINGS_OFFSET],
    )?;
    fs::copy(
        crate_path.join("partitions.csv"),
        path.join("partitions.csv"),
    )?;
    fs::copy(elf, path.join("firmware.elf"))?;
    write_metadata(root, path, board, &version, &layout)?;
    fs::write(path.join("README.txt"), instructions(board, &version))?;
    write_checksums(path)?;

    fs::create_dir_all(output)?;
    let archive = output.join(format!("wade-v{version}-{}.zip", board.id()));
    write_archive(path, &archive)?;
    Ok(archive)
}

fn save_image(
    crate_path: &Path,
    board: Board,
    elf: &Path,
    destination: &Path,
    merge: bool,
) -> Result<()> {
    // The board's working directory makes espflash load cargo run's configuration.
    let mut command = Command::new("espflash");
    command
        .current_dir(crate_path)
        .args(["save-image", "--chip", board.chip()]);
    if merge {
        command.args(["--merge", "--skip-padding"]);
    }
    command_output(command.arg(elf).arg(destination))?;
    Ok(())
}

fn write_metadata(
    root: &Path,
    destination: &Path,
    board: Board,
    version: &str,
    layout: &Layout,
) -> Result<()> {
    for (mode, binary, offset) in [
        ("install", "install.bin", 0),
        ("update", "app.bin", APP_OFFSET),
    ] {
        write_json(
            &destination.join(format!("{mode}.json")),
            &json!({
                "name": format!("Wade — {}", board.title()),
                "version": version,
                "new_install_prompt_erase": true,
                "new_install_improv_wait_time": 0,
                "builds": [{ "chipFamily": board.family(), "parts": [{"path": binary, "offset": offset}] }],
            }),
        )?;
    }
    let commit = command_output(
        Command::new("git")
            .current_dir(root)
            .args(["rev-parse", "HEAD"]),
    )?;
    write_json(
        &destination.join("build.json"),
        &json!({
            "version": version,
            "board": board.id(),
            "chip": board.chip(),
            "commit": commit,
            "flash_bytes": layout.flash_bytes,
            "app_offset": APP_OFFSET,
            "espflash": ESPFLASH_VERSION,
        }),
    )
}

fn write_json(path: &Path, value: &serde_json::Value) -> Result<()> {
    let mut text = serde_json::to_string_pretty(value)?;
    text.push('\n');
    fs::write(path, text)?;
    Ok(())
}

fn instructions(board: Board, version: &str) -> String {
    let title = board.title();
    let chip = board.chip();
    format!(
        "Wade {version} — {title}

Download espflash {ESPFLASH_VERSION} for your computer and put it on PATH:
https://github.com/esp-rs/espflash/releases/tag/v{ESPFLASH_VERSION}
Use a USB data cable and close any serial monitor first.
Add --port YOUR_PORT to the espflash command if several devices are connected.

FIRST INSTALL / RESET (replaces firmware and discards saved settings):
  espflash erase-flash --chip {chip}
  espflash write-bin --chip {chip} 0x0 install.bin

UPDATE EXISTING WADE (keeps settings with the compatible partition layout):
  espflash write-bin --chip {chip} 0x10000 app.bin

Do not erase flash or use install.bin when preserving settings.
Check release notes before downgrading: older firmware may not read newer settings.
Browser update: use update.json and leave Erase device unchecked.
Recovery and supported hardware: https://github.com/clarkedb/wade/blob/main/docs/releases.md
"
    )
}

fn package_files(path: &Path) -> Result<Vec<PathBuf>> {
    let mut files = fs::read_dir(path)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    files.sort();
    Ok(files)
}

fn write_checksums(path: &Path) -> Result<()> {
    use std::fmt::Write;

    let mut checksums = String::new();
    for file in package_files(path)? {
        let name = file
            .file_name()
            .and_then(|name| name.to_str())
            .context("invalid package filename")?;
        let digest = Sha256::digest(fs::read(&file)?);
        for byte in digest {
            write!(checksums, "{byte:02x}")?;
        }
        writeln!(checksums, "  {name}")?;
    }
    fs::write(path.join("SHA256SUMS"), checksums)?;
    Ok(())
}

fn write_archive(path: &Path, archive: &Path) -> Result<()> {
    // Persist only a complete ZIP so a failed write cannot leave a release candidate.
    let parent = archive.parent().context("missing archive directory")?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    let mut bundle = ZipWriter::new(temporary.as_file_mut());
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for file in package_files(path)? {
        let name = file
            .file_name()
            .and_then(|name| name.to_str())
            .context("invalid package filename")?;
        bundle.start_file(name, options)?;
        std::io::copy(&mut File::open(file)?, &mut bundle)?;
    }
    bundle.finish()?.flush()?;
    temporary.persist(archive)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use super::*;

    #[test]
    fn zip_preserves_files_and_standard_sha256_checksums() {
        let dir = tempfile::tempdir().unwrap();
        let contents = dir.path().join("contents");
        fs::create_dir(&contents).unwrap();
        fs::write(contents.join("app.bin"), b"abc").unwrap();
        fs::write(contents.join("empty.bin"), b"").unwrap();
        write_checksums(&contents).unwrap();
        let expected = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad  app.bin\n\
e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  empty.bin\n";
        assert_eq!(
            fs::read_to_string(contents.join("SHA256SUMS")).unwrap(),
            expected
        );
        let archive = dir.path().join("firmware.zip");
        write_archive(&contents, &archive).unwrap();
        let mut bundle = zip::ZipArchive::new(File::open(archive).unwrap()).unwrap();
        assert_eq!(bundle.len(), 3);
        for (name, expected) in [
            ("app.bin", b"abc".as_slice()),
            ("empty.bin", b""),
            ("SHA256SUMS", expected.as_bytes()),
        ] {
            let mut bytes = Vec::new();
            bundle
                .by_name(name)
                .unwrap()
                .read_to_end(&mut bytes)
                .unwrap();
            assert_eq!(bytes, expected);
        }
    }

    #[test]
    fn failed_zip_write_keeps_the_previous_archive() {
        let dir = tempfile::tempdir().unwrap();
        let contents = dir.path().join("contents");
        fs::create_dir_all(contents.join("invalid.bin")).unwrap();
        let output = dir.path().join("output");
        fs::create_dir(&output).unwrap();
        let archive = output.join("firmware.zip");
        fs::write(&archive, b"previous archive").unwrap();
        assert!(write_archive(&contents, &archive).is_err());
        assert_eq!(fs::read(&archive).unwrap(), b"previous archive");
        assert_eq!(fs::read_dir(output).unwrap().count(), 1);
    }
}
