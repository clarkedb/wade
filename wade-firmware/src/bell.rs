//! The core's tone sequences as audio samples, for platforms without a
//! synthesizer. Each note is a sine with its octave, struck like a bell with a
//! quick attack and an exponential decay, as on desktop.

use core::f32::consts::TAU;

use wade_core::sound::Tone;

/// The octave overtone's share of a note, for a brighter, bell-like tone.
const OVERTONE: f32 = 0.2;
/// Seconds each note takes to rise, and to fade at its end, so notes start and
/// stop without clicks.
const ATTACK: f32 = 0.004;
const RELEASE: f32 = 0.006;

/// How many samples `tone` lasts at `rate` hertz.
#[must_use]
pub fn sample_count(tone: &Tone, rate: u32) -> usize {
    let count = tone.duration.as_micros() * u128::from(rate) / 1_000_000;
    usize::try_from(count).unwrap_or(usize::MAX)
}

/// How many samples `tones` last at `rate` hertz.
#[must_use]
pub fn total_samples(tones: &[Tone], rate: u32) -> usize {
    tones.iter().map(|t| sample_count(t, rate)).sum()
}

/// `tones` at `rate` hertz as 16-bit samples peaking at `volume` of full scale.
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "sample counts and rates are far below 2^24, and samples stay within i16"
)]
pub fn samples(tones: &[Tone], rate: u32, volume: f32) -> impl Iterator<Item = i16> {
    tones.iter().flat_map(move |tone| {
        let length = tone.duration.as_micros() as f32 / 1_000_000.0;
        // Most of the note has rung away by its end.
        let decay = length / 4.0;
        let hz = f32::from(tone.hz);
        (0..sample_count(tone, rate)).map(move |i| {
            let t = i as f32 / rate as f32;
            let envelope =
                (t / ATTACK).min(1.0) * ((length - t) / RELEASE).min(1.0) * libm::expf(-t / decay);
            let wave = libm::sinf(TAU * hz * t) + OVERTONE * libm::sinf(2.0 * TAU * hz * t);
            (volume / (1.0 + OVERTONE) * envelope * wave * f32::from(i16::MAX)) as i16
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wade_core::sound::CHIME;

    #[test]
    fn the_chime_lasts_its_notes_and_starts_and_ends_silent() {
        assert_eq!(sample_count(&Tone::new(440, 1_000), 16_000), 16_000);
        assert_eq!(total_samples(&CHIME, 16_000), 12_000);
        assert_eq!(samples(&CHIME, 16_000, 0.5).count(), 12_000);
        let peak = samples(&CHIME, 16_000, 0.5)
            .map(i16::unsigned_abs)
            .max()
            .unwrap();
        let full = i16::MAX.unsigned_abs();
        assert!(peak <= full / 2, "louder than volume");
        assert!(peak > full / 4, "much quieter than volume");
        let first = samples(&CHIME, 16_000, 0.5).next().unwrap();
        let last = samples(&CHIME, 16_000, 0.5).last().unwrap();
        assert!(first.unsigned_abs() < 50 && last.unsigned_abs() < 400);
    }
}
