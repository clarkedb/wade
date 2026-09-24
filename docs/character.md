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

Wade is a pair of eyes: light, flat shapes on black, like a small robot's OLED face. He has no head or mouth, so the eyes carry every expression. He is drawn entirely from shapes in code with `embedded-graphics` primitives: rounded rectangles, circles, triangles, lines, and polylines.

Each eye is a rounded rectangle. Expressions reshape it by cutting parts away in the background color: a flat lid droops from above, a wide disc risen from below leaves a smiling crescent, and a triangle cut across the top slants the eye, from the outer corner for worry and the inner corner for anger. Small accents beside the eyes finish some expressions. Blush, tears and sweat, and the sparkle carry the only color: pink, blue, and yellow.

The look is tuned on the real screen in M4. The screen is 2.0" at about 200 pixels per inch, which constrains the design.

| Constraint | Value |
|---|---|
| Eye size | At least 24 px, except while closed |
| Line width | At least 3 px |
| Smallest detail | 4 px or larger |
| Color | Flat fills. No gradients, which band visibly in Rgb565. |
| Transforms | None. `embedded-graphics` cannot rotate shapes, so the rig moves, reshapes, and cuts features instead of rotating them. |

Props are part of the character. A keyboard and a cup appear during idle activities (M4).

## Pose rig

Every expression and animation reduces to a `Pose`. The drawing code reads only a `Pose`, never an expression name.

```rust
pub struct Pose {
    pub eye_size: (f32, f32),     // width and height of each eye in pixels, before blinking
    pub eye_radius: f32,          // corner radius in pixels
    pub eye_open: f32,            // 0.0 closed … 1.0 open; closing collapses each eye to a line
    pub upper_lid: f32,           // 0.0 … 1.0 of each eye covered from above (droop)
    pub lower_lid: f32,           // 0.0 … 1.0 pushed up from below into a crescent (smile)
    pub brow_tilt: f32,           // −1.0 outer corners cut (worried) … 1.0 inner corners cut (angry)
    pub asymmetry: f32,           // −1.0 right eye shorter … 1.0 right eye taller (smug)
    pub gaze: (f32, f32),         // where both eyes look, −1.0 … 1.0 on each axis
    pub face_offset: (f32, f32),  // whole-face offset in pixels: shaking and breathing
    pub accent: Accent,
    pub accent_phase: f32,        // 0.0 … 1.0 through the accent's motion; Zs runs to 2.0
    pub prop: Prop,
    pub prop_phase: f32,          // 0.0 … 1.0 through the prop's motion
}

pub enum Accent {
    None,
    Blush,      // pink marks under the outer corners (Happy)
    Exclaim,    // a "!" beside the face (Surprised)
    Tear,       // a tear falls from the right eye (Sad)
    Dots,       // up to three dots (Thinking)
    Steam,      // steam marks above the outer corners (Angry)
    Sparkle,    // one twinkle beside the right eye (Proud)
    SweatDrop,  // a drop slides down beside the left eye (Flustered)
    Zs,         // Z's rise and grow (asleep)
}

pub enum Prop {
    None,
    Keyboard,
    Cup,
}
```

An accent is a small secondary action beside the eyes. It stays inside a small region, so it costs only a small flush (see [ui.md](ui.md#partial-flush)).

The ESP32-S3 has a single-precision FPU, so `f32` is cheap on the device. Trigonometric and similar functions come from `libm`.

## Expressions

An expression is a named target pose, numbered for the desktop keys (see [platforms.md](platforms.md#desktop-wade-desktop)).

| # | Expression | Used for | Pose traits | Accent |
|---|---|---|---|---|
| 0 | Neutral | Default | Rounded squares, level | |
| 1 | Happy | Being tapped | Slightly larger; lower lids pushed up into crescents | Blush |
| 2 | Sad | No trigger yet | Smaller and drooping, outer corners cut | Tear, every few seconds |
| 3 | Angry | No trigger yet | Short, inner corners cut deep; a tremble | Steam, flashing |
| 4 | Surprised | Being woken (M1), woken by an approach (M6) | Tall and wide | Exclaim, briefly |
| 5 | Sleepy | Falling asleep | Upper lids half down | |
| 6 | Thinking | No trigger yet | Smaller, lids slightly down, looking up and to the side | Dots counting up |
| 7 | Proud | Successes, repeated attention (M2, M4) | Half-lidded and smug, right eye taller, looking up and to one side | Sparkle |
| 8 | Focused | Typing (M4) | Flattened, inner corners slightly cut, looking down | |
| 9 | Flustered | Hubris backfire (M4) | Wide, worried, uneven | SweatDrop |

An expression that sets gaze holds it: glances pause until the expression ends. An expression's accent starts when the transition into it finishes, so it never appears on a half-formed face.

Asleep is a state, not an expression; the expression is Sleepy. He does not blink, glance, or squint while asleep.

## Animation

The rendered pose is built in layers, applied in this order, each computed from elapsed time.

| Layer | Behavior |
|---|---|
| Base expression | When the expression changes, the pose moves from its current value to the new target over 200 ms with ease-in-out: 300 ms as the eyes open at startup, 1 s into sleep. |
| Pop | On an expression change, the eyes jump 12% taller (30% into Surprised) and ease back over 400 ms. |
| Squint | In Neutral, every 8 to 16 s, the eyes narrow by 40% and relax over 400 ms. |
| Twitch | Asleep, 5 to 9 s after he falls asleep and then every 6 to 12 s, the eyes flutter open a crack for 300 ms. |
| Activity (M4) | An idle activity overrides some parameters: gaze, prop, and prop phase for typing or sipping. |
| Glance | While nothing else sets gaze, the gaze jumps every 1.2 to 4 s to a random offset (up to 1.0 across and 0.8 up or down), or back to center 40% of the time. Each jump takes 80 ms. |
| Shake | Angry trembles by up to 2 px every 100 ms. Asleep, the face breathes 5 px up and down over 6 s. |
| Accent | The current accent holds (Blush), shows once (Exclaim, Sparkle, SweatDrop), or repeats (Tear, Dots, Steam, Zs). |
| Blink | Closes the eyes over 50 ms and reopens them over 70 ms by scaling `eye_open`: a fast shut and a slower open read as natural. Applied last, so Wade can blink during any expression. |

Blinks occur at random intervals of 2 to 6 seconds, and one in five is followed by a second 300 ms after it ends.

Every layer has a fixed end, never an easing that only approaches its target, so the end of each animation is a transition (see [D18](decisions.md#d18-animations-have-fixed-ends-lasting-motion-is-stepped)). Motion that lasts moves in steps at scheduled instants, so each step costs one frame instead of a stream of them: Angry's tremble and steam, Thinking's dots, and sleep's breath and Z's. Asleep, a step every 150 ms costs about seven frames a second.

While any motion is in progress, the character requests a deadline every frame (see [architecture.md](architecture.md#deadlines-and-frames)). When Wade is still, his only deadline is the next scheduled change, such as a blink, a step, or an expression expiring.

Wade animates only while the Buddy screen is visible. On other screens his schedule keeps moving forward but he requests no frames: blinks and other motions that fall due are skipped, and idle activities do not start (see [architecture.md](architecture.md#hidden-features)).

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
| Start | Eyes open from closed over 300 ms, then Neutral |
| Every 2 to 6 s, at random | Blink |
| Tap on Wade | Happy for 2 s, then Neutral. Another tap restarts the 2 s. |
| Tap on Wade while asleep | Surprised for 1 s, then Neutral |
| Tap elsewhere on the Buddy screen | Nothing |
| Desktop number key 0–9 | That expression, held until a tap or another key changes it |
| Desktop Z | Asleep, until a tap or a number key |

### M2 adds

| Trigger | Result |
|---|---|
| Returning to the Buddy screen after dismissing a finished timer (with Dismiss or back) | Proud for 2 s, then Neutral |

### M4 (proposed; finalized at the start of M4)

| Trigger | Result |
|---|---|
| 20 to 60 s without a touch on any screen, at random, while the Buddy screen is visible | Start a random idle activity |
| Tap during an activity | The activity stops; Happy |
| Three or more taps on Wade within 2 s | Proud for 3 s |
| Tap on Wade while Proud | One time in four, the hubris beat: Flustered for 1.5 s, then Neutral. Otherwise Proud restarts. |
| Touch anywhere on the Buddy screen | Gaze moves toward the touch point for 1 s |

### M6 adds

| Trigger | Result |
|---|---|
| Inactivity past the sleep threshold | Sleepy, then asleep, then the display turns off (see [roadmap.md](roadmap.md#m6-power)) |
| Touch while Sleepy, or someone approaching while Sleepy or asleep | Display on, eyes spring open with a pop, Surprised for 1 s, then Neutral |
