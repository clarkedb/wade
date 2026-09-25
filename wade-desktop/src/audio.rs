//! Desktop audio: synthesizes the core's tone sequences and plays them through
//! `rodio` on its own thread, so playing never blocks the loop (D17).

use std::f32::consts::TAU;

use rodio::buffer::SamplesBuffer;
use rodio::math::nz;
use rodio::{ChannelCount, DeviceSinkBuilder, MixerDeviceSink, SampleRate};
use wade_core::sound::{self, Tone};

const SAMPLE_RATE: SampleRate = nz!(44_100);
const MONO: ChannelCount = nz!(1);
/// Peak amplitude of a note, fundamental and overtone together.
const VOLUME: f32 = 0.3;
/// The octave overtone's share of a note, for a brighter, bell-like tone.
const OVERTONE: f32 = 0.2;
/// Seconds each note takes to rise, and to fade at its end, so notes start
/// and stop without clicks.
const ATTACK: f32 = 0.004;
const RELEASE: f32 = 0.006;

pub struct Audio {
    // Playback stops when the device sink is dropped.
    sink: MixerDeviceSink,
    chime: SamplesBuffer,
}

impl Audio {
    /// Open the default output device. Without one, say why and return `None`,
    /// so Wade runs silent.
    pub fn open() -> Option<Audio> {
        match DeviceSinkBuilder::open_default_sink() {
            Ok(mut sink) => {
                sink.log_on_drop(false);
                let chime = SamplesBuffer::new(MONO, SAMPLE_RATE, synthesize(&sound::CHIME));
                Some(Audio { sink, chime })
            }
            Err(error) => {
                eprintln!("no audio output, so the chime is silent: {error}");
                None
            }
        }
    }

    /// Start the chime and return at once. Chimes that overlap mix.
    pub fn chime(&self) {
        self.sink.mixer().add(self.chime.clone());
    }
}

/// `tones` as mono samples: each a sine with its octave, struck like a bell
/// with a quick attack and an exponential decay.
#[expect(
    clippy::cast_precision_loss,
    reason = "sample counts and rates are far below 2^24"
)]
fn synthesize(tones: &[Tone]) -> Vec<f32> {
    let rate = SAMPLE_RATE.get() as f32;
    let mut samples = Vec::with_capacity(tones.iter().map(sample_count).sum());
    for tone in tones {
        let length = tone.duration.as_secs_f32();
        // Most of the note has rung away by its end.
        let decay = length / 4.0;
        let hz = f32::from(tone.hz);
        samples.extend((0..sample_count(tone)).map(|i| {
            let t = i as f32 / rate;
            let envelope =
                (t / ATTACK).min(1.0) * ((length - t) / RELEASE).min(1.0) * (-t / decay).exp();
            let wave = (TAU * hz * t).sin() + OVERTONE * (2.0 * TAU * hz * t).sin();
            VOLUME / (1.0 + OVERTONE) * envelope * wave
        }));
    }
    samples
}

/// How many samples `tone` lasts.
fn sample_count(tone: &Tone) -> usize {
    let count = tone.duration.as_micros() * u128::from(SAMPLE_RATE.get()) / 1_000_000;
    usize::try_from(count).unwrap_or(usize::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_chime_lasts_its_notes_and_starts_and_ends_silent() {
        assert_eq!(sample_count(&Tone::new(440, 1_000)), 44_100);
        let samples = synthesize(&sound::CHIME);
        let count: usize = sound::CHIME.iter().map(sample_count).sum();
        assert_eq!(samples.len(), count);
        assert!(
            samples.iter().all(|s| s.abs() <= VOLUME),
            "a sample is louder than VOLUME"
        );
        assert!(samples[0].abs() < 1e-3 && samples[count - 1].abs() < 1e-2);
    }
}
