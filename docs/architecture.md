# Architecture

## The boundary

The project splits into one core crate and one crate per platform. The core decides what Wade does; a platform connects the core to real inputs, a real clock, and real outputs.

```text
 Platform (wade-desktop, wade-cores3)          Core (wade-core)
┌──────────────────────────────┐             ┌────────────────────┐
│ clock, touch, sensors        │── Event ───►│                    │
│ deadline timer               │── Event ───►│        App         │
│                              │             │                    │
│ scheduler                    │◄─ deadline ─│  next_deadline()   │
│ audio, storage, network      │◄─ Effect ───│  handle() → Output │
│ display                      │◄─ pixels ───│  view() + draw()   │
└──────────────────────────────┘             └────────────────────┘
```

| The core owns | The platform owns |
|---|---|
| All behavior and state transitions | Reading the clock and timestamping events |
| Screen layout, hit-testing, and gesture recognition | Reading raw input from touch and sensors |
| What to draw, and the drawing code | Sending pixels to a display |
| Deciding when it next needs to run | Sleeping, and waking at that time |
| Requesting side effects | Carrying out side effects: audio, storage, network |

## Rules for `wade-core`

`wade-core` is `#![no_std]` and does not use `alloc`. It never reads a clock, sleeps, blocks, performs I/O, touches hardware, keeps global mutable state, or generates its own randomness. Everything it knows arrives as a function argument, and everything it wants done leaves as a return value.

Its dependencies are limited to `embedded-graphics` (drawing), `heapless` (fixed-capacity collections), and `libm` (math functions such as `sinf`, which `core` does not provide). A bare-metal build check enforces the `no_std` rule (see [testing.md](testing.md#checks)).

## Crates

```text
wade/
├── Cargo.toml       workspace: wade-core, wade-desktop; excludes wade-cores3 and spikes/
├── wade-core/       no_std library: behavior, layout, drawing
├── wade-desktop/    std binary: simulator window, audio, record and replay
├── wade-cores3/     no_std firmware for the CoreS3 Lite
├── spikes/          throwaway experiments, such as the hardware spike
├── .github/         CI workflows (see testing.md)
└── docs/
```

Platforms depend on the core. The core depends on no platform.

`wade-cores3` is excluded from the root workspace because it needs Espressif's Xtensa toolchain and its own build target; inside the workspace it would break `cargo test` at the root. It has its own `rust-toolchain.toml` and `.cargo/config.toml` and depends on `wade-core` by path.

Any package that sits under the root directory but is not a workspace member must be listed in the root `Cargo.toml`'s `workspace.exclude`, or Cargo refuses to build it. This applies to `wade-cores3` and to everything under `spikes/`:

```toml
[workspace]
members = ["wade-core", "wade-desktop"]
exclude = ["wade-cores3", "spikes"]
```

Module layout inside `wade-core`:

```text
wade-core/src/
├── lib.rs       public API
├── time.rs      Instant
├── event.rs     Event, EventKind, Touch
├── app.rs       App, Output, Effect, screen routing
├── input.rs     TouchTracker: raw touch samples → taps
├── layout.rs    screen geometry shared by drawing and hit-testing
├── rng.rs       seeded pseudo-random number generator
├── settings.rs  Settings and their stored form
├── sound.rs     tone sequences (the chime)
├── character/   Wade: state, expressions, animation, pose rig
├── timer.rs     TimerState machine
├── view.rs      View types
└── render/      drawing: palette, face, screens, widgets, digits
```

## Time

```rust
/// Milliseconds since the platform started. Monotonic. A u64 never wraps in practice.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instant(u64);

impl Instant {
    pub const fn from_millis(ms: u64) -> Self { Instant(ms) }
    pub const fn as_millis(self) -> u64 { self.0 }

    /// Time elapsed since `earlier`, or zero if `earlier` is later.
    pub fn saturating_since(self, earlier: Instant) -> Duration {
        Duration::from_millis(self.0.saturating_sub(earlier.0))
    }
}

impl core::ops::Add<Duration> for Instant {
    type Output = Instant;
    fn add(self, d: Duration) -> Instant {
        // Saturates rather than overflowing, so no timestamp can make `handle` panic.
        let ms = u64::try_from(d.as_millis()).unwrap_or(u64::MAX);
        Instant(self.0.saturating_add(ms))
    }
}
```

`Duration` is `core::time::Duration`. There is deliberately no `Instant - Instant` operator, because `Duration` cannot be negative and a subtraction that panics when its inputs arrive in the wrong order is a latent bug.

Time enters the core only through `Event::at`. The core keeps `now` as the latest timestamp it has seen. If a platform delivers events slightly out of order (two tasks stamping events concurrently, for example), the core clamps: `now = max(now, event.at)`.

The core has no wall-clock time (dates, time of day, time zones). No feature before the future work in [roadmap.md](roadmap.md) needs it. When the device resets, time restarts at zero and all state except saved settings is lost.

## Events

```rust
pub struct Event {
    pub at: Instant,
    pub kind: EventKind,
}

pub enum EventKind {
    /// A raw touch sample in logical screen coordinates (320×240, origin top-left).
    Touch(Touch),
    /// A key press from the desktop keyboard, for trying out Wade's looks.
    Key(Key),
    /// A deadline requested through `App::next_deadline` has been reached.
    /// Carries no other meaning. May arrive late, or more often than requested.
    Deadline,
}

pub struct Touch {
    pub phase: TouchPhase,
    pub point: embedded_graphics::geometry::Point,
}

pub enum TouchPhase {
    Down,
    Move,
    Up,
}

pub enum Key {
    Digit(Digit), // 0–9: show that expression
    Z,            // put Wade to sleep
    P,            // switch Wade's eye style
}
```

Events report raw facts, never interpretations. A platform never sends "open the timer" or "go back"; it sends touch samples, and the core decides what they mean because the core owns the layout. The platform tracks one touch point and ignores additional fingers. Key events are raw too: the platform reports which key, and the core decides what it does. Only the desktop has keys, and they act only on the Buddy screen ([D22](decisions.md#d22-desktop-keys-are-core-events)).

`App::handle` processes an event in two steps. First it advances `now` to the event's timestamp, applying every timed transition that became due along the way (an expression expiring, a timer finishing) in chronological order across all features. Then it applies the event itself. For a `Deadline` event, the first step is the whole job.

Chronological order across features matters because features interact: a timer finishing switches the screen, which changes whether Wade is visible and which deadlines apply, and behavior draws from one RNG. So `App` does not simply call each feature's `advance(now)` in turn. It loops:

```text
advance_to(target):
    loop:
        t = the earliest pending transition across all features
        if t is None or t > target: break
        now = t
        apply the transitions due at t, in a fixed feature order
    now = target
```

The fixed feature order breaks ties between transitions due at the same instant, so the result never depends on iteration order.

## Core API

Signatures only:

```rust
pub struct App { /* private */ }

impl App {
    pub fn new(now: Instant, seed: u64, settings: Settings) -> App;

    pub fn handle(&mut self, event: Event) -> Output;

    /// The earliest time the core needs a `Deadline` event, or `None` if nothing
    /// visible will change without input. Always later than the last handled event.
    pub fn next_deadline(&self) -> Option<Instant>;

    /// What is on screen, as plain data, as of the last handled event.
    pub fn view(&self) -> View;
}

pub struct Output {
    /// Side effects for the platform to carry out, in order. If more than four
    /// are produced by one event, the extras are dropped (never a panic).
    pub effects: heapless::Vec<Effect, 4>,
    /// The view may have changed since the last render. A false positive costs
    /// one redundant frame; a false negative is a bug.
    pub redraw: bool,
}

pub enum Effect {
    /// Play the timer-finished chime, defined as a tone sequence in `wade_core::sound::CHIME`.
    /// Emitted when the timer finishes and repeated while it stays Done (see ui.md).
    Chime,
}

// In wade_core::render. Drawing is a function of plain data.
pub fn draw<D>(view: &View, target: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>;
```

Separating `view` from `draw` lets behavior tests assert on data (which screen, which expression, which timer digits) while drawing is tested separately with image snapshots.

Every platform runs the same loop:

```text
app = App::new(now(), seed, load_settings())
draw(app.view())
loop:
    event = the next input event, or a Deadline event if app.next_deadline() passes first
    output = app.handle(event)
    carry out output.effects
    if output.redraw: draw(app.view())
```

## State

```rust
pub struct App {
    now: Instant,
    screen: Screen,      // which screen is displayed and receives touch
    wade: Wade,          // character state; kept on every screen
    timer: TimerState,   // keeps running on every screen
    touch: TouchTracker, // turns raw touch samples into taps
    rng: Rng,            // seeded PRNG for behavior
    settings: Settings,  // eye style, colors, chime, timer duration
}

enum Screen {
    Buddy,
    Launcher,
    Timer,
    Settings,
}
```

`Screen` records only what is displayed. Long-lived state lives in its own `App` fields, so navigation never discards it: a running timer keeps running while Wade is shown, and Wade's state survives a visit to the timer.

Each feature module follows one pattern: a state type with `next_transition()` to report its next scheduled change, a method to apply the transitions due at a given instant, and, for features with a screen, a tap handler and a `view()` method. `App` composes them with the loop in [Events](#events); its `next_deadline` is the earliest of theirs.

Touches are global input. Any tap, on any screen, counts as activity for features that care about inactivity (Wade's idle activities in M4, display sleep in M6).

### Hidden features

A feature whose screen is not visible still keeps time but costs nothing. While Wade is hidden, his schedule (blinks and other idle motions, and idle activities from M4) keeps moving forward, but he requests no deadlines and never sets `redraw`. His transitions are applied in order when the next event arrives, like any others that fall due between events, so while he is hidden and nothing else is scheduled, the device sleeps until the next input ([D25](decisions.md#d25-hidden-features-request-no-deadlines)). A blink that falls due while he is hidden is skipped and the next one is scheduled; an idle activity never starts while he is hidden. When the Buddy screen returns, Wade resumes from his current schedule with no burst of catch-up animation. The same rule covers any future feature with animation.

## Deadlines and frames

Deadlines replace a periodic tick. Each feature reports when it will next change without input: Wade's next blink or the next frame of a running animation, the moment the timer ends, the next second at which the timer's digits change, the next chime repeat. `App::next_deadline` returns the earliest.

The platform guarantees a `Deadline` event at or after that time. It may arrive late (after a slow frame), and extra `Deadline` events may arrive (after a spurious wakeup). The core must produce the same result either way: behavior depends on elapsed time, never on how many events arrived. An invariant test checks this (see [testing.md](testing.md#invariants)). One bound: past a gap of `STALL_LIMIT` (an hour), Wade restarts his idle schedule instead of replaying it ([D20](decisions.md#d20-a-gap-over-an-hour-restarts-wades-idle-schedule)).

While something is moving, the moving feature requests a deadline one frame ahead. The frame interval is `FRAME = 33 ms`, about 30 frames per second. When nothing is moving, the only deadlines are scheduled changes, so a still screen costs no CPU time. Animations are computed from elapsed time, so a slow platform drops frames rather than slowing the animation down.

## Randomness

Wade's motion (blinks, glances, squints, tears, twitches) and behavior (which idle activity starts, the hubris beat) are random. The core uses a small seeded pseudo-random generator (xorshift64*, about ten lines), in two streams from one seed: motion's lives in Wade, behavior's in `App` ([D21](decisions.md#d21-motion-and-behavior-draw-from-separate-random-streams)). The platform supplies the seed: the hardware random number generator on the device, and OS entropy or a `--seed` flag on desktop. Tests use fixed seeds, and recordings store the seed so replays are exact.

## Planned additions

New event kinds, effects, and constructor arguments are added only when a milestone needs them.

| Milestone | Events | Effects | Other |
|---|---|---|---|
| M2 Timer | none | `Chime` | none |
| M3 Device port | none | none | `render::damage`, only if the spike's flush measurements require it (see [ui.md](ui.md#rendering)) |
| M5 Settings | none | `SaveSettings(Settings)`; `SetBrightness(u8)` once the device can use it ([D27](decisions.md#d27-brightness-waits-for-the-device)) | `App::new` takes loaded `Settings` |
| M6 Power | `Power(PowerStatus)`, `Proximity(Proximity)` | `DisplayPower(bool)` | none |
| M7 Weather | `Weather(WeatherUpdate)` | `FetchWeather` | none |
