# Testing

## Layers

| Layer | Location | Checks | Tools |
|---|---|---|---|
| Behavior | `wade-core` tests | State transitions, deadlines, and effects, asserted on `View` and `Output` | `cargo test` |
| Invariants | `wade-core` tests | Properties that must hold for every event sequence | `proptest` (dev-dependency) |
| Snapshots | `wade-core` tests | Rendered pixels for known views | `png` (dev-dependency), checked-in golden images |
| Replay | `wade-core` tests | Real sessions recorded on desktop still behave the same | Recording files in `wade-core/tests/recordings/` |
| Device | Manual | Hardware integration | The checklist for each milestone in [roadmap.md](roadmap.md) |

`wade-core` declares `#![cfg_attr(not(test), no_std)]`, so its unit tests can use the standard library while the library itself stays `no_std`.

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
| Inserting extra `Deadline` events anywhere in a sequence does not change the view at any of the original events | Platforms may wake late or spuriously. Behavior must depend on elapsed time only. |
| `next_deadline()` is `None` or strictly later than the last handled event | A deadline in the past or present would make the platform loop spin. |
| `handle` never panics, including for timestamps that go backwards | Timestamps from different tasks can arrive slightly out of order. |
| Each timer completion emits exactly one `Chime` | Duplicate or missing chimes are easy to introduce when deadlines arrive late. |

These are property tests: `proptest` generates random event sequences and checks each invariant.

## Snapshots

A snapshot test builds a `View`, draws it into an in-memory 320×240 test framebuffer, and compares the result with `wade-core/tests/snapshots/<name>.png`. On a mismatch, the test writes `<name>.actual.png` next to the golden image and fails. Running `UPDATE_SNAPSHOTS=1 cargo test` rewrites the golden images, and the diff is reviewed in version control.

Snapshot cases include Wade in each expression at rest, Wade mid-blink, and each timer state. Golden images are generated on desktop.

## Recording and replay

`wade-desktop --record <file>` writes the seed and every input event. `Deadline` events are not recorded; replay regenerates them from `next_deadline`, so recordings stay valid when animation timing changes.

```text
wade-events 1
seed 8127364512
1000 down 160 110
1080 up 160 110
5400 down 290 210
5460 up 290 210
```

Each line after the header is a timestamp in milliseconds, a touch phase (`down`, `move`, or `up`), and x and y in display coordinates. The parser and writer live behind the `harness` feature.

Recordings of interesting sessions go in `wade-core/tests/recordings/`. A replay test runs each one through the harness and checks that it completes without panicking, plus any assertions written for that recording, such as the final screen or a snapshot of the final frame.

## Checks

Run before every commit, or in CI:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build -p wade-core --target thumbv7em-none-eabihf   # proves wade-core builds without std
```

The last check needs `rustup target add thumbv7em-none-eabihf` once. It proves `wade-core` is `no_std` but not that it avoids `alloc`, because `alloc` exists on that target too. The no-`alloc` rule is enforced by never declaring `extern crate alloc` in `wade-core`; a `grep` for that line can be added to the checks.

The firmware builds separately: `cd wade-cores3 && cargo build --release`, which requires the Espressif toolchain.

## Device testing

Device testing is manual. Each milestone that touches the device lists its checks in [roadmap.md](roadmap.md). Automated tests on the hardware itself are out of scope.
