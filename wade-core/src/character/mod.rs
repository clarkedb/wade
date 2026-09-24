//! Wade: state, expressions, animation, and the pose rig (docs/character.md).

mod pose;

pub use pose::{Accent, Pose, Prop};

use core::f32::consts::TAU;

use crate::rng::Rng;
use crate::time::{Duration, Instant, millis};

/// Mixed into the seed to give motion its own random stream, apart from
/// behavior's (docs/architecture.md#randomness).
const MOTION_STREAM: u64 = 0x6D6F_7469_6F6E_5EED;

/// How long Wade stays Happy after a tap on him.
pub const HAPPY_DURATION: Duration = Duration::from_millis(2_000);
/// How long Wade stays Surprised after a tap wakes him.
pub const WAKE_DURATION: Duration = Duration::from_millis(1_000);
/// How long a blink takes, closing and reopening.
pub const BLINK_DURATION: Duration = Duration::from_millis(120);
/// A blink shuts fast and reopens slower; together they make `BLINK_DURATION`.
const BLINK_CLOSE: Duration = Duration::from_millis(50);
const BLINK_OPEN: Duration = Duration::from_millis(70);
/// Blinks start at random intervals between these bounds (inclusive).
pub const BLINK_INTERVAL_MIN: Duration = Duration::from_millis(2_000);
pub const BLINK_INTERVAL_MAX: Duration = Duration::from_millis(6_000);
/// One blink in this many is doubled: another starts `DOUBLE_BLINK_GAP` after it ends.
pub const DOUBLE_BLINK_ONE_IN: u64 = 5;
pub const DOUBLE_BLINK_GAP: Duration = Duration::from_millis(300);
/// How long the pose takes to move to a new expression's target.
pub const EXPRESSION_TRANSITION: Duration = Duration::from_millis(200);
/// How long the eyes take to open at startup.
const START_TRANSITION: Duration = Duration::from_millis(300);
/// How long the eyes take to shut when he falls asleep.
const SLEEP_TRANSITION: Duration = Duration::from_millis(1_000);

/// On an expression change the eyes jump this much taller, then settle.
const POP: f32 = 0.12;
const POP_SURPRISED: f32 = 0.3;
const POP_DURATION: Duration = Duration::from_millis(400);

const GLANCE_INTERVAL_MIN: Duration = Duration::from_millis(1_200);
const GLANCE_INTERVAL_MAX: Duration = Duration::from_millis(4_000);
const GLANCE_DURATION: Duration = Duration::from_millis(80);
const GLANCE_RECENTER_PERCENT: u64 = 40;

/// Neutral's idle squint: the eyes narrow by `SQUINT_DEPTH` and relax.
const SQUINT_INTERVAL_MIN: Duration = Duration::from_millis(8_000);
const SQUINT_INTERVAL_MAX: Duration = Duration::from_secs(16);
const SQUINT_DURATION: Duration = Duration::from_millis(400);
const SQUINT_DEPTH: f32 = 0.4;

const TEAR_DELAY: Duration = Duration::from_millis(1_500);
const TEAR_FALL: Duration = Duration::from_millis(2_000);
const TEAR_GAP_MIN: Duration = Duration::from_millis(2_500);
const TEAR_GAP_MAX: Duration = Duration::from_millis(5_000);

const DOTS_STEP: Duration = Duration::from_millis(420);
const STEAM_STEP: Duration = Duration::from_millis(200);
const SHAKE_STEP: Duration = Duration::from_millis(100);
const SHAKE_PX: f32 = 2.0;

const EXCLAIM_DURATION: Duration = Duration::from_millis(700);
const SPARKLE_DURATION: Duration = Duration::from_millis(500);
const SWEAT_DURATION: Duration = Duration::from_millis(900);

/// Asleep, breathing and Z's move on this tick, one frame per step (D18).
const SLEEP_STEP: Duration = Duration::from_millis(150);
/// A new Z starts every cycle and rises for two.
const ZS_CYCLE: Duration = Duration::from_millis(1_500);
const BREATH_PERIOD: Duration = Duration::from_millis(6_000);
const BREATH_PX: f32 = 5.0;

/// Asleep, the eyes now and then flutter open a crack.
const TWITCH_FIRST_MIN: Duration = Duration::from_millis(5_000);
const TWITCH_FIRST_MAX: Duration = Duration::from_millis(9_000);
const TWITCH_INTERVAL_MIN: Duration = Duration::from_millis(6_000);
const TWITCH_INTERVAL_MAX: Duration = Duration::from_secs(12);
const TWITCH_DURATION: Duration = Duration::from_millis(300);
/// As a twitch starts, the eyes open this much taller, as a fraction, then
/// settle: a flicker too small to show his pupils.
const TWITCH: f32 = 0.35;

/// A named target pose.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Expression {
    #[default]
    Neutral,
    Happy,
    Sad,
    Angry,
    Surprised,
    Sleepy,
    Thinking,
    Proud,
    Focused,
    Flustered,
}

impl Expression {
    /// Every expression, numbered as on the desktop keys 0–9.
    pub const ALL: [Expression; 10] = [
        Expression::Neutral,
        Expression::Happy,
        Expression::Sad,
        Expression::Angry,
        Expression::Surprised,
        Expression::Sleepy,
        Expression::Thinking,
        Expression::Proud,
        Expression::Focused,
        Expression::Flustered,
    ];

    /// The target pose for this expression, without its accent.
    #[must_use]
    pub const fn pose(self) -> Pose {
        match self {
            Expression::Neutral => Pose::REST,
            Expression::Happy => Pose {
                eye_size: (90.0, 90.0),
                lower_lid: 0.44,
                pupil: 38.0,
                ..Pose::REST
            },
            Expression::Sad => Pose {
                eye_size: (80.0, 65.0),
                eye_radius: 25.0,
                upper_lid: 0.15,
                brow_tilt: -0.77,
                pupil: 35.0,
                ..Pose::REST
            },
            Expression::Angry => Pose {
                eye_size: (85.0, 60.0),
                eye_radius: 20.0,
                brow_tilt: 0.93,
                pupil: 24.0,
                ..Pose::REST
            },
            Expression::Surprised => Pose {
                eye_size: (95.0, 110.0),
                eye_radius: 47.0,
                pupil: 22.0,
                ..Pose::REST
            },
            Expression::Sleepy => Pose {
                eye_size: (85.0, 65.0),
                eye_radius: 25.0,
                upper_lid: 0.54,
                ..Pose::REST
            },
            Expression::Thinking => Pose {
                eye_size: (75.0, 75.0),
                eye_radius: 27.0,
                upper_lid: 0.17,
                gaze: (0.875, -1.0),
                pupil: 28.0,
                ..Pose::REST
            },
            Expression::Proud => Pose {
                eye_size: (85.0, 72.0),
                eye_radius: 26.0,
                upper_lid: 0.4,
                lower_lid: 0.22,
                asymmetry: 0.8,
                gaze: (-0.6, -0.5),
                pupil: 33.0,
                ..Pose::REST
            },
            Expression::Focused => Pose {
                eye_size: (85.0, 55.0),
                eye_radius: 18.0,
                upper_lid: 0.12,
                brow_tilt: 0.35,
                gaze: (0.0, 0.9),
                pupil: 26.0,
                ..Pose::REST
            },
            Expression::Flustered => Pose {
                eye_size: (95.0, 100.0),
                eye_radius: 40.0,
                brow_tilt: -0.5,
                asymmetry: -0.6,
                pupil: 22.0,
                ..Pose::REST
            },
        }
    }

    /// The accent shown once the transition into this expression finishes.
    #[must_use]
    pub const fn accent(self) -> Accent {
        match self {
            Expression::Happy => Accent::Blush,
            Expression::Sad => Accent::Tear,
            Expression::Angry => Accent::Steam,
            Expression::Surprised => Accent::Exclaim,
            Expression::Thinking => Accent::Dots,
            Expression::Proud => Accent::Sparkle,
            Expression::Flustered => Accent::SweatDrop,
            Expression::Neutral | Expression::Sleepy | Expression::Focused => Accent::None,
        }
    }

    /// True if the expression sets its own gaze, which pauses glances.
    const fn holds_gaze(self) -> bool {
        matches!(
            self,
            Expression::Thinking | Expression::Proud | Expression::Focused
        )
    }
}

/// Asleep: the eyes shut to thin lines.
pub const ASLEEP: Pose = Pose {
    eye_size: (75.0, 8.0),
    eye_radius: 3.0,
    ..Pose::REST
};

/// Wade's character state. Kept on every screen; animates only while visible.
///
/// Every scheduled instant is stored, so the pose at any time is a function of
/// this state and that time alone (docs/architecture.md#deadlines-and-frames).
#[derive(Clone, Debug)]
pub(crate) struct Wade {
    /// The latest instant Wade was advanced to or given input at.
    now: Instant,
    /// Randomness for motion only: blinks, glances, squints, tears, and twitches.
    motion: Rng,
    expression: Expression,
    asleep: bool,
    /// When a timed expression returns to Neutral.
    expires_at: Option<Instant>,
    /// The base pose eases from `from` to the current target over `transition`,
    /// starting at `changed_at`.
    from: Pose,
    changed_at: Instant,
    transition: Duration,
    /// How much taller the eyes pop at `changed_at`.
    pop: f32,
    next_blink: Instant,
    /// The next blink is the second of a double, which never doubles again.
    second_blink: bool,
    blink_at: Option<Instant>,
    next_glance: Instant,
    /// The glance offset eases from `glance_from` to `glance_to`, starting at `glance_at`.
    glance_from: (f32, f32),
    glance_to: (f32, f32),
    glance_at: Instant,
    next_squint: Instant,
    squint_at: Option<Instant>,
    next_tear: Option<Instant>,
    tear_at: Option<Instant>,
    next_twitch: Option<Instant>,
    twitch_at: Option<Instant>,
}

impl Wade {
    #[must_use]
    pub fn new(now: Instant, seed: u64) -> Self {
        let mut wade = Wade {
            now,
            motion: Rng::new(seed ^ MOTION_STREAM),
            expression: Expression::Neutral,
            asleep: false,
            expires_at: None,
            from: Pose {
                eye_open: 0.0,
                ..Pose::REST
            },
            changed_at: now,
            transition: START_TRANSITION,
            pop: 0.0,
            next_blink: now,
            second_blink: false,
            blink_at: None,
            next_glance: now,
            glance_from: (0.0, 0.0),
            glance_to: (0.0, 0.0),
            glance_at: now,
            next_squint: now,
            squint_at: None,
            next_tear: None,
            tear_at: None,
            next_twitch: None,
            twitch_at: None,
        };
        wade.schedule_idle(now);
        wade
    }

    /// The next scheduled discrete change (a blink starting or ending, an
    /// expression expiring, a motion finishing, a step of a stepped motion), or
    /// `None`. Stepped motion schedules nothing while Wade is not `visible`.
    ///
    /// Must be strictly later than the last instant passed to `advance`, unless
    /// the schedule has saturated at `Instant::MAX`. Frames are not transitions:
    /// they come from [`Wade::animating`].
    #[must_use]
    pub fn next_transition(&self, visible: bool) -> Option<Instant> {
        let end = |start: Option<Instant>, length| start.map(|t| t + length);
        [
            self.expires_at,
            Some(self.settled_at()),
            (self.pop > 0.0).then(|| self.changed_at + POP_DURATION),
            self.accent_end(),
            Some(self.next_blink),
            end(self.blink_at, BLINK_DURATION),
            Some(self.next_glance),
            Some(self.glance_at + GLANCE_DURATION),
            Some(self.next_squint),
            end(self.squint_at, SQUINT_DURATION),
            self.next_tear,
            end(self.tear_at, TEAR_FALL),
            self.next_twitch,
            end(self.twitch_at, TWITCH_DURATION),
            self.next_step().filter(|_| visible),
        ]
        .into_iter()
        .flatten()
        .filter(|&t| t > self.now)
        .min()
    }

    /// Apply every transition due at `now`. `visible` is false while another
    /// screen is shown: the schedule still moves forward, but due blinks and
    /// other motions are skipped (docs/architecture.md#hidden-features).
    ///
    /// Returns true if the view changed without a motion to show it: a motion
    /// starting, a step, an expression expiring, or a still accent ending. A
    /// motion in progress, and its last frame, are `App`'s to redraw.
    #[must_use = "a true result means the view must be redrawn"]
    pub fn advance(&mut self, now: Instant, visible: bool) -> bool {
        let mut changed = self.next_step() == Some(now)
            || self.accent_end() == Some(now) && !self.accent().moves();
        self.now = now;
        let idle = visible && !self.asleep;
        if self.expires_at.is_some_and(|t| t <= now) {
            self.expires_at = None;
            self.change(now, Expression::Neutral);
            changed = true;
        }
        if self.next_blink <= now {
            if idle {
                self.blink_at = Some(now);
                changed = true;
            }
            let double = idle
                && !self.second_blink
                && self.motion.range_inclusive(1, DOUBLE_BLINK_ONE_IN) == 1;
            self.second_blink = double;
            self.next_blink = if double {
                now + BLINK_DURATION + DOUBLE_BLINK_GAP
            } else {
                now + random(&mut self.motion, BLINK_INTERVAL_MIN, BLINK_INTERVAL_MAX)
            };
        }
        if self.next_glance <= now {
            if idle && !self.expression.holds_gaze() {
                let x = random_unit(&mut self.motion, 8);
                let y = random_unit(&mut self.motion, 4) * 0.8;
                let recenter = self.motion.range_inclusive(1, 100) <= GLANCE_RECENTER_PERCENT;
                let to = if recenter { (0.0, 0.0) } else { (x, y) };
                let from = self.glance(now);
                if !same_point(from, to) {
                    (self.glance_from, self.glance_to, self.glance_at) = (from, to, now);
                    changed = true;
                }
            }
            self.next_glance =
                now + random(&mut self.motion, GLANCE_INTERVAL_MIN, GLANCE_INTERVAL_MAX);
        }
        if self.next_squint <= now {
            if idle && self.expression == Expression::Neutral {
                self.squint_at = Some(now);
                changed = true;
            }
            self.next_squint =
                now + random(&mut self.motion, SQUINT_INTERVAL_MIN, SQUINT_INTERVAL_MAX);
        }
        if self.next_tear.is_some_and(|t| t <= now) {
            if visible {
                self.tear_at = Some(now);
                changed = true;
            }
            self.next_tear =
                Some(now + TEAR_FALL + random(&mut self.motion, TEAR_GAP_MIN, TEAR_GAP_MAX));
        }
        if self.next_twitch.is_some_and(|t| t <= now) {
            if visible {
                self.twitch_at = Some(now);
                changed = true;
            }
            self.next_twitch =
                Some(now + random(&mut self.motion, TWITCH_INTERVAL_MIN, TWITCH_INTERVAL_MAX));
        }
        visible && changed
    }

    /// Jump to `now` without replaying the transitions before it, after a gap
    /// longer than any platform is late (see `STALL_LIMIT`). A due expiry still
    /// applies and the idle schedule restarts from `now`. Every motion is a
    /// function of elapsed time with a fixed end, so all have long finished.
    pub fn fast_forward(&mut self, now: Instant) {
        if let Some(t) = self.expires_at.filter(|&t| t <= now) {
            self.expires_at = None;
            self.change(t, Expression::Neutral);
        }
        self.now = now;
        self.schedule_idle(now);
        self.next_tear = self.next_tear.map(|_| now + TEAR_DELAY);
        self.next_twitch = self
            .next_twitch
            .map(|_| now + random(&mut self.motion, TWITCH_INTERVAL_MIN, TWITCH_INTERVAL_MAX));
    }

    /// True while the pose is changing with time, so a frame is needed every
    /// `FRAME`. The end of each motion is also a transition, so its final frame
    /// is drawn. Stepped motion is not animation: each step is a transition.
    #[must_use]
    pub fn animating(&self, now: Instant) -> bool {
        let during = |start: Instant, length| running(start, length, now).is_some();
        let during_opt = |start: Option<Instant>, length| start.is_some_and(|t| during(t, length));
        now < self.settled_at()
            || (self.pop > 0.0 && during(self.changed_at, POP_DURATION))
            || self.blinking(now)
            || during(self.glance_at, GLANCE_DURATION)
            || during_opt(self.squint_at, SQUINT_DURATION)
            || during_opt(self.tear_at, TEAR_FALL)
            || during_opt(self.twitch_at, TWITCH_DURATION)
            || (self.accent().moves() && self.accent_end().is_some_and(|end| now < end))
    }

    /// A tap landed on Wade: Happy for `HAPPY_DURATION`, restarted by another
    /// tap, or, if asleep, Surprised for `WAKE_DURATION`.
    /// Returns true if the view may have changed.
    #[must_use = "a true result means the view must be redrawn"]
    pub fn on_tap(&mut self, now: Instant) -> bool {
        self.now = now;
        if self.asleep {
            self.change(now, Expression::Surprised);
            self.expires_at = Some(now + WAKE_DURATION);
        } else {
            if self.expression != Expression::Happy {
                self.change(now, Expression::Happy);
            }
            self.expires_at = Some(now + HAPPY_DURATION);
        }
        true
    }

    /// Show `expression` until something else changes it, waking him if
    /// asleep. Returns true if the view may have changed.
    #[must_use = "a true result means the view must be redrawn"]
    pub fn show(&mut self, expression: Expression, now: Instant) -> bool {
        self.now = now;
        if self.asleep || expression != self.expression {
            self.change(now, expression);
        }
        self.expires_at = None;
        true
    }

    /// Fall asleep until a tap or [`Wade::show`] wakes him.
    /// Returns true if the view may have changed.
    #[must_use = "a true result means the view must be redrawn"]
    pub fn sleep(&mut self, now: Instant) -> bool {
        self.now = now;
        if self.asleep {
            return false;
        }
        self.fall_asleep(now);
        true
    }

    #[must_use]
    pub const fn expression(&self) -> Expression {
        self.expression
    }

    #[must_use]
    pub const fn asleep(&self) -> bool {
        self.asleep
    }

    #[must_use]
    pub fn blinking(&self, now: Instant) -> bool {
        self.blink_at
            .is_some_and(|t| running(t, BLINK_DURATION, now).is_some())
    }

    /// The rendered pose at `now`: the base expression, then pop, squint, and
    /// twitch on eye height, glances, shake and breath, the accent, and the blink.
    #[must_use]
    pub fn pose(&self, now: Instant) -> Pose {
        let mut pose = self.base(now);
        let squint = self
            .squint_at
            .map_or(0.0, |t| falloff(t, SQUINT_DURATION, now));
        let twitch = self
            .twitch_at
            .map_or(0.0, |t| falloff(t, TWITCH_DURATION, now));
        pose.eye_size.1 *= (1.0 + self.pop * falloff(self.changed_at, POP_DURATION, now))
            * (1.0 - SQUINT_DEPTH * squint)
            * (1.0 + TWITCH * twitch);
        let glance = self.glance(now);
        pose.gaze = (pose.gaze.0 + glance.0, pose.gaze.1 + glance.1);
        let shake = self.shake(now);
        pose.face_offset = (pose.face_offset.0 + shake.0, pose.face_offset.1 + shake.1);
        (pose.accent, pose.accent_phase) = self.accent_at(now);
        pose.eye_open *= self.blink_open(now);
        pose
    }

    /// Draw the next blink, glance, and squint, in that order, from `now`.
    fn schedule_idle(&mut self, now: Instant) {
        let rng = &mut self.motion;
        self.second_blink = false;
        self.next_blink = now + random(rng, BLINK_INTERVAL_MIN, BLINK_INTERVAL_MAX);
        self.next_glance = now + random(rng, GLANCE_INTERVAL_MIN, GLANCE_INTERVAL_MAX);
        self.next_squint = now + random(rng, SQUINT_INTERVAL_MIN, SQUINT_INTERVAL_MAX);
    }

    /// Start a transition from the current base pose, forgetting motions that
    /// belong to the old state.
    fn begin_change(&mut self, now: Instant) {
        self.from = self.base(now);
        self.changed_at = now;
        self.tear_at = None;
        self.next_tear = None;
        self.twitch_at = None;
        self.next_twitch = None;
    }

    /// Move to `expression`, awake.
    fn change(&mut self, now: Instant, expression: Expression) {
        self.begin_change(now);
        self.expression = expression;
        self.asleep = false;
        self.transition = EXPRESSION_TRANSITION;
        self.pop = if expression == Expression::Surprised {
            POP_SURPRISED
        } else {
            POP
        };
        if expression.holds_gaze() {
            self.recenter_glance(now);
        }
        if expression == Expression::Sad {
            self.next_tear = Some(now + TEAR_DELAY);
        }
    }

    fn fall_asleep(&mut self, now: Instant) {
        self.begin_change(now);
        self.expression = Expression::Sleepy;
        self.asleep = true;
        self.expires_at = None;
        self.transition = SLEEP_TRANSITION;
        self.pop = 0.0;
        self.recenter_glance(now);
        self.next_twitch = Some(now + random(&mut self.motion, TWITCH_FIRST_MIN, TWITCH_FIRST_MAX));
    }

    fn recenter_glance(&mut self, now: Instant) {
        self.glance_from = self.glance(now);
        self.glance_to = (0.0, 0.0);
        self.glance_at = now;
    }

    /// When the base pose reaches its target, and the accent starts.
    fn settled_at(&self) -> Instant {
        self.changed_at + self.transition
    }

    fn target(&self) -> Pose {
        if self.asleep {
            ASLEEP
        } else {
            self.expression.pose()
        }
    }

    fn base(&self, now: Instant) -> Pose {
        let t = fraction(now.saturating_since(self.changed_at), self.transition);
        self.from.lerp(&self.target(), smoothstep(t))
    }

    fn glance(&self, now: Instant) -> (f32, f32) {
        let t = smoothstep(fraction(
            now.saturating_since(self.glance_at),
            GLANCE_DURATION,
        ));
        let (from, to) = (self.glance_from, self.glance_to);
        (from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t)
    }

    /// Angry trembles; asleep breathes.
    fn shake(&self, now: Instant) -> (f32, f32) {
        let elapsed = now.saturating_since(self.changed_at);
        if self.asleep {
            let (_, stepped) = on_grid(elapsed, SLEEP_STEP);
            let t = fraction(modulo(stepped, BREATH_PERIOD), BREATH_PERIOD);
            (0.0, BREATH_PX * libm::sinf(TAU * t))
        } else if self.expression == Expression::Angry {
            let (step, _) = on_grid(elapsed, SHAKE_STEP);
            (SHAKE_PX * jitter(step), 0.0)
        } else {
            (0.0, 0.0)
        }
    }

    /// The accent of the current state.
    const fn accent(&self) -> Accent {
        if self.asleep {
            Accent::Zs
        } else {
            self.expression.accent()
        }
    }

    /// The accent and its phase at `now`.
    fn accent_at(&self, now: Instant) -> (Accent, f32) {
        const NONE: (Accent, f32) = (Accent::None, 0.0);
        let settled = self.settled_at();
        if now < settled {
            return NONE;
        }
        let accent = self.accent();
        let since = now.saturating_since(settled);
        match accent {
            Accent::None => NONE,
            Accent::Blush => (accent, 1.0),
            // One that only shows holds a fixed phase, so the view stays still.
            Accent::Exclaim | Accent::Sparkle | Accent::SweatDrop => accent
                .once()
                .and_then(|length| running(settled, length, now))
                .map_or(NONE, |p| (accent, if accent.moves() { p } else { 1.0 })),
            Accent::Tear => self
                .tear_at
                .and_then(|t| running(t, TEAR_FALL, now))
                .map_or(NONE, |p| (accent, p)),
            Accent::Dots => {
                let (step, _) = on_grid(since, DOTS_STEP);
                let dots = u8::try_from((step + 1) % 4).unwrap_or(0);
                (accent, f32::from(dots) / 3.0)
            }
            Accent::Steam => {
                let (step, _) = on_grid(since, STEAM_STEP);
                if step % 2 == 0 { (accent, 1.0) } else { NONE }
            }
            Accent::Zs => {
                // Z's move on the sleep tick, which counts from falling asleep.
                let (_, stepped) = on_grid(now.saturating_since(self.changed_at), SLEEP_STEP);
                let Some(rising) = stepped.checked_sub(self.transition) else {
                    return NONE;
                };
                let within = fraction(modulo(rising, ZS_CYCLE), ZS_CYCLE);
                let phase = if rising < ZS_CYCLE {
                    within
                } else {
                    1.0 + within
                };
                (accent, phase)
            }
        }
    }

    /// When the current one-time accent ends.
    fn accent_end(&self) -> Option<Instant> {
        self.accent()
            .once()
            .map(|length| self.settled_at() + length)
    }

    /// The next step of a stepped motion.
    fn next_step(&self) -> Option<Instant> {
        let grid = |origin, period| next_on_grid(origin, period, self.now);
        if self.asleep {
            return Some(grid(self.changed_at, SLEEP_STEP));
        }
        let accent = self
            .accent()
            .step()
            .map(|period| grid(self.settled_at(), period));
        let shake =
            (self.expression == Expression::Angry).then(|| grid(self.changed_at, SHAKE_STEP));
        accent.into_iter().chain(shake).min()
    }

    fn blink_open(&self, now: Instant) -> f32 {
        let Some(start) = self.blink_at else {
            return 1.0;
        };
        if !self.blinking(now) {
            return 1.0;
        }
        let elapsed = now.saturating_since(start);
        match elapsed.checked_sub(BLINK_CLOSE) {
            None => 1.0 - fraction(elapsed, BLINK_CLOSE),
            Some(opening) => fraction(opening, BLINK_OPEN),
        }
    }
}

impl Accent {
    /// How long a one-time accent lasts, or `None` for one that holds or repeats.
    const fn once(self) -> Option<Duration> {
        match self {
            Accent::Exclaim => Some(EXCLAIM_DURATION),
            Accent::Sparkle => Some(SPARKLE_DURATION),
            Accent::SweatDrop => Some(SWEAT_DURATION),
            Accent::None
            | Accent::Blush
            | Accent::Tear
            | Accent::Dots
            | Accent::Steam
            | Accent::Zs => None,
        }
    }

    /// True for a one-time accent that moves, rather than just showing.
    const fn moves(self) -> bool {
        matches!(self, Accent::Sparkle | Accent::SweatDrop)
    }

    /// The step of an accent that moves in steps, counted from when it starts.
    /// Zs step with sleep's breath instead.
    const fn step(self) -> Option<Duration> {
        match self {
            Accent::Dots => Some(DOTS_STEP),
            Accent::Steam => Some(STEAM_STEP),
            Accent::None
            | Accent::Blush
            | Accent::Exclaim
            | Accent::Tear
            | Accent::Sparkle
            | Accent::SweatDrop
            | Accent::Zs => None,
        }
    }
}

/// A random duration in `lo..=hi`, to the millisecond.
fn random(rng: &mut Rng, lo: Duration, hi: Duration) -> Duration {
    Duration::from_millis(rng.range_inclusive(millis(lo), millis(hi)))
}

/// A random multiple of `1 / steps` in −1.0 … 1.0.
fn random_unit(rng: &mut Rng, steps: u8) -> f32 {
    let k = rng.range_inclusive(0, 2 * u64::from(steps));
    let k = u8::try_from(k).unwrap_or(steps);
    (f32::from(k) - f32::from(steps)) / f32::from(steps)
}

/// `part / whole`, clamped to 0.0 … 1.0.
fn fraction(part: Duration, whole: Duration) -> f32 {
    if part >= whole {
        1.0
    } else {
        part.div_duration_f32(whole)
    }
}

fn modulo(d: Duration, period: Duration) -> Duration {
    Duration::from_millis(millis(d) % millis(period).max(1))
}

/// Progress 0.0 … 1.0 through a motion from `start` lasting `length`, or
/// `None` before it starts and once it has ended.
fn running(start: Instant, length: Duration, now: Instant) -> Option<f32> {
    let elapsed = now.saturating_since(start);
    (now >= start && elapsed < length).then(|| fraction(elapsed, length))
}

/// 1.0 as a motion starts, easing to 0.0 as it ends.
fn falloff(start: Instant, length: Duration, now: Instant) -> f32 {
    running(start, length, now).map_or(0.0, |p| (1.0 - p) * (1.0 - p))
}

fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// How many whole `period`s fit in `elapsed`, and their total length.
fn on_grid(elapsed: Duration, period: Duration) -> (u64, Duration) {
    let p = millis(period).max(1);
    let steps = millis(elapsed) / p;
    (steps, Duration::from_millis(steps.saturating_mul(p)))
}

/// The first instant after `after` on the grid `origin + k × period`.
fn next_on_grid(origin: Instant, period: Duration, after: Instant) -> Instant {
    if after < origin {
        return origin;
    }
    let (steps, _) = on_grid(after.saturating_since(origin), period);
    origin + Duration::from_millis(steps.saturating_add(1).saturating_mul(millis(period)))
}

/// True if two gaze offsets are the same to well under a pixel.
fn same_point(a: (f32, f32), b: (f32, f32)) -> bool {
    (a.0 - b.0).abs() < 1e-4 && (a.1 - b.1).abs() < 1e-4
}

/// −1.0, 0.0, or 1.0: scattered, but fixed for each `step`.
fn jitter(step: u64) -> f32 {
    match step.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 62 {
        0 => -1.0,
        3 => 1.0,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(ms: u64) -> Instant {
        Instant::from_millis(ms)
    }

    /// Advance `wade` through every transition up to `until`, calling `each` after each.
    fn run(wade: &mut Wade, until: Instant, mut each: impl FnMut(&Wade, Instant)) {
        while let Some(t) = wade.next_transition(true).filter(|&t| t <= until) {
            let _ = wade.advance(t, true);
            each(wade, t);
        }
    }

    /// Show `expression` from `at`, as a key press would.
    fn hold(wade: &mut Wade, expression: Expression, at: Instant) {
        wade.now = at;
        wade.change(at, expression);
    }

    /// Push the idle schedule out of the way of a test.
    fn quiet(wade: &mut Wade) {
        wade.next_blink = Instant::MAX;
        wade.next_glance = Instant::MAX;
        wade.next_squint = Instant::MAX;
    }

    #[test]
    fn glances_pause_while_an_expression_holds_its_gaze() {
        let mut wade = Wade::new(ms(0), 3);
        hold(&mut wade, Expression::Thinking, ms(1_000));
        let held = Expression::Thinking.pose().gaze;
        run(&mut wade, ms(60_000), |wade, t| {
            if t >= ms(1_200) {
                assert_eq!(wade.pose(t).gaze, held, "gaze moved at {t:?}");
            }
        });
    }

    #[test]
    fn an_accent_waits_for_the_transition_to_finish() {
        let mut wade = Wade::new(ms(0), 3);
        hold(&mut wade, Expression::Happy, ms(1_000));
        let settled = ms(1_000) + EXPRESSION_TRANSITION;
        assert_eq!(wade.pose(ms(1_100)).accent, Accent::None);
        assert_eq!(wade.pose(settled).accent, Accent::Blush);
    }

    #[test]
    fn he_squints_only_in_neutral() {
        let mut happy = Wade::new(ms(0), 3);
        hold(&mut happy, Expression::Happy, ms(1_000));
        run(&mut happy, ms(120_000), |wade, t| {
            assert!(wade.squint_at.is_none(), "squinted at {t:?} while Happy");
        });
        let mut neutral = Wade::new(ms(0), 3);
        let mut squinted = false;
        run(&mut neutral, ms(120_000), |wade, _| {
            squinted |= wade.squint_at.is_some();
        });
        assert!(squinted, "no squint in two minutes of Neutral");
    }

    #[test]
    fn stepped_motion_is_not_animation() {
        for (expression, step) in [
            (Expression::Angry, SHAKE_STEP),
            (Expression::Thinking, DOTS_STEP),
        ] {
            let mut wade = Wade::new(ms(0), 3);
            hold(&mut wade, expression, ms(1_000));
            quiet(&mut wade);
            let _ = wade.advance(ms(2_000), true);
            assert!(!wade.animating(ms(2_010)), "{expression:?} animates");
            let next = wade.next_transition(true).expect("a next step");
            assert!(
                next > ms(2_000) && next <= ms(2_000) + step,
                "{expression:?}: next step at {next:?}"
            );
        }
    }

    #[test]
    fn next_on_grid_is_strictly_later() {
        let origin = Instant::from_millis(1_000);
        let period = Duration::from_millis(100);
        assert_eq!(
            next_on_grid(origin, period, Instant::from_millis(500)),
            origin
        );
        assert_eq!(
            next_on_grid(origin, period, origin),
            Instant::from_millis(1_100)
        );
        assert_eq!(
            next_on_grid(origin, period, Instant::from_millis(1_150)),
            Instant::from_millis(1_200)
        );
    }

    #[test]
    fn hidden_wade_schedules_no_steps() {
        let mut wade = Wade::new(ms(0), 1);
        hold(&mut wade, Expression::Angry, ms(1_000));
        quiet(&mut wade);
        wade.next_blink = ms(60_000);
        let _ = wade.advance(ms(3_000), true);
        // Angry trembles every 100 ms from the change.
        assert_eq!(wade.next_transition(true), Some(ms(3_100)));
        assert_eq!(wade.next_transition(false), Some(ms(60_000)));
    }

    #[test]
    fn fast_forward_replays_no_accent_and_starts_no_motion() {
        let mut wade = Wade::new(ms(0), 1);
        hold(&mut wade, Expression::Surprised, ms(1_000));
        let later = ms(1_000 + 2 * 60 * 60 * 1_000);
        wade.fast_forward(later);
        assert_eq!(wade.pose(later).accent, Accent::None, "the \"!\" came back");
        assert!(!wade.animating(later), "moving right after a fast-forward");
        assert!(wade.next_transition(true).is_some_and(|t| t > later));
    }

    #[test]
    fn blink_closes_then_reopens() {
        let mut wade = Wade::new(ms(0), 1);
        let start = wade.next_blink;
        let _ = wade.advance(start, true);
        let open = |ms| wade.blink_open(start + Duration::from_millis(ms));
        assert!((open(0) - 1.0).abs() < 1e-6, "not open as the blink starts");
        assert!(open(50) < 1e-6, "not shut after 50 ms");
        assert!(open(85) > 0.0 && open(85) < 1.0, "not reopening at 85 ms");
        assert!(
            (open(120) - 1.0).abs() < 1e-6,
            "not open once the blink ends"
        );
    }
}
