# Platforms

The desktop platform runs the loop in [architecture.md](architecture.md#core-api). The planned CoreS3 Lite firmware will use the same core API: stamp events, handle effects, redraw when asked, and wake at `App::next_deadline`.

## Desktop (`wade-desktop`)

| Concern | Implementation |
|---|---|
| Window | `embedded-graphics-simulator`: a 320×240 Rgb565 display shown at 2× scale. Requires SDL2. |
| Clock | `std::time::Instant` captured at startup; `Instant::from_millis(elapsed_ms)`. |
| Input | Mouse button down → `Touch Down`; mouse movement while held → `Touch Move`; button up → `Touch Up`. Points passed to the core must be in 320×240 display coordinates. The simulator reports display coordinates after undoing the window scale. Number keys 0–9, Z, and P → `Key` events, which show an expression, put Wade to sleep, or switch his eye style (see [character.md](character.md#m1)). |
| Loop | The simulator offers only non-blocking event polling. The loop polls window events, then sleeps until the next deadline or for 10 ms, whichever is sooner. This polling stays inside the desktop crate. |
| Audio | `rodio`, synthesizing `wade_core::sound::CHIME` as bell-like notes on its own thread, so the loop never waits. No audio files; without an output device, Wade runs silent. |
| Settings | A `settings` file under `wade/` in the user's config directory (`~/Library/Application Support` on macOS), replaced atomically on each `SaveSettings`. Failed writes retain the latest value and retry every two seconds, including on quit. A pending core change is also saved on quit. A missing, unreadable, or corrupt file starts Wade with the defaults. A replay starts with the settings it recorded and saves nothing. |
| Brightness | A simulated backlight: every pixel is scaled as the backlight would scale its light, gamma-corrected so 25% looks like a quarter of the light rather than a quarter of the pixel value. Applied at startup and on each `SetBrightness` ([D32](decisions.md#d32-brightness-is-a-setting-and-the-desktop-simulates-the-backlight)). |
| Seed | OS entropy, or `--seed <n>`. |

Development flags:

| Flag | Effect |
|---|---|
| `--seed <n>` | Fixed random seed |
| `--record <file>` | Write the seed, the starting settings, and every input event to a recording (format in [testing.md](testing.md#recording-and-replay)) |
| `--replay <file>` | Play a recording back in real time instead of reading the mouse |
| `--time-scale <x>` | Run the clock `x` times faster, for example to watch a 5-minute timer finish in 30 s. Affects only the desktop clock. |
| `--band-rows <n>` | Draw each frame in strips of `n` rows through `render::Band`, as a low-memory board would ([ui.md](ui.md#banded-rendering)) |

macOS setup: `brew install sdl2`. On Apple Silicon the linker may not find Homebrew's SDL2; if so, add `export LIBRARY_PATH="${LIBRARY_PATH:+$LIBRARY_PATH:}$(brew --prefix)/lib"` to your shell profile. Linux support is deferred; it will need SDL2 and ALSA (`libasound2-dev`) from the distribution's package manager, and serial-port permissions for flashing the device.

## M5Stack CoreS3 Lite (planned `wade-cores3`)

### Hardware

| Part | Role in Wade | Notes |
|---|---|---|
| ESP32-S3: dual-core Xtensa LX7 at 240 MHz, 512 KB internal SRAM, 16 MB flash, 8 MB PSRAM | Runs everything | Xtensa requires Espressif's Rust toolchain |
| ILI9342C 320×240 IPS LCD over SPI | Display | |
| Capacitive touch controller on I²C | Touch | Shares the internal I²C bus. Its interrupt line is believed to be routed through the AW9523B (unverified). |
| AXP2101 PMIC | Power to the display and peripherals; battery and charging status | Must be configured before the display works. The backlight is believed to be one of its LDO outputs (DLDO1 in M5Unified), which would make brightness a PMIC voltage setting (unverified). |
| AW9523B IO expander on I²C | Reset and enable lines for the display, touch, and audio | Must be configured before those parts work |
| AW88298 amplifier over I²S, 1 W speaker | Chime | |
| LTR-553ALS proximity and ambient-light sensor | M6: wake on approach, possibly auto-brightness | |
| BM8563 real-time clock | Future: wall-clock time, timed wake | |
| 200 mAh battery | M6 | |
| BMI270 IMU, BMM150 magnetometer, GC0308 camera, ES7210 microphone codec | Unused | |

The device has no vibration motor, so there is no vibration effect.

M5Stack sells cut-down CoreS3 variants that drop some of these parts. The bring-up's first job is to confirm this unit has the parts listed above, especially the proximity sensor and battery that M6 depends on. If it does not, the project moves to the standard CoreS3, which the rest of this document also describes.

M5Stack's C++ library M5Unified is the reference for the board's initialization sequences (PMIC rails, IO expander pins, display and audio setup). Port only the parts Wade needs, and record them in `docs/hardware-notes.md` during the hardware bring-up.

### Firmware stack

| Concern | Choice |
|---|---|
| HAL and runtime | `esp-hal` with Embassy, `no_std`. Scaffold the crate with `esp-generate` and keep the ecosystem crate versions it selects, since they must match each other. |
| Flashing and serial monitor | `espflash`, configured as the cargo runner, so `cargo run --release` flashes and shows logs |
| Logging | `log` with `esp-println` |
| Panics | `esp-backtrace`, which prints a backtrace over serial |
| Display driver | `mipidsi` (which supports the ILI9342C), or a small custom driver if an async DMA flush is needed |
| Shared I²C bus | `embassy-embedded-hal` shared-bus wrappers, because the touch controller, PMIC, IO expander, and proximity sensor share one bus |
| Heap | None until Wi-Fi (M7) requires one, then `esp-alloc`, confined to the platform crate |
| Wi-Fi (M7) | `esp-radio` with `embassy-net` |

### Tasks

| Task | Responsibility | Talks to the app task through |
|---|---|---|
| App | Owns `App`. Waits for an event or the next deadline, handles it, renders into the framebuffer, and flushes it to the display. | (it is the app task) |
| Touch | Reads the touch controller and sends `Touch` events | Event channel |
| Audio | Receives chime requests and plays the chime over I²S | Audio channel |
| Power (M6) | Reads the PMIC and proximity sensor | Event channel |
| Network (M7) | Wi-Fi connection and weather requests | Network channel, event channel |

Channels are fixed-capacity `embassy-sync` channels. Each producing task stamps its events with the current time when it creates them.

There is one event channel into the app task, but one channel per consuming task out of it. An `embassy-sync` channel delivers each message to only one receiver, so a shared effect channel would let the network task receive a chime. The app task routes each effect to its consumer, or carries it out directly when it is cheap (brightness and display power over I²C, for example).

The app task never waits on an outgoing channel. It uses `try_send`, and if a consumer's channel is full the effect is dropped and logged. Waiting would stall touch handling and rendering behind a chime that is still playing.

App task loop, in outline:

```rust
loop {
    let event = match app.next_deadline() {
        Some(deadline) => match select(events.receive(), Timer::at(to_embassy(deadline))).await {
            Either::First(event) => event,
            Either::Second(()) => Event { at: now(), kind: EventKind::Deadline },
        },
        None => events.receive().await,
    };
    let output = app.handle(event);
    for effect in output.effects {
        route(effect); // try_send to the consumer's channel; drop and log if full
    }
    if output.redraw {
        let view = app.view();
        wade_core::render::draw(&view, &mut framebuffer).ok();
        display.flush(&framebuffer).await; // or only render::damage(&last_view, &view); see ui.md
        last_view = view;
    }
}
```

Touch input: if the touch controller's interrupt line is usable (on this board it may be routed through the IO expander), the touch task waits on it. Otherwise it polls at 50 Hz while the display is on. Either way, polling stays inside the platform and the core sees only events.

Memory: the framebuffer is 153,600 bytes. It fits in internal SRAM today, but in M7 the Wi-Fi stack, its heap, and TLS all need internal RAM too, and moving the framebuffer to PSRAM then would reopen the flush-time measurements. The hardware bring-up therefore makes the placement decision with M7's needs in mind: it measures flush time from both internal SRAM and PSRAM, and records the choice and the reasoning in `docs/hardware-notes.md`. Drawing in strips ([ui.md](ui.md#banded-rendering)) avoids the full framebuffer if internal RAM runs short.

### Boot sequence

1. Initialize clocks, the I²C bus, and logging.
2. Configure the AXP2101 power rails.
3. Configure the AW9523B: release resets and enable the display, touch, and amplifier.
4. Initialize the display over SPI, then touch, then audio.
5. Seed the PRNG from the hardware random number generator and create `App`.
6. Spawn tasks and render the first frame.

### Toolchain setup

```sh
cargo install espup espflash esp-generate
espup install           # installs Espressif's Xtensa Rust toolchain
. ~/export-esp.sh       # sets toolchain environment variables; run in each new shell
```

If a bad firmware image stops the USB connection from working, the board can be put into download mode with its reset button. See M5Stack's CoreS3 Lite documentation for the exact procedure.

## CYD stand-in (`wade-cyd`)

A classic-ESP32 board with a 2.8" 320×240 resistive touchscreen, used until the CoreS3 Lite arrives ([D30](decisions.md#d30-the-cyd-stands-in-until-the-cores3-arrives)). It shares the CoreS3's toolchain, HAL, runtime, task layout, and app loop; the boards differ in the parts below. Pins, init sequences, and measurements are in [hardware-notes.md](hardware-notes.md#cyd-esp32-2432s028r-single-usb-c).

| Concern | On the CYD |
|---|---|
| Display | ILI9341 over SPI with DMA at 40 MHz. No room for a framebuffer beside Wi-Fi, so frames are drawn in 40-row bands ([ui.md](ui.md#banded-rendering)), one drawn while the last is sent. |
| Touch | XPT2046, resistive. The touch task waits for the controller's interrupt line, then samples at about 100 Hz until it clears. `wade-firmware` filters the samples and maps them to the screen with fixed calibration constants, turning them into Down, Move, and Up. |
| Audio | Amp on a GPIO; the chime as PWM tones with a hardware fade. Needs a speaker plugged into the board. |
| Settings | Stored in the partition table's NVS data partition (24 KB in espflash's default table) through `wade-firmware`'s settings store, which uses `sequential-storage` to spread writes across the partition. A missing or unusable partition is logged and Wade starts with the defaults. |
| Brightness | PWM on the backlight pin |
| Power | No PMIC or IO expander to configure; no battery, proximity sensor, or RTC, so M6 does not apply |

Build with `--features measure` to log frame times, touch latency, and uptime ([hardware-notes.md](hardware-notes.md#measurements)).

Toolchain: `espup` ships Rust 1.97 for Xtensa, so the workspace's `rust-version` is 1.97 and the firmware builds `wade-core` as is. CI pins the same toolchain version.

