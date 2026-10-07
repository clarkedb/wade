#![no_std]
#![no_main]

//! Step 7: does a full 320×240 Rgb565 framebuffer link, and what is left?

use embassy_executor::Spawner;
use esp_backtrace as _;
use esp_hal::timer::timg::TimerGroup;
use log::info;
use static_cell::ConstStaticCell;

esp_bootloader_esp_idf::esp_app_desc!();

static FRAMEBUFFER: ConstStaticCell<[u8; 320 * 240 * 2]> = ConstStaticCell::new([0; 320 * 240 * 2]);

#[esp_rtos::main]
async fn main(_spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);
    let fb = FRAMEBUFFER.take();
    fb[1000] = 7;
    info!("framebuffer at {:p}, byte {}", fb.as_ptr(), fb[1000]);
    loop {}
}
