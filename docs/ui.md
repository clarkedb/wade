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
```

From M5, the apps button opens the Launcher. Back from an app returns to the Launcher, and back from the Launcher returns to Buddy.

## Touch handling

The platform sends raw `Down`, `Move`, and `Up` samples. The core's `TouchTracker` turns them into taps.

A tap targets the element under the `Down` point. The element shows a pressed style while the touch stays inside it. The tap fires on `Up` if the `Up` point is still inside the same element; moving outside cancels it. There is no time limit, so no deadline is involved. Long-press and swipe gestures are not used until a feature needs them.

## Layout

All geometry (element rectangles, Wade's hit area, button positions) is defined once in `wade_core::layout` and used by both drawing and hit-testing, so what you see is always what you can tap.

## Buddy screen

Wade fills the screen. The apps button is a 48×48 hit area in the bottom-right corner with a small, low-contrast icon. Wade's hit area is his head. In M1 through M3, touches elsewhere do nothing; M4 adds gaze-following (see [character.md](character.md#behavior-rules)).

## Timer

### States

```rust
pub enum Timer {
    Ready   { set: Duration },
    Running { set: Duration, ends_at: Instant },
    Paused  { set: Duration, remaining: Duration },
    Done    { set: Duration },
}
```

`set` is the chosen duration. It is kept in every state so that Reset and Dismiss return to Ready with the same duration.

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
| Display | MM:SS. Remaining time rounds up to whole seconds, so the display shows 00:01 during the last second and changes exactly on second boundaries. |
| Pause and resume | Pause stores `remaining = ends_at.saturating_since(now)`. Resume sets `ends_at = now + remaining`. |
| Completion | When `now` reaches `ends_at`, the timer enters Done, emits `Effect::Chime` once, and switches the screen to Timer. |
| Screen independence | The timer runs regardless of which screen is visible. |
| Dismiss | Done returns to Ready with the same `set`, the screen returns to Buddy, and Wade is Proud for 2 s. |
| Deadlines | While Running: `ends_at`. While Running and the Timer screen is visible: also the next second boundary, so the digits update on time. |

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

`render::draw` draws a complete frame from a `View` every time. Full redraws keep the drawing code simple. On the device, a full 320×240 Rgb565 frame is 153,600 bytes; at a 40 MHz SPI clock, sending it takes about 31 ms. The hardware spike measures the real figure. If it is too slow for smooth animation, the first optimization is to send only the rectangle that changed, which for Wade is usually the face.

Text:

| Use | Approach |
|---|---|
| Timer digits | Drawn from primitives in a rounded seven-segment style. Scales to any size with no font dependency. |
| Labels, titles, buttons | `embedded-graphics` built-in mono fonts. The largest, 10×20 px, is readable at this pixel density. |

Colors are defined once in a palette module. Colors should be checked on the device, because the panel and a desktop monitor render the same Rgb565 values differently.
