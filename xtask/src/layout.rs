use std::{collections::BTreeMap, fs, path::Path};

use anyhow::{Context, Result, bail, ensure};

use crate::{firmware::Board, read_toml};

pub const TABLE_OFFSET: usize = 0x8000;
pub const SETTINGS_OFFSET: usize = 0x9000;
pub const APP_OFFSET: usize = 0x10000;

#[derive(Debug, PartialEq, Eq)]
pub struct Partition {
    pub kind: u8,
    pub subtype: u8,
    pub offset: usize,
    pub size: usize,
}

#[derive(Debug)]
pub struct Layout {
    pub flash_bytes: usize,
    pub partitions: BTreeMap<String, Partition>,
}

impl Layout {
    pub fn load(crate_path: &Path, board: Board) -> Result<Self> {
        let config = read_toml(&crate_path.join("espflash.toml"))?;
        let flash = config.get("flash").context("missing flash configuration")?;
        ensure!(
            flash.get("mode").and_then(toml::Value::as_str) == Some("dio")
                && flash.get("frequency").and_then(toml::Value::as_str) == Some("40MHz"),
            "flash configuration must use DIO and 40MHz"
        );
        let size = format!("{}MB", board.flash_bytes() / (1024 * 1024));
        ensure!(
            flash.get("size").and_then(toml::Value::as_str) == Some(&size),
            "{} requires {size} flash",
            board.id()
        );
        let idf = config
            .get("idf")
            .context("missing partition table configuration")?;
        ensure!(
            idf.get("partition_table").and_then(toml::Value::as_str) == Some("partitions.csv")
                && idf
                    .get("partition_table_offset")
                    .and_then(toml::Value::as_integer)
                    == Some(0x8000),
            "partition table must use partitions.csv at 0x8000"
        );
        let csv = fs::read(crate_path.join("partitions.csv"))?;
        Self::parse(&csv, board.flash_bytes())
    }

    fn parse(csv: &[u8], flash_bytes: usize) -> Result<Self> {
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .comment(Some(b'#'))
            .trim(csv::Trim::All)
            .from_reader(csv);
        let mut partitions = BTreeMap::new();
        for record in reader.records() {
            let record = record.context("parse partitions.csv")?;
            ensure!(record.len() == 6, "partition row must contain six columns");
            let (kind_name, subtype_name, kind, subtype) = match &record[0] {
                "nvs" => ("data", "nvs", 1, 2),
                "phy_init" => ("data", "phy", 1, 1),
                "factory" => ("app", "factory", 0, 0),
                name => bail!("unsupported partition {name}"),
            };
            ensure!(
                &record[1] == kind_name && &record[2] == subtype_name && record[5].is_empty(),
                "{} partition type or flags differ from the pinned layout",
                &record[0]
            );
            let partition = Partition {
                kind,
                subtype,
                offset: number(&record[3])?,
                size: number(&record[4])?,
            };
            ensure!(
                partitions.insert(record[0].to_owned(), partition).is_none(),
                "duplicate partition {}",
                &record[0]
            );
        }
        let layout = Self {
            flash_bytes,
            partitions,
        };
        for (name, offset, size) in [
            ("nvs", SETTINGS_OFFSET, 0x6000),
            ("phy_init", 0xf000, 0x1000),
        ] {
            let partition = layout
                .partitions
                .get(name)
                .with_context(|| format!("missing {name} partition"))?;
            ensure!(
                partition.offset == offset && partition.size == size,
                "{name} partition differs from the pinned layout"
            );
        }
        let app = layout.app()?;
        ensure!(
            app.offset == APP_OFFSET
                && app.size > 0
                && app
                    .offset
                    .checked_add(app.size)
                    .is_some_and(|end| end <= flash_bytes),
            "factory partition must start at 0x10000 and fit in flash"
        );
        Ok(layout)
    }

    pub fn app(&self) -> Result<&Partition> {
        self.partitions
            .get("factory")
            .context("missing factory partition")
    }

    pub fn validate_images(&self, board: Board, install: &[u8], app: &[u8]) -> Result<()> {
        ensure!(
            app.first() == Some(&0xe9) && app.len() <= self.app()?.size,
            "application must fit the factory partition and have an ESP image header"
        );
        ensure!(
            install.get(board.boot_offset()) == Some(&0xe9),
            "missing bootloader"
        );
        let header = [2, board.flash_header()];
        // espflash silently falls back to defaults if its TOML cannot be decoded.
        ensure!(
            install.get(board.boot_offset() + 2..board.boot_offset() + 4) == Some(&header),
            "bootloader flash settings differ from the board configuration"
        );
        ensure!(
            app.get(2..4) == Some(&header),
            "application flash settings differ from the board configuration"
        );
        ensure!(
            install.len() <= self.flash_bytes,
            "installation image exceeds flash capacity"
        );
        let table = install
            .get(TABLE_OFFSET..SETTINGS_OFFSET)
            .context("truncated partition table")?;
        self.validate_table(table)?;
        let end = APP_OFFSET
            .checked_add(app.len())
            .context("application length overflow")?;
        ensure!(
            install.get(APP_OFFSET..end) == Some(app),
            "install/update application mismatch"
        );
        Ok(())
    }

    fn validate_table(&self, table: &[u8]) -> Result<()> {
        let mut partitions = BTreeMap::new();
        for entry in table[..0xc00].as_chunks::<32>().0 {
            if entry[..2] != [0xaa, 0x50] {
                break;
            }
            let label = &entry[12..28];
            let label = label
                .split(|byte| *byte == 0)
                .next()
                .context("missing partition label")?;
            let name = std::str::from_utf8(label).context("invalid partition label")?;
            ensure!(
                entry[28..32] == [0; 4],
                "{name} partition flags must be zero"
            );
            let partition = Partition {
                kind: entry[2],
                subtype: entry[3],
                offset: usize::try_from(u32::from_le_bytes(entry[4..8].try_into()?))?,
                size: usize::try_from(u32::from_le_bytes(entry[8..12].try_into()?))?,
            };
            ensure!(
                partitions.insert(name.to_owned(), partition).is_none(),
                "duplicate generated partition {name}"
            );
        }
        ensure!(
            partitions == self.partitions,
            "generated partition table differs from the pinned layout"
        );
        Ok(())
    }
}

fn number(text: &str) -> Result<usize> {
    match text.strip_prefix("0x") {
        Some(hex) => usize::from_str_radix(hex, 16),
        None => text.parse(),
    }
    .with_context(|| format!("invalid partition offset/size {text}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(board: Board) -> Layout {
        let crate_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join(format!("wade-{}", board.id()));
        Layout::load(&crate_path, board).unwrap()
    }

    fn images(board: Board) -> (Vec<u8>, Vec<u8>) {
        let layout = layout(board);
        let app = vec![0xe9, 1, 2, board.flash_header(), 0xab, 0xcd];
        let mut install = vec![0xff; APP_OFFSET];
        install[board.boot_offset()..board.boot_offset() + 4].copy_from_slice(&app[..4]);
        for (index, (name, partition)) in layout.partitions.iter().enumerate() {
            let start = TABLE_OFFSET + index * 32;
            let entry = &mut install[start..start + 32];
            entry.fill(0);
            entry[..4].copy_from_slice(&[0xaa, 0x50, partition.kind, partition.subtype]);
            entry[4..8].copy_from_slice(&u32::try_from(partition.offset).unwrap().to_le_bytes());
            entry[8..12].copy_from_slice(&u32::try_from(partition.size).unwrap().to_le_bytes());
            entry[12..12 + name.len()].copy_from_slice(name.as_bytes());
        }
        install.extend_from_slice(&app);
        (install, app)
    }

    #[test]
    fn accepts_images_for_both_boards() {
        for board in [Board::Cyd, Board::Cores3] {
            let (install, app) = images(board);
            layout(board)
                .validate_images(board, &install, &app)
                .unwrap();
        }
    }

    #[test]
    fn rejects_default_flash_headers() {
        for board in [Board::Cyd, Board::Cores3] {
            let (install, app) = images(board);
            for offset in [board.boot_offset() + 2, board.boot_offset() + 3] {
                let mut corrupt = install.clone();
                corrupt[offset] = 0;
                assert!(
                    layout(board)
                        .validate_images(board, &corrupt, &app)
                        .unwrap_err()
                        .to_string()
                        .contains("bootloader flash settings")
                );
            }
            for offset in [2, 3] {
                let mut corrupt = app.clone();
                corrupt[offset] = 0;
                assert!(
                    layout(board)
                        .validate_images(board, &install, &corrupt)
                        .unwrap_err()
                        .to_string()
                        .contains("application flash settings")
                );
            }
        }
    }

    #[test]
    fn rejects_changed_partition_fields() {
        let (install, app) = images(Board::Cyd);
        for field in [0, 2, 3, 4, 8, 12, 28] {
            let mut corrupt = install.clone();
            corrupt[TABLE_OFFSET + field] ^= 1;
            assert!(
                layout(Board::Cyd)
                    .validate_images(Board::Cyd, &corrupt, &app)
                    .is_err()
            );
        }
    }

    #[test]
    fn rejects_truncated_and_mismatched_images_without_panicking() {
        let (install, app) = images(Board::Cyd);
        for length in [
            0,
            1,
            3,
            TABLE_OFFSET,
            SETTINGS_OFFSET - 1,
            APP_OFFSET + app.len() - 1,
        ] {
            assert!(
                layout(Board::Cyd)
                    .validate_images(Board::Cyd, &install[..length], &app)
                    .is_err()
            );
        }
        for length in [0, 1, 3] {
            assert!(
                layout(Board::Cyd)
                    .validate_images(Board::Cyd, &install, &app[..length])
                    .is_err()
            );
        }
        let mut corrupt = install;
        corrupt[APP_OFFSET + 4] ^= 1;
        assert!(
            layout(Board::Cyd)
                .validate_images(Board::Cyd, &corrupt, &app)
                .unwrap_err()
                .to_string()
                .contains("install/update application mismatch")
        );
    }

    #[test]
    fn rejects_oversized_images() {
        let (install, app) = images(Board::Cyd);
        let mut too_large = app.clone();
        too_large.resize(layout(Board::Cyd).app().unwrap().size + 1, 0);
        assert!(
            layout(Board::Cyd)
                .validate_images(Board::Cyd, &install, &too_large)
                .is_err()
        );
        let mut too_large = install;
        too_large.resize(Board::Cyd.flash_bytes() + 1, 0);
        assert!(
            layout(Board::Cyd)
                .validate_images(Board::Cyd, &too_large, &app)
                .is_err()
        );
    }

    #[test]
    fn rejects_settings_moves_and_duplicate_csv_partitions() {
        let csv = include_str!("../../wade-cyd/partitions.csv");
        for invalid in [
            csv.replace("0x9000", "0xa000"),
            csv.replace("0x6000", "0x5000"),
            csv.replace("0x10000", "0x20000"),
            format!("{csv}\nnvs, data, nvs, 0x9000, 0x6000,\n"),
        ] {
            assert!(Layout::parse(invalid.as_bytes(), Board::Cyd.flash_bytes()).is_err());
        }
    }

    #[test]
    fn rejects_espflash_config_values_that_trigger_fallbacks() {
        let csv = include_str!("../../wade-cores3/partitions.csv");
        let config = include_str!("../../wade-cores3/espflash.toml");
        for invalid in [
            config.replace("16MB", "16Mb"),
            config.replace("40MHz", "40"),
            config.replace("dio", "qio"),
            config.replace("32768", "36864"),
            config.replace("partitions.csv", "other.csv"),
        ] {
            let dir = tempfile::tempdir().unwrap();
            fs::write(dir.path().join("partitions.csv"), csv).unwrap();
            fs::write(dir.path().join("espflash.toml"), invalid).unwrap();
            assert!(Layout::load(dir.path(), Board::Cores3).is_err());
        }
    }
}
