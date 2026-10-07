# Hardware notes

Measured facts about each board, from its hardware bring-up. [platforms.md](platforms.md) has the design that uses them.

## CYD (ESP32-2432S028R, single USB-C)

The stand-in board ([D30](decisions.md#d30-the-cyd-stands-in-until-the-cores3-arrives)). Bring-up firmware: `bringup/cyd/`, with each test kept as an example.

### Board

| | |
|---|---|
| Module | ESP32-32E N4: ESP32 rev v3.1, dual core 240 MHz, 4 MB flash, no PSRAM |
| USB serial | CH340; shows up on macOS as `/dev/cu.usbserial-*` with no driver |
| Toolchain | `espup` ships Rust 1.97 for Xtensa; `wade-core` builds on it unchanged |

### Display

ILI9341-compatible, over SPI. Pins: SCLK 14, MOSI 13, MISO 12, CS 15, DC 2, backlight 21.

- Init: SWRESET, wait 150 ms, SLPOUT, wait 120 ms, COLMOD `0x55`, MADCTL `0x28` (landscape, BGR), INVOFF, DISPON. Colors and orientation are correct with no inversion.
- The ID commands (`0xD3`, `0x04`) read back zeros. Not needed.
- esp-hal caps one SPI DMA transfer at 32,736 bytes. A 40-row band (25,600 bytes) fits.
- No TE line, so a changing frame can tear.

Full frame through 40-row bands, two DMA buffers:

| Clock | Bus only | Drawing Wade | Draw, then send | Draw while sending |
|---|---|---|---|---|
| 40 MHz | 30–31 ms | 4–15 ms, by view | 35–46 ms | 31–33 ms |
| 80 MHz | 15 ms | 15 ms | 31 ms | 18 ms, corrupt |

Stability, redrawing a fixed frame: plain SPI is steady at 40 and 80 MHz; DMA is steady at 40 MHz. DMA at 80 MHz shifts the image sideways, badly when drawing during transfers and slightly when not. Routing through the GPIO matrix is the suspect.

Choice: DMA at 40 MHz, drawing one strip while the last is sent, about 30 fps.

### Touch

XPT2046, resistive, on its own SPI bus at 2 MHz. Pins: CLK 25, MOSI 32, MISO 39, CS 33, IRQ 36 (low while touched).

- Control bytes: Z1 `0xB1`, Z2 `0xC1`, X `0xD1`, Y `0x91`, then `0xD0` to power down and re-arm IRQ. A reading is the 16-bit response shifted right by 3.
- Axes are swapped: the controller's Y runs along the screen's x, its X along the screen's y.
- Calibration, raw at the screen's edges: x 0 → 166, x 320 → 3744; y 0 → 272, y 240 → 3908.
- Repeatability: ±8 raw at a fixed point, under a pixel.
- After release, readings jump to x 4095 with z1 near 0, and a press has occasional outliers.
- Pressure (z1) varies with position, from about 160 at the top left to 1,300 at the bottom right.
- Filter that works, at about 100 Hz: only while IRQ is low, take three samples, keep them only if all have z1 ≥ 60, then use the median of each axis. A stylus lands under its tip, edges included; a finger lands within a 2–3 px cluster.

### Audio

Amp on GPIO 26 (DAC2) to a 2-pin JST 1.25 mm speaker socket. Untested: no speaker on hand. The chime example plays the chime as PWM square waves with a hardware duty fade, and as 16 kHz DAC samples of the desktop's bell. PWM is preferred because it runs without the CPU.

### Backlight

PWM on GPIO 21 (LEDC, 5 kHz, 10-bit). 25, 50, 75, and 100% are distinct, and 25% is a usable dim level. Hardware fades are smooth with no flicker.

### Memory

| Region | Size | Use |
|---|---|---|
| `dram_seg` | 192 KB | Statics and the main stack, which gets the remainder |
| `dram2_seg` | about 96 KB | Heap or explicitly placed buffers only |

- With two 25.6 KB band buffers: `.data` 9 KB, `.bss` 52 KB, stack 136 KB, `dram2` free.
- A full 150 KB framebuffer links but leaves a 39 KB stack for an otherwise empty program. `StaticCell::init([0; N])` builds the array on the stack and overflows it; `ConstStaticCell::new([0; N])` places it directly in `.bss`.
- Bands win: a full framebuffer gains nothing at 40 MHz, where the bus is the limit, and would leave little room for Wi-Fi.
