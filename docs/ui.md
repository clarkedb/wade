# User interface

## Screen facts

| Property | Value |
|---|---|
| Logical resolution | 320×240, landscape, origin top-left |
| Color format | Rgb565 |
| Physical size | 2.0" diagonal, about 200 pixels per inch |
| Minimum touch target | 48×48 px (about 6 mm) |
| Input | Touch only. The device has no user buttons besides power and reset. |

Desktop renders the same 320×240 image scaled 2× in a window. Mouse input maps to touch.

## Screens and navigation

Each screen fills the display. Wade appears only on the Buddy screen; app screens do not show him.

| Screen | Milestone | Purpose |
|---|---|---|
| Buddy | M1 | Wade. The default screen. |
| Timer | M2 | Countdown timer |
| Launcher | M5 | Grid of app tiles |
| Settings | M5 | Device settings |
| Weather | M7 | Current conditions |

Navigation in M2:

```text
Buddy ──(apps button)──► Timer
Timer ──(back button)──► Buddy
Timer finishes, on any screen ──► Timer, showing Done
Timer in Done ──(Dismiss or back button)──► Buddy, Wade Proud
```

From M5, the apps button opens the Launcher. Back from an app returns to the Launcher, and back from the Launcher returns to Buddy. The one exception is a finished timer: dismissing it, with Dismiss or with back, always returns to Buddy so Wade can be Proud.

## Touch handling

The platform sends raw `Down`, `Move`, and `Up` samples. The core's `TouchTracker` turns them into taps.

A tap targets the element under the `Down` point. The element shows a pressed style while the touch stays inside it. The tap fires on `Up` if the `Up` point is still inside the same element; moving outside cancels it. There is no time limit, so no deadline is involved.

A screen change cancels any tap in progress. If the core switches screens while a finger is down (the timer finishing, for example), the rest of that touch is ignored until the next `Down`. Otherwise the `Up` would land on an element of the new screen that the user never pressed. Long-press and swipe gestures are not used until a feature needs them.

## Layout

All geometry (element rectangles, Wade's hit area, button positions) is defined once in `wade_core::layout` and used by both drawing and hit-testing, so what you see is always what you can tap.

The four 48×48 corners are reserved for navigation on every screen: back top-left, apps bottom-right, and the other two kept free. Nothing else takes touches there, Wade included.

## Buddy screen

Wade fills the screen. The apps button is a 48×48 hit area in the bottom-right corner with a small, low-contrast icon. Wade's hit area is a rectangle around his eyes, clear of the corners. In M1 through M3, touches elsewhere do nothing; M4 adds gaze-following (see [character.md](character.md#behavior-rules)).

## Timer

### States

```rust
pub enum TimerState {
    Ready   { set: Duration },
    Running { set: Duration, ends_at: Instant },
    Paused  { set: Duration, remaining: Duration },
    Done    { set: Duration, since: Instant },
}
```

`set` is the chosen duration. It is kept in every state so that Reset and Dismiss return to Ready with the same duration. `since` is when the timer finished; the chime repeats are scheduled from it.

```text
            −1m / +1m
             ┌─────┐
             ▼     │
          ┌─────────┐   Start   ┌─────────┐   Pause    ┌─────────┐
          │  Ready  │ ────────► │ Running │ ─────────► │ Paused  │
          └─────────┘           └────┬────┘ ◄───────── └────┬────┘
             ▲  ▲                    │        Resume        │
             │  └───────── Reset ────┼──────────────────────┘
             │                       │ time reaches ends_at
             │  Dismiss         ┌────┴────┐
             └───────────────── │  Done   │
                                └─────────┘
```

### Rules

| Rule | Detail |
|---|---|
| Duration range | 1 to 99 minutes, in 1-minute steps. The default is 5:00. |
| Display | MM:SS. Remaining time rounds up to whole seconds, so the display shows 00:01 during the last second. The digits change on second boundaries measured back from `ends_at` (at `ends_at − k × 1 s`), not on whole seconds of the clock. |
| Pause and resume | Pause stores `remaining = ends_at.saturating_since(now)`. Resume sets `ends_at = now + remaining`. |
| Completion | When `now` reaches `ends_at`, the timer enters Done with `since = ends_at`, emits `Effect::Chime`, and switches the screen to Timer. |
| Chime repeats | While Done, the chime repeats every 10 s: at `since + 10 s`, `since + 20 s`, and so on, for 10 chimes in total including the first. Dismissing stops them. From M5, the "chime off" setting silences all of them. |
| Screen independence | The timer runs regardless of which screen is visible. |
| Dismiss | The Dismiss button or the back button. Done returns to Ready with the same `set`, the screen returns to Buddy, and Wade is Proud for 2 s. |
| Deadlines | While Running: `ends_at`. While Running and the Timer screen is visible: also the next second boundary, so the digits update on time. While Done: the next chime repeat, until the tenth. |

### Layout

```text
┌──────────────────────────────────────────┐
│ ‹                                  Timer │  y 0–48: back button (48×48), title
│                                          │
│                  05:00                   │  y 48–176: digits, 64 px tall
│                                          │
│    −1m          Start          +1m       │  y 176–232: buttons, 56 px tall
└──────────────────────────────────────────┘
```

The button row has 8 px side margins and 16 px gaps: left 72 px, center 128 px, right 72 px.

| State | Left | Center | Right |
|---|---|---|---|
| Ready | −1m | Start | +1m |
| Running | (empty) | Pause | (empty) |
| Paused | Reset | Resume | (empty) |
| Done | (empty) | Dismiss | (empty) |

In Done, the word "Done" replaces the digits. −1m is dimmed and ignores taps at 1:00, and +1m does the same at 99:00.

## Rendering

`render::draw` draws a complete frame from a `View` every time. Full redraws keep the drawing code simple. On the device, a full 320×240 Rgb565 frame is 153,600 bytes; at a 40 MHz SPI clock, sending it takes about 31 ms. That leaves almost nothing of a 33 ms frame, and with a single framebuffer the app task cannot draw the next frame while the last one is being sent. The hardware spike measures the real figure.

### Partial flush

If the measurement confirms full-frame flushes are too slow (expected), the fix is to keep drawing full frames into the framebuffer, which is cheap, and send only the part of the screen that changed. The ILI9342C accepts writes to any rectangular window.

```rust
// In wade_core::render. The smallest rectangle containing every pixel that
// differs between drawing `prev` and drawing `next`, or None if none differ.
// Returning a larger rectangle (up to the full screen) is always correct.
pub fn damage(prev: &View, next: &View) -> Option<Rectangle>;
```

The platform keeps the last drawn `View`, calls `damage` when `redraw` is set, draws the full frame, and flushes only the damaged rectangle. Rough costs at 40 MHz:

| Change | Rectangle | Flush time |
|---|---|---|
| Blink | about 210×90 (the eyes) | about 8 ms |
| Expression change | about 260×150 (the eyes and accents) | about 16 ms |
| A step of Thinking's dots | the dots | under 1 ms |
| A step asleep (breath, Z's) | the eye lines and Z's | up to about 10 ms |
| Timer tick | the changed digits | a few ms |
| Screen change | full screen | about 31 ms |

This keeps a single 150 KB framebuffer; double buffering would need a second one. `damage` is a pure function, so it is tested on desktop: a property test draws `prev` and `next`, computes the pixels that differ, and checks that they all lie inside `damage(prev, next)`. The spike also tries a faster SPI clock (60–80 MHz), which would add headroom if the panel and board wiring tolerate it.

`damage` is built in M3 only if the spike's numbers call for it ([D14](decisions.md#d14-partial-flush-via-a-damage-rectangle)).

### Text

| Use | Approach |
|---|---|
| Timer digits | Drawn from primitives in a rounded seven-segment style. Scales to any size with no font dependency. |
| Labels, titles, buttons | `embedded-graphics` built-in mono fonts. The largest, 10×20 px, is readable at this pixel density. |

### Color

Colors are defined once in a palette module. Colors should be checked on the device, because the panel and a desktop monitor render the same Rgb565 values differently.
