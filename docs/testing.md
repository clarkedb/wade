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
| Each timer completion emits between 1 and 10 `Chime` effects, the first at `ends_at`, repeats exactly 2 s apart, none after Dismiss | Duplicate, missing, or runaway chimes are easy to introduce when deadlines arrive late. |
| A tap is never delivered to a screen other than the one its `Down` landed on | Screens can change mid-touch when the timer finishes. |
| From M3, if `render::damage` exists: every pixel that differs between drawing `prev` and `next` lies inside `damage(prev, next)` | A too-small rectangle leaves stale pixels on the device, which desktop never shows. |

These are property tests: `proptest` generates random event sequences and checks each invariant.

## Snapshots

A snapshot test builds a `View`, draws it into an in-memory 320×240 test framebuffer, and compares the result with `wade-core/tests/snapshots/<name>.png`. On a mismatch, the test writes `<name>.actual.png` next to the golden image and fails. Running `UPDATE_SNAPSHOTS=1 cargo test` rewrites the golden images, and the diff is reviewed in version control.

Snapshot cases include each expression at rest with its accent, and a glance, in both eye styles, and Wade asleep and mid-blink; these draw his face alone. Whole-screen cases cover Buddy at startup, once awake, and with the apps button pressed, the Launcher with each button pressed, and the Timer screen in each state with each button pressed. Golden images are generated on desktop.

## Recording and replay

`wade-desktop --record <file>` writes the seed and every input event. `Deadline` events are not recorded; replay regenerates them from `next_deadline`, so recordings stay valid when animation timing changes.

```text
wade-events 1
seed 8127364512
1000 down 160 110 #3f9a1c02
1080 up 160 110 #b7e0442d
2400 key 3 #5d21a7c4
5400 down 290 210 #5d21a7c4
5460 up 290 210 #51c8d9e0
```

Each line after the header is a timestamp in milliseconds, then either a touch phase (`down`, `move`, or `up`) with x and y in display coordinates or `key` with the key (`0`–`9`, `z`, or `p`), then a state hash. The parser and writer live behind the `harness` feature.

The state hash is taken after the event is handled. It covers the screen, Wade's expression and sleep state (on every screen), eye style, and the timer's state, duration, and digits; M4 adds the activity. Including eye style catches a broken P key ([D24](decisions.md#d24-eye-style-belongs-in-the-recording-hash)). It excludes `Pose` and pixels, so tuning animation curves or redrawing Wade does not invalidate recordings; snapshots cover those. Replay recomputes the hash after each event and reports the first mismatch with its timestamp. This is how "replays identically" is checked.

Recordings will need more input kinds as milestones add events: loaded settings (M5), power and proximity (M6), weather updates (M7). Each addition bumps the header version (`wade-events 2`, …). The parser accepts every older version, so existing recordings keep working.

Recordings of interesting sessions go in `wade-core/tests/recordings/` as `*.events.wade` files. The suffix identifies Wade's custom event format; desktop captures use a `desktop-` prefix. The core replay tests own the fixtures. One runs every recording through the harness and checks that it completes without panicking and that every state hash matches. A recording with more to check, such as the final screen or a snapshot of the final frame, gets its own test that loads it by name, so renaming the file fails that test instead of silently skipping its checks. A behavior change that alters a hash on purpose is accepted by re-recording, or by rewriting the hashes with `UPDATE_RECORDINGS=1 cargo test` and reviewing the diff.

## Checks

Run before every commit, and in CI:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo clippy -p wade-core --lib --target thumbv7em-none-eabihf -- -D warnings   # proves wade-core builds without std
! grep -rnE "extern[[:space:]]+crate[[:space:]]+alloc" wade-core/src           # proves wade-core does not use alloc
```

The bare-metal build needs `rustup target add thumbv7em-none-eabihf` once. It proves `wade-core` is `no_std` but not that it avoids `alloc`, because `alloc` exists on that target too; the `grep` covers that.

The firmware builds separately: `cd wade-cores3 && cargo build --release`, which requires the Espressif toolchain.

### CI

CI runs on GitHub Actions, in two jobs:

| Job | Runner | Steps |
|---|---|---|
| Workspace | `ubuntu-latest` | `sudo apt-get install -y libsdl2-dev libasound2-dev` (SDL2 and ALSA, which only `wade-desktop` needs), then every check above |
| Firmware (from M3) | `ubuntu-latest` | Install the Xtensa toolchain with the `esp-rs/xtensa-toolchain` action, then `cargo build --release` in `wade-cores3`, plus `cargo fmt --check` and `cargo clippy` there |

A macOS job (`brew install sdl2`) is added only if something platform-specific breaks. The spike under `spikes/` is not built in CI.

## Device testing

Device testing is manual. Each milestone that touches the device lists its checks in [roadmap.md](roadmap.md). Automated tests on the hardware itself are out of scope.
