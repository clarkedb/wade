# Character

## Persona

Wade is helpful, smart, and quick-witted. He is loyal to the person at the desk. He is also a little too proud of himself, and occasionally that pride backfires.

For now Wade communicates only through expression, gaze, and small actions. He has no text or speech. A scrolling text readout may come later (see [roadmap.md](roadmap.md#future)).

| Trait | How it shows |
|---|---|
| Helpful, loyal | Attentive. He looks toward touches and reacts immediately. |
| Smart, quick-witted | Crisp timing and knowing looks rather than slow, broad reactions. |
| Proud | A smug, pleased expression after successes, such as a finished timer or repeated attention. |
| Suffers for hubris | Sometimes a proud moment goes wrong and he is briefly flustered. |

## Appearance

Wade is an original character design, drawn entirely from shapes in code with `embedded-graphics` primitives: circles, ellipses, arcs, polylines, triangles, and rounded rectangles. The style is flat color with thick outlines and a small palette.

His final look is designed in M4 on the real screen. M1 through M3 use placeholder geometry (a round head, two eyes, two brows, and a mouth) driven by the same pose rig, so replacing the look later changes only the drawing code.

The eyes carry most of the expression: each expression should read from the eyes and brows alone, with the mouth reinforcing it. Eyes are shaped by covering parts of them with shapes in the skin color, then outlining the cut edge. A lid lowers from above, a disc rising from below pushes the lower lid into a smiling crescent, and a triangle cut from one top corner slants the eye. Each pupil carries a small highlight that moves only part as far as the pupil, so the eye reads as round.

The screen is 2.0" at about 200 pixels per inch, which constrains the design.

| Constraint | Value |
|---|---|
| Face height | At least 160 px |
| Eye size | At least 24 px |
| Line width | At least 3 px |
| Smallest detail | 4 px or larger |
| Color | Flat fills. No gradients, which band visibly in Rgb565. |
| Transforms | None. `embedded-graphics` cannot rotate shapes, so the rig moves, reshapes, and covers features instead of rotating them. |

Props are part of the character. A keyboard and a cup with a straw appear during idle activities (M4).

## Pose rig

Every expression and animation reduces to a `Pose`. The drawing code reads only a `Pose`, never an expression name.

```rust
pub struct Pose {
    pub eye_open: f32,            // 0.0 closed … 1.0 open; the upper lid lowers
    pub eye_scale: f32,           // 1.0 normal … 1.3 widest (surprise)
    pub lower_lid: f32,           // 0.0 flat … 1.0 pushed up into a crescent (smile)
    pub gaze: (f32, f32),         // pupil offset, −1.0 … 1.0 on each axis
    pub brow_raise: f32,          // −1.0 lowered … 1.0 raised
    pub brow_tilt: f32,           // −1.0 worried … 1.0 determined
    pub brow_asymmetry: f32,      // 0.0 even … 1.0 one brow fully raised (skeptical)
    pub mouth_curve: f32,         // −1.0 frown … 1.0 smile
    pub mouth_open: f32,          // 0.0 closed … 1.0 open
    pub mouth_skew: f32,          // −1.0 … 1.0, lopsided (smirk)
    pub head_offset: (f32, f32),  // small bob or lean, in pixels
    pub prop: Prop,
    pub prop_phase: f32,          // 0.0 … 1.0 through the prop's motion
    pub accent: Accent,
    pub accent_phase: f32,        // 0.0 … 1.0 through the accent's motion
}

pub enum Prop {
    None,
    Keyboard,
    Cup,
}

pub enum Accent {
    None,
    Sparkle,    // one twinkle beside an eye (Proud)
    SweatDrop,  // a drop slides down the side of the head (Flustered)
    Zs,         // Z's rise from the head (asleep)
}
```

An accent is a small secondary action beside the face. It stays inside a small region, so it costs only a small flush (see [ui.md](ui.md#partial-flush)).

The ESP32-S3 has a single-precision FPU, so `f32` is cheap on the device. Trigonometric and similar functions come from `libm`.

## Expressions

An expression is a named target pose.

| Expression | Used for | Key pose traits | First milestone |
|---|---|---|---|
| Neutral | Default | Eyes open, slight smile | M1 |
| Happy | Being tapped | Wide smile, raised brows, lower lids pushed up into crescents | M1 |
| Proud | Successes, repeated attention | Smirk, half-closed eyes, one brow raised, gaze up and to one side; Sparkle | M2 (placeholder), M4 (final) |
| Focused | Typing | Eyes down, brows slightly lowered | M4 |
| Flustered | Hubris backfire | Eyes wide, brows worried, mouth open and uneven; SweatDrop | M4 |
| Surprised | Woken suddenly | Eyes wide and enlarged, brows high, mouth open | M6 |
| Sleepy | Long inactivity | Eyes mostly closed, head lowered | M6 |

An expression that sets gaze holds it: glances pause until the expression ends. An expression's accent starts when the transition into it finishes, so it never appears on a half-formed face.

## Animation

The rendered pose is built in layers, applied in this order, each computed from elapsed time.

| Layer | Behavior | First milestone |
|---|---|---|
| Base expression | When the expression changes, the pose moves from its current value to the new target over 200 ms with ease-in-out. | M1 |
| Pop | On an expression change, `eye_scale` jumps up by 10% (25% into Surprised) and eases back over 300 ms. | M4 |
| Activity | An idle activity overrides some parameters: gaze, prop, and prop phase for typing or sipping. | M4 |
| Glance | While nothing else sets gaze, the gaze jumps every 3 to 8 s to a small random offset (up to 0.3 on each axis), or back to center half the time. Each jump takes 50 ms. | M4 |
| Accent | The current accent runs its motion once, or loops for Zs. | M4 |
| Blink | Closes the eyes over 50 ms and reopens them over 70 ms by scaling `eye_open`: a fast shut and a slower open read as natural. Applied last, so Wade can blink during any expression. | M1 |

Blinks occur at random intervals of 2 to 6 seconds. From M4, one blink in five is followed by a second 300 ms later.

Every layer has a fixed end, never an easing that only approaches its target, so the end of each animation is a transition (see [D18](decisions.md#d18-animations-have-fixed-ends-lasting-motion-is-stepped)). Motion that lasts, such as rising Z's or a sleeping breath, moves in whole-pixel steps at scheduled instants, so each step costs one frame instead of a stream of them.

While a transition, blink, glance, or activity is in progress, the character requests a deadline every frame (see [architecture.md](architecture.md#deadlines-and-frames)). When Wade is still, his only deadline is the next scheduled blink, glance, step, or activity.

Wade animates only while the Buddy screen is visible. On other screens his schedule keeps moving forward but he requests no frames: blinks that fall due are skipped, and idle activities do not start (see [architecture.md](architecture.md#hidden-features)).

Idle activities (M4):

| Activity | Duration | Motion |
|---|---|---|
| Typing | 3 to 8 s | Focused expression, keyboard prop, keys alternating every 80 to 120 ms |
| Sipping | 2 to 3 s | Cup rises over 400 ms, holds, lowers; eyes half closed |
| Looking around | 1 to 2 s | Gaze moves to two or three random points |

## Behavior rules

### M1

| Trigger | Result |
|---|---|
| Start | Neutral |
| Every 2 to 6 s, at random | Blink |
| Tap on Wade | Happy for 2 s, then Neutral. Another tap restarts the 2 s. |
| Tap elsewhere on the Buddy screen | Nothing |

### M2 adds

| Trigger | Result |
|---|---|
| Returning to the Buddy screen after dismissing a finished timer (with Dismiss or back) | Proud for 2 s, then Neutral |

### M4 (proposed; finalized at the start of M4)

| Trigger | Result |
|---|---|
| Start | Eyes open from closed over 300 ms, then Neutral |
| 20 to 60 s without a touch on any screen, at random, while the Buddy screen is visible | Start a random idle activity |
| Tap during an activity | The activity stops; Happy |
| Three or more taps on Wade within 2 s | Proud for 3 s |
| Tap on Wade while Proud | One time in four, the hubris beat: Flustered for 1.5 s, then Neutral. Otherwise Proud restarts. |
| Touch anywhere on the Buddy screen | Gaze moves toward the touch point for 1 s |

### M6 adds

| Trigger | Result |
|---|---|
| Inactivity past the sleep threshold | Sleepy, then asleep, then the display turns off (see [roadmap.md](roadmap.md#m6-power)) |
| Touch or someone approaching while Sleepy or asleep | Display on, eyes spring open with a pop, Surprised for 1 s, then Neutral |

Asleep is a state, not an expression. The eyes close over 1 s, much slower than a blink. A slow breath moves the head a pixel or two, and Zs loop. Both are stepped, so sleeping costs a few frames a second until the display turns off.
