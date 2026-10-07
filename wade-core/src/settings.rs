//! Settings: what the person at the desk has chosen, and how it is stored
//! (docs/roadmap.md#m5-settings).

use crate::time::Duration;
use crate::timer::{DEFAULT_SET, MAX_SET, MIN_SET};
use crate::view::{ColorMode, EyeStyle};

/// The length of an encoding: the version, then one byte each for the eye
/// style, colors, chime, timer minutes, and brightness.
pub const ENCODED_LEN: usize = 6;

/// The first byte of every encoding. A new layout gets a new version, and
/// decoding keeps reading every older one, so stored settings and recordings
/// survive the change.
const VERSION: u8 = 2;
/// The layout before brightness, which decodes at full brightness.
const VERSION_1: u8 = 1;

/// A toggle on the Settings screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SettingsButton {
    /// Switch between pupils and plain eyes.
    EyeStyle,
    /// Switch between color and mono.
    Color,
    /// Turn the chime on or off.
    Chime,
    /// Step the backlight down a level, from the lowest back to full.
    Brightness,
}

/// The display's backlight level.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Brightness {
    Quarter,
    Half,
    ThreeQuarters,
    #[default]
    Full,
}

impl Brightness {
    /// Every level, dimmest first.
    pub const ALL: [Brightness; 4] = [
        Brightness::Quarter,
        Brightness::Half,
        Brightness::ThreeQuarters,
        Brightness::Full,
    ];

    /// The backlight's share of full power.
    #[must_use]
    pub const fn percent(self) -> u8 {
        match self {
            Brightness::Quarter => 25,
            Brightness::Half => 50,
            Brightness::ThreeQuarters => 75,
            Brightness::Full => 100,
        }
    }

    /// The next level down, or full after the lowest.
    #[must_use]
    pub const fn dimmer(self) -> Brightness {
        match self {
            Brightness::Full => Brightness::ThreeQuarters,
            Brightness::ThreeQuarters => Brightness::Half,
            Brightness::Half => Brightness::Quarter,
            Brightness::Quarter => Brightness::Full,
        }
    }

    const fn encode(self) -> u8 {
        match self {
            Brightness::Quarter => 0,
            Brightness::Half => 1,
            Brightness::ThreeQuarters => 2,
            Brightness::Full => 3,
        }
    }

    const fn decode(byte: u8) -> Option<Brightness> {
        match byte {
            0 => Some(Brightness::Quarter),
            1 => Some(Brightness::Half),
            2 => Some(Brightness::ThreeQuarters),
            3 => Some(Brightness::Full),
            _ => None,
        }
    }
}

/// Every setting. Always valid: the timer is whole minutes from 1 to 99.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Settings {
    eye_style: EyeStyle,
    color: ColorMode,
    chime: bool,
    timer_minutes: u8,
    brightness: Brightness,
}

impl Settings {
    pub const DEFAULT: Settings = Settings {
        eye_style: EyeStyle::Pupils,
        color: ColorMode::Color,
        chime: true,
        timer_minutes: whole_minutes(DEFAULT_SET).expect("the default timer is whole minutes"),
        brightness: Brightness::Full,
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

    /// The timer's duration as last set, which it starts at.
    #[must_use]
    pub fn timer(&self) -> Duration {
        Duration::from_mins(u64::from(self.timer_minutes))
    }

    #[must_use]
    pub const fn brightness(&self) -> Brightness {
        self.brightness
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

    #[must_use]
    pub const fn with_brightness(self, brightness: Brightness) -> Settings {
        Settings { brightness, ..self }
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

    /// These settings with `button`'s setting switched.
    #[must_use]
    pub const fn toggled(self, button: SettingsButton) -> Settings {
        match button {
            SettingsButton::EyeStyle => self.with_eye_style(self.eye_style.toggled()),
            SettingsButton::Color => self.with_color(self.color.toggled()),
            SettingsButton::Chime => self.with_chime(!self.chime),
            SettingsButton::Brightness => self.with_brightness(self.brightness.dimmer()),
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
            self.brightness.encode(),
        ]
    }

    /// Settings from their stored form. Missing, corrupt, or unknown-version
    /// data gives the defaults.
    #[must_use]
    pub fn decode(bytes: &[u8]) -> Settings {
        Settings::try_decode(bytes).unwrap_or_default()
    }

    /// Settings from their stored form, or `None` unless `bytes` is exactly
    /// an encoding of some settings, in this layout or an older one.
    #[must_use]
    pub fn try_decode(bytes: &[u8]) -> Option<Settings> {
        let (eye_style, color, chime, minutes, brightness) = match *bytes {
            [VERSION, eye_style, color, chime, minutes, brightness] => (
                eye_style,
                color,
                chime,
                minutes,
                Brightness::decode(brightness)?,
            ),
            [VERSION_1, eye_style, color, chime, minutes] => {
                (eye_style, color, chime, minutes, Brightness::Full)
            }
            _ => return None,
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
            brightness,
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
        assert_eq!(Settings::default().encode(), [VERSION, 0, 0, 1, 5, 3]);
    }

    #[test]
    fn the_first_layout_still_decodes_at_full_brightness() {
        let v1 = [VERSION_1, 1, 1, 0, 12];
        let expected = Settings::DEFAULT
            .with_eye_style(EyeStyle::Plain)
            .with_color(ColorMode::Mono)
            .with_chime(false)
            .with_timer(Duration::from_mins(12))
            .unwrap();
        assert_eq!(Settings::try_decode(&v1), Some(expected));
    }

    #[test]
    fn every_brightness_round_trips() {
        for b in Brightness::ALL {
            let s = Settings::DEFAULT.with_brightness(b);
            assert_eq!(Settings::try_decode(&s.encode()), Some(s));
        }
    }

    #[test]
    fn brightness_steps_down_and_wraps_to_full() {
        let mut b = Brightness::Full;
        let mut seen = [0u8; 4];
        for (i, slot) in seen.iter_mut().enumerate() {
            b = b.dimmer();
            *slot = b.percent();
            assert_eq!(i == 3, b == Brightness::Full);
        }
        assert_eq!(seen, [75, 50, 25, 100]);
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
    fn each_toggle_switches_its_setting_and_back() {
        for (button, changed) in [
            (
                SettingsButton::EyeStyle,
                Settings::DEFAULT.with_eye_style(EyeStyle::Plain),
            ),
            (
                SettingsButton::Color,
                Settings::DEFAULT.with_color(ColorMode::Mono),
            ),
            (SettingsButton::Chime, Settings::DEFAULT.with_chime(false)),
        ] {
            let s = Settings::DEFAULT.toggled(button);
            assert_eq!(s, changed);
            assert_eq!(s.toggled(button), Settings::DEFAULT);
        }
        let s = Settings::DEFAULT.toggled(SettingsButton::Brightness);
        assert_eq!(
            s,
            Settings::DEFAULT.with_brightness(Brightness::ThreeQuarters)
        );
    }

    #[test]
    fn bad_data_decodes_to_the_defaults() {
        let good = Settings::DEFAULT.with_chime(false).encode();
        assert_eq!(Settings::decode(&good), Settings::DEFAULT.with_chime(false));
        for (bytes, why) in [
            (&[][..], "missing"),
            (&good[..5], "short"),
            (&[VERSION, 0, 0, 0, 5, 3, 0][..], "long"),
            (&[VERSION_1, 0, 0, 0, 5, 3][..], "first layout, too long"),
            (&[3, 0, 0, 0, 5, 3][..], "unknown version"),
            (&[VERSION, 2, 0, 0, 5, 3][..], "bad eye style"),
            (&[VERSION, 0, 2, 0, 5, 3][..], "bad colors"),
            (&[VERSION, 0, 0, 2, 5, 3][..], "bad chime"),
            (&[VERSION, 0, 0, 0, 0, 3][..], "timer too short"),
            (&[VERSION, 0, 0, 0, 100, 3][..], "timer too long"),
            (&[VERSION, 0, 0, 0, 5, 4][..], "bad brightness"),
        ] {
            assert_eq!(Settings::try_decode(bytes), None, "{why}");
            assert_eq!(Settings::decode(bytes), Settings::DEFAULT, "{why}");
        }
    }
}
