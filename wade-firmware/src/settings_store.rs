//! Settings kept in a region of NOR flash (docs/roadmap.md#m5-settings).
//!
//! The bytes are the core's `Settings::encode`, stored as one key of a
//! `sequential-storage` map, which spreads writes across the region's pages
//! instead of erasing the same sector on every save.

use core::ops::Range;

use embedded_storage_async::nor_flash::NorFlash;
use sequential_storage::Error;
use sequential_storage::cache::{Cache, Uncached};
use sequential_storage::map::{MapConfig, MapConfigError, MapStorage};
use wade_core::Settings;
use wade_core::settings::ENCODED_LEN;

const SETTINGS_KEY: u8 = 0;

/// Room for the key and value, rounded up to any flash's word size. Aligned
/// because some flash drivers otherwise copy through a sector-sized stack buffer.
#[repr(align(4))]
struct Buffer([u8; BUFFER_LEN]);

const BUFFER_LEN: usize = 32;
const _: () = assert!(
    size_of::<u8>() + ENCODED_LEN <= BUFFER_LEN,
    "the buffer holds a key and settings"
);

pub struct SettingsStore<S: NorFlash> {
    map: MapStorage<u8, S, Cache<Uncached, Uncached, Uncached, u8>>,
    buffer: Buffer,
}

impl<S: NorFlash> core::fmt::Debug for SettingsStore<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SettingsStore").finish_non_exhaustive()
    }
}

impl<S: NorFlash> SettingsStore<S> {
    /// A store in `range` of `flash`, which must span at least two erase
    /// blocks and start and end on erase-block boundaries.
    ///
    /// # Errors
    ///
    /// Returns why `range` cannot hold a store.
    pub fn new(flash: S, range: Range<u32>) -> Result<Self, MapConfigError> {
        let config = MapConfig::try_new(range)?;
        Ok(SettingsStore {
            map: MapStorage::new(flash, config, Cache::new_uncached()),
            buffer: Buffer([0; BUFFER_LEN]),
        })
    }

    /// The stored settings. Missing, corrupt, or unknown-version bytes give
    /// the defaults, as `Settings::decode` defines.
    ///
    /// # Errors
    ///
    /// Returns the flash's error if it cannot be read; the caller then starts
    /// with the defaults too.
    pub async fn load(&mut self) -> Result<Settings, Error<S::Error>> {
        let bytes = self
            .map
            .fetch_item::<&[u8]>(&mut self.buffer.0, &SETTINGS_KEY)
            .await?;
        Ok(Settings::decode(bytes.unwrap_or_default()))
    }

    /// Store `settings`. If the region is found to be corrupt, it is erased
    /// and the save tried once more.
    ///
    /// # Errors
    ///
    /// Returns the flash's error if the save still fails.
    pub async fn save(&mut self, settings: &Settings) -> Result<(), Error<S::Error>> {
        let bytes = settings.encode();
        match self.store(&bytes).await {
            Err(Error::Corrupted { .. }) => {
                self.map.erase_all().await?;
                self.store(&bytes).await
            }
            result => result,
        }
    }

    async fn store(&mut self, bytes: &[u8]) -> Result<(), Error<S::Error>> {
        self.map
            .store_item(&mut self.buffer.0, &SETTINGS_KEY, &bytes)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embassy_futures::block_on;
    use sequential_storage::mock_flash::{MockFlashBase, WriteCountCheck};
    use wade_core::view::{ColorMode, EyeStyle};

    /// Four 1 KiB pages with 4-byte words, like a small slice of real flash.
    type Flash = MockFlashBase<4, 4, 256>;
    const RANGE: Range<u32> = 0..4 * 1024;

    fn flash() -> Flash {
        Flash::new(WriteCountCheck::Twice, None, true)
    }

    fn changed() -> Settings {
        Settings::DEFAULT
            .with_eye_style(EyeStyle::Plain)
            .with_color(ColorMode::Mono)
            .with_chime(false)
    }

    #[test]
    fn an_empty_store_gives_the_defaults() {
        let mut store = SettingsStore::new(flash(), RANGE).unwrap();
        assert_eq!(block_on(store.load()).unwrap(), Settings::DEFAULT);
    }

    #[test]
    fn saved_settings_load_back_after_a_restart() {
        let mut store = SettingsStore::new(flash(), RANGE).unwrap();
        block_on(store.save(&changed())).unwrap();
        let (flash, _) = store.map.destroy();
        let mut store = SettingsStore::new(flash, RANGE).unwrap();
        assert_eq!(block_on(store.load()).unwrap(), changed());
    }

    #[test]
    fn the_last_of_many_saves_wins() {
        // Enough saves to fill the pages several times over and force erases.
        let mut store = SettingsStore::new(flash(), RANGE).unwrap();
        for i in 0..1_000 {
            let settings = if i % 2 == 0 {
                changed()
            } else {
                Settings::DEFAULT
            };
            block_on(store.save(&settings)).unwrap();
        }
        assert_eq!(block_on(store.load()).unwrap(), Settings::DEFAULT);
    }

    #[test]
    fn unreadable_bytes_give_the_defaults() {
        let mut store = SettingsStore::new(flash(), RANGE).unwrap();
        block_on(store.store(&[0xEE; 3])).unwrap();
        assert_eq!(block_on(store.load()).unwrap(), Settings::DEFAULT);
    }

    #[test]
    fn a_corrupt_region_is_erased_on_the_next_save() {
        let mut store = SettingsStore::new(flash(), RANGE).unwrap();
        block_on(store.save(&changed())).unwrap();
        let (mut flash, _) = store.map.destroy();
        for (i, byte) in flash.as_bytes_mut().iter_mut().enumerate() {
            *byte = u8::try_from(i * 37 % 251).unwrap();
        }
        let mut store = SettingsStore::new(flash, RANGE).unwrap();
        assert!(
            block_on(store.load()).is_err(),
            "garbage is reported, not trusted"
        );
        block_on(store.save(&changed())).unwrap();
        assert_eq!(block_on(store.load()).unwrap(), changed());
    }

    #[test]
    fn a_range_off_the_erase_boundaries_is_refused() {
        assert!(SettingsStore::new(flash(), 100..4 * 1024).is_err());
        assert!(
            SettingsStore::new(flash(), 0..1024).is_err(),
            "one page is too few"
        );
    }
}
