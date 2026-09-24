//! Wade: state, expressions, animation, and the pose rig (docs/character.md).

mod pose;

pub use pose::{Accent, Pose, Prop};

use crate::rng::Rng;
use crate::time::{Duration, Instant};

/// How long Wade stays Happy after a tap on him.
pub const HAPPY_DURATION: Duration = Duration::from_millis(2_000);
/// How long a blink takes, closing and reopening.
pub const BLINK_DURATION: Duration = Duration::from_millis(120);
/// Blinks start at random intervals between these bounds (inclusive).
pub const BLINK_INTERVAL_MIN: Duration = Duration::from_millis(2_000);
pub const BLINK_INTERVAL_MAX: Duration = Duration::from_millis(6_000);
/// How long the pose takes to move to a new expression's target.
pub const EXPRESSION_TRANSITION: Duration = Duration::from_millis(200);

/// A named target pose.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Expression {
    #[default]
    Neutral,
    Happy,
}

impl Expression {
    /// The target pose for this expression.
    #[must_use]
    pub const fn pose(self) -> Pose {
        match self {
            // TODO(M1): tune the placeholder poses.
            Expression::Neutral => Pose {
                mouth_curve: 0.3,
                ..Pose::REST
            },
            Expression::Happy => Pose {
                eye_open: 0.8,
                brow_raise: 0.5,
                mouth_curve: 1.0,
                ..Pose::REST
            },
        }
    }
}

/// Wade's character state. Kept on every screen; animates only while visible.
#[derive(Clone, Debug)]
pub struct Wade {
    expression: Expression,
    // TODO(M1): when Happy expires, the next blink, the blink in progress, and the
    // expression transition in progress.
}

impl Wade {
    pub fn new(_now: Instant, _rng: &mut Rng) -> Self {
        // TODO(M1): schedule the first blink from `rng`.
        Wade {
            expression: Expression::Neutral,
        }
    }

    /// The next scheduled discrete change (a blink starting or ending, Happy
    /// expiring, an expression transition finishing), or `None`.
    ///
    /// Must be strictly later than the last instant passed to `advance`, unless
    /// the schedule has saturated at `Instant::MAX`. Frames are not transitions:
    /// they come from [`Wade::animating`].
    #[must_use]
    pub fn next_transition(&self) -> Option<Instant> {
        // TODO(M1)
        None
    }

    /// Apply every transition due at `now`. `visible` is false while another
    /// screen is shown: the schedule still moves forward, but due blinks are
    /// skipped (docs/architecture.md#hidden-features).
    /// Returns true if the view may have changed.
    #[must_use = "a true result means the view must be redrawn"]
    pub fn advance(&mut self, _now: Instant, _rng: &mut Rng, _visible: bool) -> bool {
        // TODO(M1)
        false
    }

    /// True while the pose is changing with time (a blink or an expression
    /// transition), so a frame is needed every `FRAME`. The end of each
    /// animation must also be a transition, so its final frame is drawn.
    #[must_use]
    pub fn animating(&self, _now: Instant) -> bool {
        // TODO(M1)
        false
    }

    /// A tap landed on Wade. Returns true if the view may have changed.
    #[must_use = "a true result means the view must be redrawn"]
    pub fn on_tap(&mut self, _now: Instant) -> bool {
        // TODO(M1): Happy for HAPPY_DURATION; another tap restarts it.
        false
    }

    #[must_use]
    pub fn expression(&self) -> Expression {
        self.expression
    }

    #[must_use]
    pub fn blinking(&self, _now: Instant) -> bool {
        // TODO(M1)
        false
    }

    /// The rendered pose at `now`: base expression, then blink.
    #[must_use]
    pub fn pose(&self, _now: Instant) -> Pose {
        // TODO(M1): ease between expressions and apply the blink layer.
        self.expression.pose()
    }
}
