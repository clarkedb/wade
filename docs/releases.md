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
app-only image, then restart and confirm they survived. Repeat an update through
the browser installer when it changes. CI checks builds and package contents;
it cannot prove USB flashing or hardware behavior.

## USB installation

Download the ZIP matching your board from
[Releases](https://github.com/clarkedb/wade/releases) and extract it. Supported
hardware is the M5Stack CoreS3 Lite (ESP32-S3, 16 MB flash) and the single-USB-C
CYD ESP32-2432S028R (ESP32, 4 MB flash, resistive touch). Other CYD variants need
their own tested firmware, even when they share an ESP32 chip.

Device not listed? New ports are welcome; the [hardware contribution guide](../CONTRIBUTING.md#new-hardware)
covers requirements and adding support.

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

## Browser installer

The [installer](https://clarkedb.github.io/wade/) offers published stable releases,
board selection, and separate first-install and update choices. Use a desktop
browser with Web Serial: Chrome, Edge, or Firefox 151 or newer. On updates, leave **Erase
device** unchecked. The installer cannot identify the exact board from its chip
alone, so choose the supported board model yourself.

Visitors need no Rust toolchain, Python, espflash, or installed app. The browser
loads ESP Web Tools and writes the firmware over USB. First installation works
without Wade already on the device; updates require a compatible Wade
installation. Some systems need a USB serial driver; see [USB recovery](#usb-recovery).

Firefox asks for permission through its Web Serial add-on gate before the port
picker appears. Safari can browse releases and download packages, but lacks
Web Serial for direct flashing. See [Mozilla's Web Serial announcement](https://hacks.mozilla.org/2026/05/web-serial-support-in-firefox/)
and [browser compatibility](https://developer.mozilla.org/en-US/docs/Web/API/Web_Serial_API#browser_compatibility).

The site is static HTML, CSS, JavaScript, and firmware files hosted on GitHub
Pages, with no jQuery or frontend framework. Content and firmware download links
are in the built HTML, so readers and crawlers do not need JavaScript. JavaScript
adds USB installation and loads ESP Web Tools after a board and installation
mode are chosen.

The site build uses Node.js 22 or newer and npm. It validates firmware packages
and generates the HTML downloads and JSON catalog; no frontend
bundler or runtime server is needed. Build dependencies are a ZIP reader and a
local preview server. GitHub Actions downloads published packages through the
GitHub CLI, then deploys the static output.

The header shows a loop rendered through the core's normal event loop: idle
blinks and glances, then a tap makes Wade Happy. Click Wade, or focus him and press
Space or Enter, to pause or resume. Reduced motion starts with the neutral-face
snapshot; visits without JavaScript show it too. Native controls, keyboard focus,
linked field descriptions, and live status messages support accessible installation.

The page includes canonical and social metadata, source-code structured data,
a sitemap, and a linked JSON release catalog. Keep these URLs current if hosting
changes. Search and AI crawlers can read the same visible content and links as
visitors. GitHub project Pages lives under `/wade/`; crawler rules belong at the
host's `/robots.txt`, not `/wade/robots.txt`. The sitemap can be submitted to
search engines after deployment. Metadata does not guarantee indexing.

To regenerate the animation, use the pinned Rust toolchain and FFmpeg:

```sh
cargo run -p wade-core --example site-animation --features harness -- dist/site-animation
ffmpeg -y -framerate 25 -i dist/site-animation/frame-%03d.png -filter_complex '[0:v]split[a][b];[a]palettegen=reserve_transparent=0[p];[b][p]paletteuse=dither=none' -loop 0 site/public/wade.gif
```

GitHub Pages must use GitHub Actions as its publishing source. The Installer
workflow rebuilds the site after a release publishes or the page changes, and
can be run manually. Firmware, manifests, and downloads share the site's origin.
Before the first release, the page explains that no firmware is available yet.

The workflow downloads board ZIPs from published stable GitHub releases,
skipping drafts and prereleases. The site builder checks and extracts those
packages, then generates `releases.json`, sorted newest first. The browser reads
that catalog and selects the image for the chosen version, board, and install
or update mode. Branch commits alone do not add releases to the list.

For a local preview after packaging both boards:

```sh
npm ci --prefix site
npm --prefix site run build
npm --prefix site run preview
```

Local preview uses the ZIPs already in `dist/`; building the site does not compile
firmware. Rebuild and repackage both boards when their source changes. The local
version label comes from package metadata, so it can still say `0.1.0` while
About identifies a development commit. Localhost is for development; visitors
use the hosted HTTPS site.

About, startup logs, and desktop `--version` identify the build. Only a clean
checkout of the matching release tag is labeled a release; other builds include
a commit identifier and development status, plus a marker for local changes.

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
