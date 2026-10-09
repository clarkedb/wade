# Contributing

Wade welcomes fixes, new features, and new hardware ports. CoreS3 Lite and CYD
are the currently supported boards; Wade's shared Rust core is independent of
their chips and peripherals.

## Getting started

Run the [desktop simulator](README.md#setup) first. Read the relevant
[design docs](docs/README.md) before changing behavior, and update them when
your change makes them wrong. Keep Wade's behavior and look true to his
[character](docs/character.md).

Keep behavior, layout, and state transitions in the shared core. Put hardware
access in the platform and reusable device logic in the shared firmware code.
The core and shared firmware stay `no_std` without a heap allocator.

Send focused pull requests with conventional-commit titles. Include what
changed and how you checked it. Run the [local checks](docs/testing.md#checks);
changes to behavior or drawing also need a desktop run. Review intentional
snapshot changes, and build and test hardware changes on the device.

## New hardware

Have another device in mind? [Open a hardware proposal](https://github.com/clarkedb/wade/issues/new?template=new-hardware.md).
Include the exact model and revision, chip, available RAM and flash, display
and touch controllers, wiring or schematics, and a link to the board's docs.
Say which features it has and whether you have one available to test.
Variants sharing a chip can still need different firmware.

### Minimum hardware

These are the requirements for the current Wade UI and core. A different chip
or display controller is welcome; it needs its own platform implementation.

| Part | Requirement |
|---|---|
| Processor | A Rust toolchain that can build the shared core, with drivers for the device's peripherals. ESP32 is not required. |
| Display | A 320×240 landscape logical canvas. The core draws Rgb565 pixels; the platform sends them to the panel, converting or scaling if needed. A smaller logical canvas needs layout work. |
| Input | A touchscreen or another pointer input that produces Down, Move, and Up in logical screen coordinates. Physical buttons alone need navigation work. |
| Clock | Monotonic millisecond timestamps and a way to wake for the core's next deadline. A wall-clock RTC is not required. |
| Memory | Enough RAM for the core, drivers, stacks, and display buffers, and enough program storage for firmware and boot code. Measure the port's actual use. |
| Seed | A seed for the core's random generator. A hardware random generator is optional. |

A full 320×240 Rgb565 framebuffer takes 153,600 bytes. Banded rendering can
use much less: one row takes 640 bytes, and the current ports use two 40-row
buffers totaling 51,200 bytes. Those figures cover pixels only; stacks,
drivers, audio, and other state need additional RAM. There is no established
universal RAM, flash, or CPU-speed minimum. The CYD's 4 MB flash and lack of
PSRAM are tested configurations, not requirements for every port.

Wi-Fi, a battery, proximity sensors, PSRAM, cameras, and microphones are not
required for today's Wade. Networking and power features are planned; see
the [roadmap](docs/roadmap.md).

### Feature coverage

A first bring-up can run silently and start with default settings. Supporting
the complete current experience also needs:

- Persistent storage for saved settings, with safe recovery from missing or
  corrupt data and settings preserved across compatible updates.
- A speaker or tone-capable buzzer for the timer chime.
- Display brightness control.

If hardware omits a feature, describe that in the proposal. Any reduced feature
set needs corresponding UI support so a visible control still does what it
promises; silently ignoring effects is not a finished port.

### Adding a supported device

1. Prove the display, input, and supported brightness and audio controls on the
   real hardware. Record pin assignments, initialization, orientation, and
   any touch calibration.
2. Connect the platform to the [core API](docs/architecture.md#core-api):
   timestamp events, handle effects, honor deadlines, and redraw when asked.
   Supply the board and build identity for About. Shared behavior changes
   must also work and be tested on desktop.
3. Run the device through Buddy, Timer, both Settings pages, and About. Check
   touch targets and colors, timer completion from other screens, chime,
   brightness, and settings after restart. Match the existing port criteria:
   at least 20 fps while animating, touch response under 100 ms, and a one-hour
   run without panics or more than a second of timer drift.
4. Add a reproducible CI build and join the shared version and
   [release process](docs/releases.md). Provide prebuilt images, checksums,
   installation and recovery instructions, and a documented update path.
   Test installation and an update preserving non-default settings on the
   exact board. Identify someone who can repeat the hardware checks.
5. Add the tested device to the supported-board docs and installer where
   applicable. The current browser flasher supports ESP-family devices;
   other chips need an appropriate installation path. An existing board's
   firmware is not a generic image for every device with the same chip.

Keep findings in the [hardware notes](docs/hardware-notes.md). A successful CI
build alone does not establish hardware support.
