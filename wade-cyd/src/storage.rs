//! Settings in the flash partition the partition table reserves for NVS data.
//! Wade does not use ESP-IDF's NVS format, so the partition is his to use.

use embassy_embedded_hal::adapter::BlockingAsync;
use esp_bootloader_esp_idf::partitions::{
    DataPartitionSubType, PARTITION_TABLE_MAX_LEN, PartitionType, read_partition_table,
};
use esp_hal::peripherals::FLASH;
use esp_storage::FlashStorage;
use log::warn;
use wade_firmware::settings_store::SettingsStore;

pub type Store = SettingsStore<BlockingAsync<FlashStorage<'static>>>;

/// The settings store, or `None`, logged, if the partition cannot be found or used.
pub fn open(flash: FLASH<'static>) -> Option<Store> {
    let mut flash = FlashStorage::new(flash);
    let mut table = [0; PARTITION_TABLE_MAX_LEN];
    let partition = match read_partition_table(&mut flash, &mut table)
        .and_then(|t| t.find_partition(PartitionType::Data(DataPartitionSubType::Nvs)))
    {
        Ok(Some(partition)) => partition,
        Ok(None) => {
            warn!("no NVS partition; settings will not be kept");
            return None;
        }
        Err(e) => {
            warn!("partition table unreadable ({e:?}); settings will not be kept");
            return None;
        }
    };
    let range = partition.offset()..partition.offset() + partition.len();
    SettingsStore::new(BlockingAsync::new(flash), range)
        .inspect_err(|e| warn!("NVS partition unusable ({e:?}); settings will not be kept"))
        .ok()
}
