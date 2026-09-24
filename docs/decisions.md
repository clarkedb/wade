# Decisions

Each entry records a decision, the reason for it, and the alternatives rejected. Add a new entry when a decision changes rather than editing an old one.

## D1. The core is `no_std` and allocation-free

Reason: the same code must run unchanged on a microcontroller. Fixed-capacity collections make memory use predictable and avoid heap fragmentation on a device that runs for weeks. A bare-metal build check enforces `no_std`.

Rejected: a `std` core, which cannot run on the `no_std` firmware; a core using `alloc`, which is unnecessary for this application and adds a failure mode.

## D2. Time arrives on events; the core has no clock

Reason: one source of truth. Tests construct timestamps directly, and a recording of timestamped inputs replays exactly.

Rejected: injecting a `Clock` trait into the core, which creates a second source of time that can disagree with event timestamps.

## D3. No periodic tick; the core reports its next deadline

Reason: the platform can sleep until input or the next deadline, so a still screen costs nothing, which matters on battery. Frames are requested only while something is moving.

Rejected: a fixed-rate tick, which wastes power and tempts behavior to depend on the tick rate.

## D4. `Screen` records focus; long-lived state lives outside it

Reason: a running timer must survive navigating away from its screen, and Wade's state must survive a visit to an app.

Rejected: a mode enum whose variants own each feature's state, which destroys that state on every navigation.

## D5. Platforms send raw input; the core interprets it

Reason: the core owns the layout, so only the core can know what a touch at a point means. Gesture logic in the core is testable without hardware.

Rejected: platforms sending semantic events such as "open timer" or "back", which duplicates layout knowledge in every platform.

## D6. `esp-hal` with Embassy, not ESP-IDF

Reason: Embassy's async tasks, timers, and channels are first-class on `esp-hal`. Wi-Fi is available through `esp-radio` and `embassy-net` without ESP-IDF. The build is pure Rust, and the firmware uses the same `no_std` model as the core.

Cost: HTTPS is less turnkey than with ESP-IDF's built-in HTTP client. M7 spikes TLS before building on it.

Rejected: `esp-idf-svc` (`std` on top of ESP-IDF), which has mature networking and storage but treats Embassy as secondary and adds ESP-IDF's C build system.

## D7. `embedded-graphics` is the rendering interface

Reason: an established ecosystem with a desktop simulator and display drivers, so the same drawing code runs on both targets.

Rejected: a custom canvas trait, which would need its own desktop and device implementations for no gain.

## D8. Wade is drawn from shapes through a pose rig

Reason: a few continuous parameters describe every expression, so transitions interpolate smoothly. There is no asset pipeline, the binary stays small, and all drawing is one code path.

Rejected: pixel-art sprites, which need a separate drawing for every expression and frame, cannot interpolate, and need asset tooling. Revisit if shapes cannot reach the desired look; props could become sprites without changing the rig.

## D9. `view` and `draw` are separate

Reason: behavior tests assert on plain data instead of pixels. Drawing is tested with snapshots.

Rejected: a single `render` method on `App`, which makes behavior testable only through pixels.

## D10. Full-frame redraws first

Reason: the simplest correct approach. The hardware spike measures flush time; partial updates are added only if the measurement requires them.

Rejected: dirty-rectangle tracking from the start, which adds complexity before there is evidence it is needed.

## D11. Randomness is a seeded generator inside the core

Reason: random behavior stays deterministic for tests and replays. The platform provides only the seed.

Rejected: reading a hardware or OS random source from the core, which breaks determinism and the no-hardware rule.

## D12. No vibration effect

Reason: the CoreS3 Lite has no vibration motor.

## D13. `wade-cores3` sits outside the root workspace

Reason: it needs a different toolchain and target. Inside the workspace it would break `cargo test` at the root.

Rejected: a single workspace with per-crate target configuration, which is fragile with the Xtensa toolchain.

## D14. Partial flush via a damage rectangle

Reason: a full-frame flush takes about 31 ms at 40 MHz, nearly a whole 33 ms frame, and a single framebuffer cannot be drawn into while it is being sent. Drawing the full frame and sending only the changed rectangle cuts a blink to about 3 ms. `render::damage(prev, next)` is a pure function in the core, so its correctness is a desktop property test. This refines D10: it is built in M3 only if the spike's measurements call for it.

Rejected: double buffering, which needs a second 150 KB framebuffer and still sends every pixel; dirty-rectangle tracking inside the drawing code, which couples every widget to damage bookkeeping.

## D15. Hidden features keep time but request no frames

Reason: a blink nobody can see should not wake the device. Wade's schedule keeps advancing while he is off screen, so returning to Buddy shows no catch-up burst, and deadlines stay a function of elapsed time.

Rejected: animating Wade on every screen, which wastes power; freezing his schedule while hidden, which makes his behavior depend on navigation history.

## D16. Recordings carry a hash of discrete state

Reason: a hash after each input event makes "replays identically" checkable and pinpoints the first divergence. Hashing only discrete state (screen, expression, activity, timer) keeps recordings valid when animation or artwork changes.

Rejected: hashing the rendered frame, which would break every recording whenever Wade's look or animation timing changes; no hash, which leaves replay fidelity checked by eye.

## D17. Effects are never waited on

Reason: `Output` holds at most four effects and drops extras, and the device app task sends effects with `try_send`, dropping them if a consumer is busy. Losing a chime is better than stalling touch and rendering, and a panic would violate the "`handle` never panics" invariant.

Rejected: blocking sends, which let a playing chime freeze the UI; a larger or growable effect list, which adds memory for a case that should not occur.
