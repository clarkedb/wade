# Releases

Wade has one version across desktop and both supported boards. Releases use
`vX.Y.Z` tags and immutable GitHub release assets. Rust crates are not published.

## Versioning

Conventional Commits drive Release Please: fixes bump patch, features bump minor,
and breaking changes bump major. During `0.x`, breaking changes bump minor too.
Mark a breaking change with `!` or a `BREAKING CHANGE:` footer. A change requiring
users to discard settings or adopt an incompatible public format is breaking;
an internal refactor or a visual adjustment alone is not.

The saved-settings and recording formats have their own versions. Preserve their
older decoders when possible and describe any migration or downgrade limits in
the release notes.

## Process

Release Please maintains a PR with the next version and changelog. It updates the
product version, Cargo manifests, and all three lockfiles together. Merging that
PR explicitly creates a tag and a **draft** release, so CI can check out the tag
while the release is still a draft. A release is published only after the
checks and firmware packaging for that exact tag succeed.

The repository must allow GitHub Actions to create pull requests. The default
`GITHUB_TOKEN` cannot trigger CI from the release PR's creation or updates; run
the CI workflow manually on that branch before merging. The release workflow
uses its own job outputs to start verification, rather than relying on a
token-created tag to trigger a second workflow.

To recover a failed release build, rerun its failed jobs. Do not move an existing
tag or replace published binaries: make a new release instead.

Before merging a release PR, smoke-test the candidate on both boards: boot,
touch, timer, chime, and brightness. Set non-default settings, update with the
app-only image, then restart and confirm they survived. CI checks builds and package contents;
it cannot prove USB flashing or hardware behavior.

## USB installation

Download the ZIP matching your board from
[Releases](https://github.com/clarkedb/wade/releases) and extract it. Supported
hardware is the M5Stack CoreS3 Lite (ESP32-S3, 16 MB flash) and the single-USB-C
CYD ESP32-2432S028R (ESP32, 4 MB flash, resistive touch). Other CYD variants need
their own tested firmware, even when they share an ESP32 chip.

Each package includes a complete installation image, an app-only update image,
bootloader and partition table, browser manifests, debug ELF, build metadata,
checksums, and commands in its README. For command-line flashing, download the
[espflash 4.6.0 binary](https://github.com/esp-rs/espflash/releases/tag/v4.6.0)
for your computer, extract it, and put it on `PATH`. These prebuilt binaries
need no Rust toolchain or Python installation.

Use `--chip esp32s3` for CoreS3 Lite or `--chip esp32` for CYD. Add
`--port <port>` if needed; examples below use CYD. Close serial monitors and
connect a USB **data** cable.

First installation or a deliberate reset replaces the old firmware and settings:

```sh
espflash erase-flash --chip esp32
espflash write-bin --chip esp32 0x0 install.bin
```

To update an existing Wade installation while keeping settings:

```sh
espflash write-bin --chip esp32 0x10000 app.bin
```

Do not erase flash or write `install.bin` when preserving settings. Merged images
contain padding over the settings partition. App-only updates assume the pinned
partition layout; release notes must call out any future change requiring a full
installation. Downgrades may fall back to defaults if the older build cannot read
newer settings.

The pinned layout retains settings at `0x9000` (24 KB), PHY data at `0xf000`, and
the factory application at `0x10000`. Release packages and local `cargo run`
use the same DIO, 40 MHz flash configuration. Package with `espflash 4.6.0` after
building the board in release mode:

```sh
scripts/check-version.sh
scripts/package-firmware.sh cyd
scripts/package-firmware.sh cores3
```

These shell commands run a host-only Rust tool with the pinned workspace
toolchain. It parses manifests and lockfiles, validates images, and writes
manifests, checksums, and ZIPs. Packaging needs a built board ELF and the pinned
espflash; it does not flash a connected device. Use `--output <directory>` to
choose a package destination instead of `dist`.

Packaging checks the generated flash headers and partition table against the
board configuration. In `espflash.toml`, size and frequency values use units
such as `16MB` and `40MHz`; invalid values can silently select espflash defaults.

## USB recovery

The CYD uses a CH340 serial bridge; the tested macOS board needs no driver. If
Windows does not show a serial port, use the
[WCH CH340 driver](https://www.wch.cn/downloads/ch341ser_exe.html). On Linux,
check access to the serial device and your distribution's serial-port group.
If automatic reset fails on a board with BOOT and RESET buttons, hold BOOT,
press and release RESET, then release BOOT once the flasher connects.

CoreS3 Lite uses native USB. Hold its reset button until the green indicator
lights (about three seconds), then release it to enter download mode. Select
the newly appearing serial port; the name may change after a reset. See
[M5Stack's CoreS3 Lite instructions](https://docs.m5stack.com/en/core/CoreS3-Lite).
