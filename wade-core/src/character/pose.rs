/// Every expression and animation reduces to a `Pose`. Drawing reads only a `Pose`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    /// 0.0 closed … 1.0 open.
    pub eye_open: f32,
    /// Where Wade looks, −1.0 … 1.0 on each axis
    pub gaze: (f32, f32),
    /// −1.0 lowered … 1.0 raised
    pub brow_raise: f32,
    /// −1.0 worried … 1.0 determined
    pub brow_tilt: f32,
    /// 0.0 even … 1.0 one brow fully raised (skeptical)
    pub brow_asymmetry: f32,
    /// −1.0 frown … 1.0 smile
    pub mouth_curve: f32,
    /// 0.0 closed … 1.0 open
    pub mouth_open: f32,
    /// −1.0 … 1.0, lopsided (smirk)
    pub mouth_skew: f32,
    /// Small bob or lean, in pixels
    pub head_offset: (f32, f32),
    pub prop: Prop,
    /// 0.0 … 1.0 through the prop's motion
    pub prop_phase: f32,
}

impl Pose {
    /// Eyes open, everything else centred.
    pub const REST: Pose = Pose {
        eye_open: 1.0,
        gaze: (0.0, 0.0),
        brow_raise: 0.0,
        brow_tilt: 0.0,
        brow_asymmetry: 0.0,
        mouth_curve: 0.0,
        mouth_open: 0.0,
        mouth_skew: 0.0,
        head_offset: (0.0, 0.0),
        prop: Prop::None,
        prop_phase: 0.0,
    };
}

impl Default for Pose {
    fn default() -> Self {
        Pose::REST
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Prop {
    #[default]
    None,
    Keyboard,
    Cup,
}
