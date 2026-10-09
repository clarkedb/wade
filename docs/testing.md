# Testing

## Layers

| Layer | Location | Checks | Tools |
|---|---|---|---|
| Behavior | `wade-core` tests | State transitions, deadlines, and effects, asserted on `View` and `Output` | `cargo test` |
| Invariants | `wade-core` tests | Properties that must hold for every event sequence | `proptest` (dev-dependency) |
| Snapshots | `wade-core` tests | Rendered pixels for known views | `png` (dev-dependency), checked-in golden images |
| Replay | `wade-core` tests | Real sessions recorded on desktop still behave the same | Recording files in `wade-core/tests/recordings/` |
| Device | Manual | Hardware integration | The checklist for each milestone in [roadmap.md](roadmap.md) |

`wade-core` declares:

```rust
#![no_std]

#[cfg(any(test, feature = "harness"))]
extern crate std;
```

so its unit tests and the harness can use the standard library while the library itself stays `no_std`. Linking `std` this way, rather than dropping `no_std` under those configurations, keeps the std prelude out of core modules: the harness imports what it uses explicitly.

Tests in `wade-core/tests/` (snapshots, replays, most behavior tests) are separate crates that link the library built *without* `cfg(test)`, so they cannot see anything gated on `test`. They get the harness through a dev-dependency of the crate on itself:

```toml
# wade-core/Cargo.toml
[features]
harness = []

[dev-dependencies]
wade-core = { path = ".", features = ["harness"] }
```

Because `wade-desktop` enables `harness`, Cargo's feature unification means `cargo test --workspace` and `cargo clippy --workspace` always build `wade-core` with `std`. A stray `std` use in the library would pass both. The bare-metal build in [Checks](#checks) is therefore the only guard for `no_std` and must stay in the checks.

## Harness

The harness drives `App` the same way a platform does. It lives in `wade-core` behind a `harness` feature (which enables `std`), so both the core's tests and the desktop's replay mode can use it.

```rust
pub struct Harness {
    pub app: App,
    pub effects: Vec<Effect>, // every effect emitted so far
}

impl Harness {
    pub fn new(seed: u64) -> Self;

    /// Like `new`, but starting with `settings`, as if the platform loaded them.
    pub fn with_settings(seed: u64, settings: Settings) -> Self;

    /// Deliver a Deadline event at every deadline the app requests up to and
    /// including `t`, exactly as a platform would, then deliver one at `t`.
    pub fn run_until(&mut self, t: Instant);

    /// Run until `t`, then deliver a Down and an Up at `point`.
    pub fn tap(&mut self, t: Instant, point: Point);

    /// The current view, which must be the Buddy screen.
    pub fn buddy(&self) -> BuddyView;

    /// The current view, which must be the Timer screen.
    pub fn timer(&self) -> TimerView;
}
```

Example:

```rust
#[test]
fn tap_makes_wade_happy_for_two_seconds() {
    let mut h = Harness::new(SEED);
    h.tap(ms(1_000), layout::WADE_CENTER);
    assert_eq!(h.buddy().expression, Expression::Happy);

    h.run_until(ms(2_999));
    assert_eq!(h.buddy().expression, Expression::Happy);

    h.run_until(ms(3_000));
    assert_eq!(h.buddy().expression, Expression::Neutral);
}
```

Behavior tests assert on discrete values such as the screen, the expression, and the timer's digits. They do not compare `Pose` values, which are continuous; drawing is covered by snapshots.

## Invariants

| Invariant | Why |
|---|---|
| Inserting extra `Deadline` events anywhere in a sequence, or delivering requested ones late by up to `STALL_LIMIT` ([D20](decisions.md#d20-a-gap-over-an-hour-restarts-wades-idle-schedule)), does not change the view at any of the original events. One exception: a gap longer than `STALL_LIMIT` with nothing requested, possible only while Wade is hidden, may change his motion after it, never discrete state ([D25](decisions.md#d25-hidden-features-request-no-deadlines)). | Platforms may wake late or spuriously. Behavior must depend on elapsed time only. |
| `next_deadline()` is `None` or strictly later than the last handled event | A deadline in the past or present would make the platform loop spin. |
| `handle` never panics, including for timestamps that go backwards or near `u64::MAX` | Timestamps from different tasks can arrive slightly out of order. |
| Each timer completion emits between 1 and 10 `Chime` effects (none with the chime off), the first at `ends_at`, repeats exactly 2 s apart, none after Dismiss | Duplicate, missing, or runaway chimes are easy to introduce when deadlines arrive late. |
| A tap is never delivered to a screen other than the one its `Down` landed on | Screens can change mid-touch when the timer finishes. |
| No `SaveSettings` repeats the settings last saved, and once input stops for 2 s the last saved settings are the current ones | A change left unsaved is lost at the next restart, and a needless save wears flash. |
| From M3, if `render::damage` exists: every pixel that differs between drawing `prev` and `next` lies inside `damage(prev, next)` | A too-small rectangle leaves stale pixels on the device, which desktop never shows. |

These are property tests: `proptest` generates random event sequences, from random starting settings, and checks each invariant.

## Snapshots

A snapshot test builds a `View`, draws it into an in-memory 320×240 test framebuffer, and compares the result with `wade-core/tests/snapshots/<name>.png`. On a mismatch, the test writes `<name>.actual.png` next to the golden image and fails. Running `UPDATE_SNAPSHOTS=1 cargo test` rewrites the golden images, and the diff is reviewed in version control.

Snapshot cases include each expression at rest with its accent, and a glance, in both eye styles; each colored accent in mono; and Wade asleep and mid-blink. These draw his face alone. Whole-screen cases cover Buddy at startup, once awake, and with the apps button pressed; the Launcher with each button pressed; the Timer screen in each state with each button pressed; and both Settings pages, with every setting changed and each button pressed, including the page arrows and Info tile. Golden images are generated on desktop.

## Recording and replay

`wade-desktop --record <file>` writes the seed, the settings the session started with, and every input event. `Deadline` events are not recorded; replay regenerates them from `next_deadline`, so recordings stay valid when animation timing changes.

```text
wade-events 2
seed 8127364512
settings 0100000105
1000 down 160 110 #3f9a1c02
1080 up 160 110 #b7e0442d
2400 key 3 #5d21a7c4
5400 down 290 210 #5d21a7c4
5460 up 290 210 #51c8d9e0
```

The settings are their stored form in hex. Each line after them is a timestamp in milliseconds, then either a touch phase (`down`, `move`, or `up`) with x and y in display coordinates or `key` with the key (`0`–`9`, `z`, or `p`), then a state hash. The parser and writer live behind the `harness` feature.

The state hash is taken after the event is handled. It covers the screen and Settings page, Wade's expression and sleep state (on every screen), the timer's state, duration, and digits, and every setting; M4 adds the activity. Including the eye style catches a broken P key ([D24](decisions.md#d24-eye-style-belongs-in-the-recording-hash)). It excludes `Pose` and pixels, so tuning animation curves or redrawing Wade does not invalidate recordings; snapshots cover selected rendered views. Replay checks discrete state after each recorded input and reports the first mismatch with its timestamp.

Desktop replay rejects files over 16 MiB and checks the recording before opening the window. Core replay stops after 100,000 scheduled deadlines so a timestamp far in the future cannot keep it busy indefinitely.

Version 2 added starting settings. Future milestones add power and proximity inputs (M6) and weather updates (M7). Each addition bumps the header version. The parser accepts every older version, so existing recordings keep parsing; a version 1 recording starts with the default settings. When the state hash covers more, their hashes are rewritten.

Recordings of interesting sessions go in `wade-core/tests/recordings/` as `*.events.wade` files. The suffix identifies Wade's custom event format; desktop captures use a `desktop-` prefix. The core replay tests own the fixtures. One runs every recording through the harness and checks that it completes without panicking and that every state hash matches. A recording with more to check, such as the final screen or a snapshot of the final frame, gets its own test that loads it by name, so renaming the file fails that test instead of silently skipping its checks. A behavior change that alters a hash on purpose is accepted by re-recording, or by rewriting the hashes with `UPDATE_RECORDINGS=1 cargo test` and reviewing the diff.

## Checks

Run these checks before committing Rust changes, and in CI:

```sh
scripts/check-version.sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo clippy -p wade-core -p wade-firmware --lib --target thumbv7em-none-eabihf -- -D warnings   # proves both build without std
! grep -rnE "extern[[:space:]]+crate[[:space:]]+alloc" wade-core/src wade-firmware/src           # proves neither uses alloc
```

For site changes:

```sh
npm ci --prefix site
npm test --prefix site
npm --prefix site run build
```

The bare-metal build needs `rustup target add thumbv7em-none-eabihf` once. It proves `wade-core` and `wade-firmware` are `no_std` but not that they avoid `alloc`, because `alloc` exists on that target too; the `grep` covers that.

Firmware builds separately with the Espressif toolchain: `cd wade-cores3 && cargo build --release`, and `wade-cyd` the same way.

### CI

GitHub Actions checks Rust and firmware on pushes. Site CI runs on pull requests
that change `site/` and matching pushes to `main`.

| Job | Runner | Steps |
|---|---|---|
| Workspace | `ubuntu-latest` | `sudo apt-get install -y libsdl2-dev libasound2-dev` (SDL2 and ALSA, which only `wade-desktop` needs), then the Rust checks above |
| Firmware | `ubuntu-latest` | Install the Xtensa toolchain with the `esp-rs/xtensa-toolchain` action, at the version `espup` installs locally, then `cargo fmt --check`, `cargo clippy`, and `cargo build --release` in `wade-cores3` and `wade-cyd` |
| Site | `ubuntu-latest` | Install Node.js 24, then `npm ci`, `npm test`, and `npm run build` |

A macOS job (`brew install sdl2`) is added only if something platform-specific breaks. The bring-up firmware under `bringup/` is not built in CI.

CI also checks that the product version agrees across manifests and lockfiles.
Host release-tool tests cover version drift, missing locked packages, malformed
flash configuration, moved settings partitions, and corrupt, truncated, or
oversized images. They run with the workspace tests and need no hardware or
espflash installation.
Installer tests check that HTML download links match the catalog, versions sort
numerically, and empty or rebuilt catalogs do not advertise unavailable releases.
For website changes, also check keyboard navigation, accessible field names and
descriptions, contrast, reduced motion and the animation control, 320 px reflow,
200% text size, and downloads without JavaScript. Run an accessibility scanner
on both the normal page and its loading/error states; automated checks do not
replace testing with a screen reader or testing the USB flow on hardware.

The release workflow reuses the Rust and firmware checks at the exact release
tag, packages both firmware targets, and publishes the draft only after every
check passes. About's navigation, timer interruptions, text snapshots, and banded
drawing are tested on the host. Release hardware checks are in
[releases.md](releases.md#process).

## Device testing

Device testing is manual. Each milestone that touches the device lists its checks in [roadmap.md](roadmap.md). Automated tests on the hardware itself are out of scope.

### Bring-up checklist

Every board's bring-up runs these checks and records the results in [hardware-notes.md](hardware-notes.md). Each ends in something a person can see, hear, or read in the serial log.

| Check | Method | Records |
|---|---|---|
| Board | `espflash board-info` | Chip, revision, flash, PSRAM |
| Display | Fill red, green, blue, and white, then mark three corners with colored squares | Init sequence, orientation and color-order settings, inversion |
| Transport | Redraw one unchanging frame with a 1 px border, a phase number shown as squares in a corner, for each bus option (DMA or not, each clock). Any movement or seam is a transport fault, not tearing | Which options are steady |
| Flush timing | Per frame: the bus alone, draw then send, and draw while sending | Frame time, and how it splits between bus and drawing |
| Touch | Press crosshairs near each corner, several rounds, summarizing each press; then draw dots under each touch | Axis order and direction, calibration, noise, pressure, a filter that works |
| Audio | Play the chime each way the board allows | Loudness, quality, CPU cost |
| Backlight | Step through four levels, then fade | Control method, usable levels, flicker |
| Memory | Section sizes with the planned buffers; whether a full framebuffer links and runs | Stack and heap headroom, framebuffer or bands |
