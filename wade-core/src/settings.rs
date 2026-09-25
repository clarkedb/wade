//! Settings: what the person at the desk has chosen, and how it is stored
//! (docs/roadmap.md#m5-settings).

use crate::time::Duration;
use crate::timer::{DEFAULT_SET, MAX_SET, MIN_SET};
use crate::view::{ColorMode, EyeStyle};

/// The length of an encoding: the version, then one byte each for the eye
/// style, colors, chime, and timer minutes.
pub const ENCODED_LEN: usize = 5;

/// The first byte of every encoding. A new layout gets a new version, and
/// decoding keeps reading every older one, so stored settings and recordings
/// survive the change.
const VERSION: u8 = 1;

/// Every setting. Always valid: the timer is whole minutes from 1 to 99.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Settings {
    eye_style: EyeStyle,
    color: ColorMode,
    chime: bool,
    timer_minutes: u8,
}

impl Settings {
    pub const DEFAULT: Settings = Settings {
        eye_style: EyeStyle::Pupils,
        color: ColorMode::Color,
        chime: true,
        timer_minutes: whole_minutes(DEFAULT_SET).expect("the default timer is whole minutes"),
    };

    #[must_use]
    pub const fn eye_style(&self) -> EyeStyle {
        self.eye_style
    }

    #[must_use]
    pub const fn color(&self) -> ColorMode {
        self.color
    }

    /// False silences the timer's chime.
    #[must_use]
    pub const fn chime(&self) -> bool {
        self.chime
    }

    /// The duration the timer is set to while Ready.
    #[must_use]
    pub fn timer(&self) -> Duration {
        Duration::from_mins(u64::from(self.timer_minutes))
    }

    #[must_use]
    pub const fn with_eye_style(self, eye_style: EyeStyle) -> Settings {
        Settings { eye_style, ..self }
    }

    #[must_use]
    pub const fn with_color(self, color: ColorMode) -> Settings {
        Settings { color, ..self }
    }

    #[must_use]
    pub const fn with_chime(self, chime: bool) -> Settings {
        Settings { chime, ..self }
    }

    /// These settings with the timer at `timer`, or `None` unless it is whole
    /// minutes from 1 to 99.
    #[must_use]
    pub const fn with_timer(self, timer: Duration) -> Option<Settings> {
        match whole_minutes(timer) {
            Some(timer_minutes) => Some(Settings {
                timer_minutes,
                ..self
            }),
            None => None,
        }
    }

    /// The stored form: a version byte, then one byte per setting.
    #[must_use]
    pub fn encode(&self) -> [u8; ENCODED_LEN] {
        let eye_style = match self.eye_style {
            EyeStyle::Pupils => 0,
            EyeStyle::Plain => 1,
        };
        let color = match self.color {
            ColorMode::Color => 0,
            ColorMode::Mono => 1,
        };
        [
            VERSION,
            eye_style,
            color,
            u8::from(self.chime),
            self.timer_minutes,
        ]
    }

    /// Settings from their stored form. Missing, corrupt, or unknown-version
    /// data gives the defaults.
    #[must_use]
    pub fn decode(bytes: &[u8]) -> Settings {
        Settings::try_decode(bytes).unwrap_or_default()
    }

    /// Settings from their stored form, or `None` unless `bytes` is exactly
    /// an encoding of some settings.
    #[must_use]
    pub fn try_decode(bytes: &[u8]) -> Option<Settings> {
        let &[VERSION, eye_style, color, chime, minutes] = bytes else {
            return None;
        };
        let eye_style = match eye_style {
            0 => EyeStyle::Pupils,
            1 => EyeStyle::Plain,
            _ => return None,
        };
        let color = match color {
            0 => ColorMode::Color,
            1 => ColorMode::Mono,
            _ => return None,
        };
        let chime = match chime {
            0 => false,
            1 => true,
            _ => return None,
        };
        Settings {
            eye_style,
            color,
            chime,
            ..Settings::DEFAULT
        }
        .with_timer(Duration::from_mins(u64::from(minutes)))
    }
}

/// `timer` in minutes, or `None` unless it is whole minutes from 1 to 99.
#[expect(
    clippy::cast_possible_truncation,
    reason = "at most 99 once checked against the longest timer"
)]
const fn whole_minutes(timer: Duration) -> Option<u8> {
    let secs = timer.as_secs();
    let whole = timer.subsec_nanos() == 0 && secs.is_multiple_of(60);
    if whole && secs >= MIN_SET.as_secs() && secs <= MAX_SET.as_secs() {
        Some((secs / 60) as u8)
    } else {
        None
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stored_form_of_the_defaults_never_changes() {
        assert_eq!(Settings::default().encode(), [VERSION, 0, 0, 1, 5]);
    }

    #[test]
    fn the_timer_is_whole_minutes_from_one_to_ninety_nine() {
        let s = Settings::DEFAULT;
        assert_eq!(s.with_timer(MIN_SET).map(|s| s.timer()), Some(MIN_SET));
        assert_eq!(s.with_timer(MAX_SET).map(|s| s.timer()), Some(MAX_SET));
        assert_eq!(s.with_timer(Duration::ZERO), None);
        assert_eq!(s.with_timer(MAX_SET + MIN_SET), None);
        assert_eq!(s.with_timer(Duration::from_secs(90)), None);
        assert_eq!(s.with_timer(MIN_SET + Duration::from_nanos(1)), None);
    }

    #[test]
    fn bad_data_decodes_to_the_defaults() {
        let good = Settings::DEFAULT.with_chime(false).encode();
        assert_eq!(Settings::decode(&good), Settings::DEFAULT.with_chime(false));
        for (bytes, why) in [
            (&[][..], "missing"),
            (&good[..4], "short"),
            (&[VERSION, 0, 0, 0, 5, 0][..], "long"),
            (&[2, 0, 0, 0, 5][..], "unknown version"),
            (&[VERSION, 2, 0, 0, 5][..], "bad eye style"),
            (&[VERSION, 0, 2, 0, 5][..], "bad colors"),
            (&[VERSION, 0, 0, 2, 5][..], "bad chime"),
            (&[VERSION, 0, 0, 0, 0][..], "timer too short"),
            (&[VERSION, 0, 0, 0, 100][..], "timer too long"),
        ] {
            assert_eq!(Settings::try_decode(bytes), None, "{why}");
            assert_eq!(Settings::decode(bytes), Settings::DEFAULT, "{why}");
        }
    }
}
