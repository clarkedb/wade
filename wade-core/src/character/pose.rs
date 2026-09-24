/// Every expression and animation reduces to a `Pose`. Drawing reads only a `Pose`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    /// Width and height of each eye in pixels, before blinking
    pub eye_size: (f32, f32),
    /// Corner radius in pixels
    pub eye_radius: f32,
    /// 0.0 closed … 1.0 open; closing collapses each eye to a line
    pub eye_open: f32,
    /// 0.0 … 1.0 of each eye covered from above (droop)
    pub upper_lid: f32,
    /// 0.0 … 1.0 of each eye pushed up from below into a crescent (smile)
    pub lower_lid: f32,
    /// −1.0 outer corners cut (worried) … 1.0 inner corners cut (angry)
    pub brow_tilt: f32,
    /// −1.0 right eye shorter … 1.0 right eye taller (smug)
    pub asymmetry: f32,
    /// Where both eyes look, −1.0 … 1.0 on each axis
    pub gaze: (f32, f32),
    /// Whole-face offset in pixels: shaking and breathing
    pub face_offset: (f32, f32),
    pub accent: Accent,
    /// 0.0 … 1.0 through the accent's motion; Zs runs to 2.0 (see [`Accent`])
    pub accent_phase: f32,
    pub prop: Prop,
    /// 0.0 … 1.0 through the prop's motion
    pub prop_phase: f32,
}

impl Pose {
    /// Eyes open and level, looking ahead.
    pub const REST: Pose = Pose {
        eye_size: (85.0, 85.0),
        eye_radius: 30.0,
        eye_open: 1.0,
        upper_lid: 0.0,
        lower_lid: 0.0,
        brow_tilt: 0.0,
        asymmetry: 0.0,
        gaze: (0.0, 0.0),
        face_offset: (0.0, 0.0),
        accent: Accent::None,
        accent_phase: 0.0,
        prop: Prop::None,
        prop_phase: 0.0,
    };

    /// The continuous parameters of `self` moved `t` (0.0 … 1.0) of the way to
    /// `to`. Accent and prop come from `to`.
    #[must_use]
    pub fn lerp(&self, to: &Pose, t: f32) -> Pose {
        let mix = |a: f32, b: f32| a + (b - a) * t;
        let mix2 = |a: (f32, f32), b: (f32, f32)| (mix(a.0, b.0), mix(a.1, b.1));
        Pose {
            eye_size: mix2(self.eye_size, to.eye_size),
            eye_radius: mix(self.eye_radius, to.eye_radius),
            eye_open: mix(self.eye_open, to.eye_open),
            upper_lid: mix(self.upper_lid, to.upper_lid),
            lower_lid: mix(self.lower_lid, to.lower_lid),
            brow_tilt: mix(self.brow_tilt, to.brow_tilt),
            asymmetry: mix(self.asymmetry, to.asymmetry),
            gaze: mix2(self.gaze, to.gaze),
            face_offset: mix2(self.face_offset, to.face_offset),
            accent: to.accent,
            accent_phase: mix(self.accent_phase, to.accent_phase),
            prop: to.prop,
            prop_phase: mix(self.prop_phase, to.prop_phase),
        }
    }
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

/// A small secondary action beside the eyes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Accent {
    #[default]
    None,
    /// Pink marks under the outer corners (Happy)
    Blush,
    /// A "!" beside the face (Surprised)
    Exclaim,
    /// A tear falls from the right eye (Sad)
    Tear,
    /// Up to three dots; the phase times three is how many (Thinking)
    Dots,
    /// Steam marks above the outer corners (Angry)
    Steam,
    /// One twinkle beside the right eye (Proud)
    Sparkle,
    /// A drop slides down beside the left eye (Flustered)
    SweatDrop,
    /// Z's rise and grow (asleep). 0.0 … 1.0 while the first rises; from 1.0,
    /// the fraction is the newest Z's rise, with an older one still in the air.
    Zs,
}
