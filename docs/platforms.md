# Platforms

Both platforms run the loop in [architecture.md](architecture.md#core-api): stamp events with the current time, pass them to `App::handle`, carry out effects, redraw when asked, and wake at `App::next_deadline`.

## Desktop (`wade-desktop`)

| Concern | Implementation |
|---|---|
| Window | `embedded-graphics-simulator`: a 320×240 Rgb565 display shown at 2× scale. Requires SDL2. |
| Clock | `std::time::Instant` captured at startup; `Instant::from_millis(elapsed_ms)`. |
| Input | Mouse button down → `Touch Down`; mouse movement while held → `Touch Move`; button up → `Touch Up`. Points passed to the core must be in 320×240 display coordinates. The simulator's mouse events should already be in display coordinates (it undoes the window scale); confirm this in M1 and divide by the scale only if not. Number keys 0–9 and Z → `Key` events, which show an expression or put Wade to sleep (see [character.md](character.md#m1)). |
| Loop | The simulator offers only non-blocking event polling. The loop polls window events, then sleeps until the next deadline or for 10 ms, whichever is sooner. This polling stays inside the desktop crate. |
| Audio | `rodio`, synthesizing the tone sequence in `wade_core::sound::CHIME`. No audio files. |
| Seed | OS entropy, or `--seed <n>`. |

Development flags:

| Flag | Effect |
|---|---|
| `--seed <n>` | Fixed random seed |
| `--record <file>` | Write the seed and every input event to a recording (format in [testing.md](testing.md#recording-and-replay)) |
| `--replay <file>` | Play a recording back in real time instead of reading the mouse |
| `--time-scale <x>` | Run the clock `x` times faster, for example to watch a 5-minute timer finish in 30 s. Affects only the desktop clock. |

macOS setup: `brew install sdl2`. On Apple Silicon the linker may not find Homebrew's SDL2; if so, add `export LIBRARY_PATH="$LIBRARY_PATH:$(brew --prefix)/lib"` to your shell profile. Linux support is deferred; it will need SDL2 from the distribution's package manager, and serial-port permissions for flashing the device.

## M5Stack CoreS3 Lite (`wade-cores3`)

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

M5Stack sells cut-down CoreS3 variants that drop some of these parts. The spike's first job is to confirm this unit has the parts listed above, especially the proximity sensor and battery that M6 depends on. If it does not, the project moves to the standard CoreS3, which the rest of this document also describes.

M5Stack's C++ library M5Unified is the reference for the board's initialization sequences (PMIC rails, IO expander pins, display and audio setup). Port only the parts Wade needs, and record them in `docs/hardware-notes.md` during the hardware spike.

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

Memory: the framebuffer is 153,600 bytes. It fits in internal SRAM today, but in M7 the Wi-Fi stack, its heap, and TLS all need internal RAM too, and moving the framebuffer to PSRAM then would reopen the flush-time measurements. The hardware spike therefore makes the placement decision with M7's needs in mind: it measures flush time from both internal SRAM and PSRAM, and records the choice and the reasoning in `docs/hardware-notes.md`.

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
