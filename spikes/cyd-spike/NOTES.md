# CYD spike notes

Running findings; they move to `docs/` when the spike is written up.

## Board

- ESP32-32E N4 module: ESP32 rev v3.1, 4 MB flash, no PSRAM, 240 MHz dual core.
- USB serial: CH340 (`/dev/cu.usbserial-*` on macOS, no driver needed).

## Toolchain

- `espup` installs Rust 1.97 for Xtensa; `wade-core` declares 1.98. It builds unchanged with `--ignore-rust-version`.
- esp-generate 1.4: esp-hal 1.2.2, esp-rtos 0.4 (Embassy), esp-println, esp-backtrace.

## Display

- Panel behaves as an ILI9341: MADCTL `0x28` (landscape, BGR) gives the right colors and orientation with USB-C as usually held; no inversion.
- Pins: SCLK 14, MOSI 13, MISO 12, CS 15, DC 2, backlight 21 (high = on).
- ID reads (`0xD3`, `0x04`) return zeros. Not needed; colors and orientation identify it.
- Init that works: SWRESET, 150 ms, SLPOUT, 120 ms, COLMOD `0x55`, MADCTL, INVOFF, DISPON.

## Flush timing (40-row bands, 25.6 KB each, two DMA buffers)

| Clock | Bus only | Drawing Wade (6 strips) | Draw then send | Draw while sending |
|---|---|---|---|---|
| 40 MHz | 30–31 ms | 4–15 ms by view | 35–46 ms | 31–33 ms |
| 80 MHz | 15 ms | 15 ms | 31 ms | 18 ms (corrupt) |

- esp-hal caps one SPI DMA transfer at 32,736 bytes; a 40-row band fits.
- Stability with a static frame redrawn continuously:
  - plain SPI 40 and 80 MHz: steady
  - DMA 40 MHz: steady
  - DMA 80 MHz: horizontal jitter; much worse when drawing during transfers, still present when idle
- Choice: DMA at 40 MHz with two buffers, about 30 fps. 80 MHz DMA is an open question (GPIO-matrix routing is the suspect).
- Tearing: no TE line, so a changing frame can show a seam. Partial flushes should make it rarer.

## Touch (XPT2046)

- Separate SPI bus: CLK 25, MOSI 32, MISO 39, CS 33, IRQ 36 (low while touched). 2 MHz, mode 0.
- Control bytes: Z1 `0xB1`, Z2 `0xC1`, X `0xD1`, Y `0x91`, then `0xD0` to power down and re-enable IRQ. Value = 16-bit response `>> 3`, 12 bits.
- Axes are swapped: the controller's Y runs along the screen's x, its X along the screen's y.
- Calibration (raw at screen edges): x 0 → 166, x 320 → 3744; y 0 → 272, y 240 → 3908. About 11 raw per px across, 15 down.
- Repeatability at a crosshair: about ±8 raw, under a pixel.
- Release: once IRQ goes high, readings go to x 4095 with z1 near 0. Occasional outliers mid-press.
- Pressure (z1) depends on position: about 160 top-left, 1,300 bottom-right. A threshold of 60 works.
- Filter that works: only while IRQ is low, three samples, all with z1 ≥ 60, median of each axis. ~100 Hz.
- Stylus: dots land under the tip, edges included. Finger: a 2–3 px cluster, fine for 48 px targets.
