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

Each eye is a rounded rectangle. Expressions reshape it by cutting parts away in the background color: a flat lid droops from above, a wide disc risen from below leaves a smiling crescent, and a triangle cut across the top slants the eye, from the outer corner for worry and the inner corner for anger. Small accents beside the eyes finish some expressions. Blush, tears and sweat, and the sparkle carry the only color: pink, blue, and yellow. In mono, they take the eyes' color.

The eyes come in two styles, a setting ([D23](decisions.md#d23-eye-style-is-a-setting)):

| Style | Look |
|---|---|
| Pupils (default) | A round black pupil in each eye with a small highlight, up and to the left. The pupils do most of the looking: they travel inside the eye while the eyes move less than plain ones. Each expression sets its pupil size, from small for shock to large for warmth. Each pupil sits in the opening between the lids, and a nearly shut eye hides it. |
| Plain | Blank eyes. The whole eye moves to look. |

The look is tuned on the real screen in M4. The screen is 2.0" at about 200 pixels per inch, which constrains the design.

| Constraint | Value |
|---|---|
| Eye size | At least 24 px, except while closed |
| Line width | At least 3 px |
| Smallest detail | 4 px or larger |
| Color | Flat fills. No gradients, which band visibly in Rgb565. |
| Transforms | None. `embedded-graphics` cannot rotate shapes, so the rig moves, reshapes, and cuts features instead of rotating them. |

Props are part of the character. A keyboard and a cup are planned for idle activities in M4.

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
    pub pupil: f32,               // pupil diameter in pixels, when the eyes have pupils
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
| 4 | Surprised | Being woken by a tap (M1) or the power button, woken by an approach (M6), a timer he keeps finishing | Tall and wide | Exclaim, briefly |
| 5 | Sleepy | Falling asleep; groggy after the timer wakes him (M2) | Upper lids half down | |
| 6 | Thinking | No trigger yet | Smaller, lids slightly down, looking up and to the side | Dots counting up |
| 7 | Proud | Successes, such as a finished timer; repeated attention (M2, M4) | Smiling crescents, right eye taller, looking up and to one side | Sparkle |
| 8 | Focused | Watching the timer's last seconds; typing (M4) | Flattened, inner corners slightly cut, looking down | |
| 9 | Flustered | Caught napping by a timer he keeps; hubris backfire (M4) | Wide, worried, uneven | SweatDrop |

His pupils widen visibly when he is tapped. An expression that sets gaze holds it: glances pause until the expression ends. An expression's accent starts when the transition into it finishes, so it never appears on a half-formed face.

A beat is a short scripted reaction: expressions played in order, each held for its time, then back to what he rests in. Anything else that changes his expression, such as a tap, cuts it short. None has a trigger yet; the desktop's B key plays each in turn.

| Beat | Steps | Meant for |
|---|---|---|
| Eureka | Thinking 1.45 s, Proud 1.5 s | Working something out |
| Hubris | Flustered 1.5 s | Pride backfiring: a tap while Proud (M4) |
| Huff | Surprised 0.5 s, Angry 1.2 s | Being poked once too often |
| Mope | Sad 4 s | Being let down, such as the timer he keeps being reset |

Asleep is a state, not an expression; the expression is Sleepy. He does not blink, glance, or squint while asleep.

## Mood

His mood is how pleased with himself he has been lately: points from 0 to 100 that fade one every 6 s, so a full mood is gone in 10 minutes. It changes slowly and colors what he does rather than setting an expression.

| Event | Points |
|---|---|
| A tap on him while awake | +8 |
| Dismissing a finished timer he takes credit for (Proud) | +30 |
| The timer catching him napping while he keeps time | −40 |

| Level | Points | Shows as |
|---|---|---|
| Plain | 0–29 | Neutral as drawn |
| Pleased | 30–69 | Neutral with a slight smile from below and wider pupils |
| Smug | 70–100 | Neutral smiling more, right eye a little taller. Keeping time, he dozes off one time in two instead of three: pride before the fall. |

Only Neutral shows his mood. When the level changes while he is Neutral, he eases into the new look over 1 s, without a pop; otherwise the next expression change picks it up. A level change is a scheduled transition, so mood adds no frames while he is still.

## Animation

The rendered pose is built in layers, applied in this order, each computed from elapsed time.

| Layer | Behavior |
|---|---|
| Base expression | When the expression changes, the pose moves from its current value to the new target over 200 ms with ease-in-out: 300 ms as the eyes open at startup, 1 s into sleep. |
| Pop | On an expression change, the eyes jump 12% taller (30% into Surprised) and ease back over 400 ms. |
| Squint | In Neutral, every 8 to 16 s, the eyes narrow by 40% and relax over 400 ms. |
| Twitch | Asleep, 5 to 9 s after he falls asleep and then every 6 to 12 s, the eyes flicker open slightly for 300 ms, too little to show his pupils. |
| Activity (M4) | An idle activity overrides some parameters: gaze, prop, and prop phase for typing or sipping. |
| Glance | While nothing else sets gaze, the gaze jumps every 1.2 to 4 s to a random offset (up to 1.0 across and 0.8 up or down), or back to center 40% of the time. Each jump takes 80 ms. |
| Shake | Angry trembles by up to 2 px every 100 ms. Asleep, the face breathes 5 px up and down over 6 s. |
| Accent | The current accent holds (Blush), shows once (Exclaim, Sparkle, SweatDrop), or repeats (Tear, Dots, Steam, Zs). |
| Blink | Closes the eyes over 50 ms and reopens them over 70 ms by scaling `eye_open`: a fast shut and a slower open read as natural. Applied last, so Wade can blink during any expression. |

Blinks occur at random intervals of 2 to 6 seconds, and one in five is followed by a second 300 ms after it ends.

Every layer has a fixed end, never an easing that only approaches its target, so the end of each animation is a transition (see [D18](decisions.md#d18-animations-have-fixed-ends-lasting-motion-is-stepped)). Motion that lasts moves in steps at scheduled instants, so each step costs one frame instead of a stream of them: Angry's tremble and steam, Thinking's dots, and sleep's breath and Z's. Asleep, a step every 150 ms costs about seven frames a second.

While any motion is in progress, the character requests a deadline every frame (see [architecture.md](architecture.md#deadlines-and-frames)). When Wade is still, his only deadline is the next scheduled change, such as a blink, a step, or an expression expiring.

Wade animates only while the Buddy screen is visible. On other screens his schedule keeps moving forward but he requests no deadlines: blinks and other motions that fall due are skipped, and idle activities do not start (see [architecture.md](architecture.md#hidden-features)).

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
| Desktop P | Switch the eye style setting between pupils and plain |
| Desktop B | Play the next beat, in turn |

### M2 adds

| Trigger | Result |
|---|---|
| Returning to the Buddy screen after dismissing a finished timer (with Dismiss or back) | Proud for 2 s, then Neutral |
| The timer finishing on another screen while he is asleep | It wakes him, Sleepy, out of sight on the Timer screen, even with the volume off |
| Dismissing a finished timer that woke him | Sleepy for 3 s instead of Proud, then a coin flip: Neutral, or back to sleep |

### Keeping time

When the timer runs while the Buddy screen is visible, its countdown shows below Wade and he keeps time (see [ui.md](ui.md#keeping-time)). He is proud to be trusted with it, which is also how he gets caught out.

| Trigger | Result |
|---|---|
| A glance while keeping time | About a third of glances look down at the countdown |
| The last 10 s | Focused on the countdown until it ends; a tap makes him Happy for 2 s, then he goes back to watching |
| Keeping time with 2 min or more left, once per run | One time in three (two when [Smug](#mood)), he dozes off somewhere 30 to 70% of the way through what is left |
| The timer finishing while he keeps time | Surprised for 1 s, then Proud until it is dismissed |
| The timer finishing while he keeps time asleep | Caught napping: it wakes him Surprised for 1 s, then Flustered until it is dismissed |
| Dismissing a timer he kept | Proud for 2 s, then Neutral, even if it caught him napping |

Leaving Buddy, or pausing, ends his watch: he returns to Neutral, and the timer behaves as in M2.

### Power button

The CoreS3's power button and the CYD's BOOT button put Wade to bed or get him up. The desktop's space bar stands in for them.

| Trigger | Result |
|---|---|
| Power button on the Buddy screen | Asleep; if he already is, Surprised for 1 s, then Neutral, as for a tap |
| Power button on another screen | The Buddy screen, and asleep |
| Power button while the timer shows Done | Dismisses it, as Dismiss does |

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
