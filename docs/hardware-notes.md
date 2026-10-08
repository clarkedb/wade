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

### Flash

espflash's default partition table reserves `nvs` at `0x9000`, 24 KB (six 4 KB sectors), then `phy_init` and a 3.9 MB `factory` app. Wade claims `nvs` for settings; nothing on the board uses ESP-IDF's NVS format.

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

### Measurements

From `wade-cyd` built with its `measure` feature, which logs a summary every 10 s and the uptime every minute.

| What | Result |
|---|---|
| Frame, drawn and sent | 33.2 ms on average, 33.4 ms at most while idle; up to 36 ms while touch is busy |
| Animation | 29 fps, against the core's 33 ms frame |
| Deadlines | Handled 4 ms late on average, 27 ms at most: one due during a frame waits for it to finish |
| Touch to screen | From the touch's sample to the end of the frame showing it: 34–36 ms on average, 39 ms at most. Sampling every 10 ms adds up to 10 ms more. |
| Clock | Over 63 minutes, the board's uptime fell 31 ms behind the host's clock, about 8 ppm. No warnings or restarts in 64 minutes. |

Gaps of up to about 100 ms between frames are the core's own schedule, such as motions that start just after others end; the deadline lateness above shows no frame is held up longer than one frame.


## CoreS3 Lite

The real device. Bring-up firmware: `bringup/cores3/`, with each test kept as an example. Initialization follows M5Unified and M5GFX.

### Board

| | |
|---|---|
| Chip | ESP32-S3 rev v0.2, 16 MB flash |
| PSRAM | 8 MB, quad SPI at 40 MHz (chip ID `0x525D0D`); octal mode would claim the LCD's pins. Passes a full write and read-back. |
| USB serial | Native USB; shows up on macOS as `/dev/cu.usbmodem*` |
| Parts | All nine expected I²C devices answer, so the Lite has everything Wade needs, proximity sensor and battery included |

Internal I²C: SDA 12, SCL 11, 400 kHz.

| Address | Part |
|---|---|
| `0x21` | GC0308 camera |
| `0x23` | LTR-553ALS proximity and light |
| `0x34` | AXP2101 PMIC (chip ID `0x4A`) |
| `0x36` | AW88298 amplifier (ID `0x1852`) |
| `0x38` | FT6336U touch |
| `0x40` | ES7210 microphone codec |
| `0x51` | BM8563 clock |
| `0x58` | AW9523B IO expander |
| `0x69` | BMI270 IMU |

### Power

The AXP2101 keeps its registers across an ESP32 reset while the battery holds, so firmware must set every rail it relies on rather than assume defaults.

AXP2101 writes at boot:

| Register | Value | Meaning |
|---|---|---|
| `0x90` | `0xBF` | Enable ALDO1–4, BLDO1–2, and DLDO1 |
| `0x92` | 13 | ALDO1 1.8 V: amplifier |
| `0x93`–`0x95` | 28 | ALDO2–4 3.3 V: microphone codec, camera, SD card |
| `0x99` | 28 | DLDO1 3.3 V: backlight |
| `0x27` | `0x00` | Power key: hold 1 s on, 4 s off |
| `0x69` | `0x11` | Charge LED |
| `0x10` | `0x30` | PMU common config |
| `0x30` | `0x0F` | ADCs on |

Battery: register `0x00` bit 3 is battery present and bit 5 is VBUS good. Register `0xA4` is the charge in percent.

AW9523B pins:

| Pin | Use | Direction |
|---|---|---|
| P0.0 | Touch reset | Out, high |
| P0.2 | Amplifier reset | Out, high |
| P1.0 | Camera reset | Out, high |
| P1.1 | LCD reset, active low | Out; pulse low 10 ms, then wait 120 ms |
| P1.2 | Touch interrupt | In |
| P1.3 | Amplifier interrupt | In |
| P1.7 | Boost enable | Out, high |

Registers: `0x02`/`0x03` output, `0x04` = `0x18` and `0x05` = `0x0C` direction (1 is input), `0x11` = `0x10` port 0 push-pull, `0x12`/`0x13` = `0xFF` GPIO mode.

### Display

ILI9342C over SPI2. Pins: SCLK 36, MOSI 37, DC 35, CS 3; reset through the AW9523B. MISO shares the DC pin, so the panel cannot be read.

- Init: SWRESET, wait 150 ms, SLPOUT, wait 120 ms, COLMOD `0x55`, MADCTL `0x08` (BGR, native landscape), INVON, DISPON. Colors and orientation are correct.
- Transport, a fixed frame redrawn: DMA is steady at 40 and 80 MHz, drawing while sending included.

| Clock | Bus only | Drawing Wade | Draw, then send | Draw while sending | Blink, 210×90 | Expression, 260×150 |
|---|---|---|---|---|---|---|
| 40 MHz | 30.8 ms | 3.6 ms | 34.4 ms | 31.4 ms | 7.6 ms | 15.7 ms |
| 80 MHz | 15.5 ms | 3.6 ms | 19.1 ms | 16.1 ms | 3.8 ms | 7.9 ms |

Choice: DMA at 80 MHz, drawing one band while the last is sent, about 16 ms a frame. That fits the core's 33 ms frame with room to spare, so M3 does not need `render::damage`.

### Touch

FT6336U at `0x38`, in polling mode (`0xA4` = 0): its interrupt stays low while touched. Reports run at about 100 Hz.

- Read 5 bytes from `0x02`: touch count in the low nibble, then X and Y as 12-bit values. Coordinates are screen coordinates directly, with no swap, flip, or calibration, and reach both edges.
- Interrupt: touch INT goes to AW9523B P1.2, and the AW9523B's interrupt output reaches GPIO 21, low on any input change. Reading the input port (`0x00`/`0x01`) clears it. It went low on every press, so the touch task can wait on it. Mask the other inputs (`0x06`/`0x07`, 1 disables) so only touch wakes it.
- Taps land within a finger's width of the target. Near the right edge, presses with the right index finger read a little right of where they feel.

### Audio

AW88298 over I²S1, Philips format, 16-bit stereo. Pins: BCK 34, WS 33, DOUT 13; no MCLK.

- Init, 16-bit register writes: `0x61` = `0x0673` (boost off), `0x04` = `0x4040` (I²S on, powered up), `0x05` = `0x0008` (unmuted), `0x06` = `0x14C0` plus a rate index (3 for 16 kHz), `0x0C` volume.
- The chime plays cleanly at 16 kHz from one 48 KB DMA buffer, synthesized as on desktop. At half-scale samples and volume `0x0064` it is too loud for a desk; raising the volume register's top nibble (`0x1064`, `0x2064`) steps it down audibly.

### Backlight

DLDO1 voltage, register `0x99`: 0.5 V plus 0.1 V per step. Usable from about step 22 (2.7 V) to 28 (3.3 V), matching M5GFX's range; below that the screen goes dark. Steps 22, 24, 26, and 28 are distinct; stepping one at a time every 60 ms fades smoothly with no flicker. Clearing DLDO1's enable (`0x90` bit 7) turns the backlight off, and the panel keeps its image.

### Memory

| Region | Size | Use |
|---|---|---|
| `dram_seg` | 334 KB | Statics and the main stack, which gets the remainder |
| `dram2_seg` | 72 KB | Heap or explicitly placed buffers only |
| PSRAM | 8 MB | Heap or explicitly placed buffers |

- With two 25.6 KB band buffers: `.data` 10 KB, `.bss` 52 KB, stack 263 KB.
- With a full 150 KB framebuffer in internal SRAM: `.bss` 152 KB, stack 156 KB.
- A full framebuffer at 80 MHz, five 48-row DMA buffers:

| Framebuffer | Draw | Flush | Draw, then flush |
|---|---|---|---|
| Internal SRAM | 3.5 ms | 15.5 ms | 19.0 ms |
| PSRAM | 20.2 ms | 15.7 ms | 36.2 ms |

DMA from PSRAM is correct (esp-hal writes back the cache), but drawing into it is six times slower.

Choice: bands in internal SRAM, as on the CYD. They are as fast as a full framebuffer, since drawing overlaps the bus, and leave about 100 KB more internal RAM for Wi-Fi and TLS in M7. PSRAM stays free for the heap and large buffers that the CPU touches rarely.
