//! The sounds of the daemon, synthesized in memory as WAV files.
//!
//! Each sound is two notes: a rising pair when a tunnel woke, a falling pair
//! when it could not. The audio is built once per sound and event and lives
//! for the rest of the process, so a player may keep a pointer to it while it
//! plays asynchronously.
//!
//! A note rings until it has decayed to silence: the sound is cut where its
//! amplitude falls under [`DECAYED_AMPLITUDE`] of its peak, never earlier.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, OnceLock, PoisonError};

use engine::{MAX_STANDBY_VOLUME, MIN_STANDBY_VOLUME, StandbySound};

use super::SoundEvent;

const SAMPLE_RATE: u32 = 44_100;
const BITS_PER_SAMPLE: u16 = 16;
const CHANNEL_COUNT: u16 = 1;
const WAV_HEADER_BYTES: u32 = 44;
const PCM_FORMAT: u16 = 1;
const PEAK_AMPLITUDE: f64 = 0.5;
const FULL_SCALE: f64 = 32_767.0;
const END_FADE_SECONDS: f64 = 0.02;
const PERCENT: f64 = 100.0;

/// The amplitude, relative to the peak of the sound, under which a sound is
/// taken as over: -50 dB.
const DECAYED_AMPLITUDE: f64 = 0.003_162_277_7;

/// The longest a note is rendered, so that a mistake in a decay cannot make a
/// huge buffer. A note that is still ringing at this point is cut. The slowest
/// note, the chime, needs about 1.7 seconds to decay.
const MAX_NOTE_SECONDS: f64 = 3.0;

/// The silence appended after the sound. Some players and drivers (`PlaySound`
/// in asynchronous mode, `pw-play`) stop as soon as they have consumed the
/// data, before the device buffer has drained, which eats the last
/// milliseconds of the sound; the silence is what they eat instead.
const PLAYBACK_PADDING_SECONDS: f64 = 0.15;

const E4: f64 = 329.63;
const G4: f64 = 392.00;
const A4: f64 = 440.00;
const B4: f64 = 493.88;
const C5: f64 = 523.25;
const D4: f64 = 293.66;
const E5: f64 = 659.25;
const G5: f64 = 783.99;
const B5: f64 = 987.77;
const B3: f64 = 246.94;

const CHIME_ATTACK_SECONDS: f64 = 0.004;
/// Ratio to the fundamental, amplitude, and decay time constant in seconds.
const CHIME_PARTIALS: [(f64, f64, f64); 4] = [
    (1.0, 1.0, 0.3),
    (2.0, 0.35, 0.15),
    (3.01, 0.12, 0.083),
    (4.2, 0.05, 0.05),
];

const MARIMBA_BODY_ATTACK_SECONDS: f64 = 0.002;
const MARIMBA_BODY_DECAY_SECONDS: f64 = 0.22;
const MARIMBA_KNOCK_RATIO: f64 = 3.93;
const MARIMBA_KNOCK_AMPLITUDE: f64 = 0.25;
const MARIMBA_KNOCK_ATTACK_SECONDS: f64 = 0.001;
const MARIMBA_KNOCK_DECAY_SECONDS: f64 = 0.03;

const PLUCK_ATTACK_SECONDS: f64 = 0.003;
const PLUCK_DAMPING: f64 = 0.996;
const PLUCK_SMOOTHING_PASSES: u32 = 3;

const SOUND_COUNT: usize = 3;
const EVENT_COUNT: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Voice {
    Chime,
    Marimba,
    Pluck,
}

impl Voice {
    fn index(self) -> usize {
        match self {
            Self::Chime => 0,
            Self::Marimba => 1,
            Self::Pluck => 2,
        }
    }

    fn of(sound: StandbySound) -> Option<Self> {
        match sound {
            StandbySound::System => None,
            StandbySound::Chime => Some(Self::Chime),
            StandbySound::Marimba => Some(Self::Marimba),
            StandbySound::Pluck => Some(Self::Pluck),
        }
    }
}

/// The two notes of a sound and the time between their starts.
struct Melody {
    first_hertz: f64,
    second_hertz: f64,
    gap_seconds: f64,
}

fn melody(voice: Voice, event: SoundEvent) -> Melody {
    let (first_hertz, second_hertz, gap_seconds) = match (voice, event) {
        (Voice::Chime, SoundEvent::Woke) => (E5, B5, 0.13),
        (Voice::Chime, SoundEvent::WakeFailed) => (B4, E4, 0.18),
        (Voice::Marimba, SoundEvent::Woke) => (C5, G5, 0.11),
        (Voice::Marimba, SoundEvent::WakeFailed) => (G4, D4, 0.16),
        (Voice::Pluck, SoundEvent::Woke) => (A4, E5, 0.12),
        (Voice::Pluck, SoundEvent::WakeFailed) => (E4, B3, 0.17),
    };
    Melody {
        first_hertz,
        second_hertz,
        gap_seconds,
    }
}

/// Where the noise of a plucked string starts, for each note of each event.
fn pluck_seeds(event: SoundEvent) -> [u64; 2] {
    match event {
        SoundEvent::Woke => [1, 2],
        SoundEvent::WakeFailed => [3, 4],
    }
}

type WavKey = (usize, usize, u8);

/// The WAV file of `sound` for `event` at `volume` percent, built on first use
/// and kept for the rest of the process; `None` for the sound of the operating
/// system, which is not synthesized.
///
/// The volume is brought into its allowed range. A WAV is kept per sound,
/// event and volume because a player may read the buffer after this returns,
/// so it can never be freed. The cache is bounded: 3 sounds, 2 events and
/// 96 volumes make 576 files of about 100 KiB at most.
pub(super) fn synthesized_wav(
    sound: StandbySound,
    event: SoundEvent,
    volume: u8,
) -> Option<&'static [u8]> {
    static WAVS: LazyLock<Mutex<HashMap<WavKey, &'static [u8]>>> = LazyLock::new(Mutex::default);

    let voice = Voice::of(sound)?;
    let volume = volume.clamp(MIN_STANDBY_VOLUME, MAX_STANDBY_VOLUME);
    let key = (voice.index(), event_index(event), volume);
    let mut wavs = WAVS.lock().unwrap_or_else(PoisonError::into_inner);
    let wav = *wavs.entry(key).or_insert_with(|| {
        let bytes = encode_wav(rendered(voice, event), gain_of(volume));
        Box::leak(bytes.into_boxed_slice())
    });
    Some(wav)
}

fn event_index(event: SoundEvent) -> usize {
    match event {
        SoundEvent::Woke => 0,
        SoundEvent::WakeFailed => 1,
    }
}

/// The samples of `voice` for `event`, at full volume, built on first use.
fn rendered(voice: Voice, event: SoundEvent) -> &'static [f64] {
    static SAMPLES: [OnceLock<Vec<f64>>; SOUND_COUNT * EVENT_COUNT] =
        [const { OnceLock::new() }; SOUND_COUNT * EVENT_COUNT];

    SAMPLES[voice.index() * EVENT_COUNT + event_index(event)].get_or_init(|| render(voice, event))
}

/// The factor applied to the samples for `volume` percent. It is the square
/// of the ratio, because loudness is perceived on a curve: half the
/// percentage then sounds about half as loud.
fn gain_of(volume: u8) -> f64 {
    let ratio = f64::from(volume) / PERCENT;
    ratio * ratio
}

fn render(voice: Voice, event: SoundEvent) -> Vec<f64> {
    let melody = melody(voice, event);
    let [first_seed, second_seed] = pluck_seeds(event);
    let first = note(voice, melody.first_hertz, first_seed);
    let second = note(voice, melody.second_hertz, second_seed);
    let mixed = mix(&[first, second], melody.gap_seconds);
    pad_with_silence(normalize(fade_out(trim_decayed(mixed))))
}

fn note(voice: Voice, hertz: f64, seed: u64) -> Vec<f64> {
    match voice {
        Voice::Chime => chime_note(hertz, MAX_NOTE_SECONDS),
        Voice::Marimba => marimba_note(hertz, MAX_NOTE_SECONDS),
        Voice::Pluck => pluck_note(hertz, MAX_NOTE_SECONDS, seed),
    }
}

fn seconds_at(sample_index: u32) -> f64 {
    f64::from(sample_index) / f64::from(SAMPLE_RATE)
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn sample_count(seconds: f64) -> u32 {
    (seconds * f64::from(SAMPLE_RATE)) as u32
}

fn envelope(seconds: f64, attack: f64, decay: f64) -> f64 {
    let rise = (seconds / attack).min(1.0);
    rise * (-seconds / decay).exp()
}

fn chime_note(hertz: f64, length: f64) -> Vec<f64> {
    (0..sample_count(length))
        .map(|index| {
            let time = seconds_at(index);
            CHIME_PARTIALS
                .iter()
                .map(|&(ratio, amplitude, decay)| {
                    amplitude
                        * envelope(time, CHIME_ATTACK_SECONDS, decay)
                        * (std::f64::consts::TAU * hertz * ratio * time).sin()
                })
                .sum()
        })
        .collect()
}

fn marimba_note(hertz: f64, length: f64) -> Vec<f64> {
    (0..sample_count(length))
        .map(|index| {
            let time = seconds_at(index);
            let body = envelope(
                time,
                MARIMBA_BODY_ATTACK_SECONDS,
                MARIMBA_BODY_DECAY_SECONDS,
            ) * (std::f64::consts::TAU * hertz * time).sin();
            let knock = MARIMBA_KNOCK_AMPLITUDE
                * envelope(
                    time,
                    MARIMBA_KNOCK_ATTACK_SECONDS,
                    MARIMBA_KNOCK_DECAY_SECONDS,
                )
                * (std::f64::consts::TAU * hertz * MARIMBA_KNOCK_RATIO * time).sin();
            body + knock
        })
        .collect()
}

/// A small deterministic generator of noise, so that a sound is the same on
/// every run and machine.
struct NoiseGenerator {
    state: u64,
}

impl NoiseGenerator {
    const SEED_SPREAD: u64 = 0x9E37_79B9_7F4A_7C15;
    const UNIT_BITS: u32 = 53;

    fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_mul(Self::SEED_SPREAD) | 1,
        }
    }

    /// The next value, between -1 and 1.
    fn next_sample(&mut self) -> f64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        let unit_bits = self.state >> (u64::BITS - Self::UNIT_BITS);
        #[allow(clippy::cast_precision_loss)]
        let unit = unit_bits as f64 / (1_u64 << Self::UNIT_BITS) as f64;
        unit * 2.0 - 1.0
    }
}

/// A plucked string (Karplus-Strong): a burst of noise, smoothed, circling in
/// a delay line as long as one period and losing energy at each turn.
fn pluck_note(hertz: f64, length: f64, seed: u64) -> Vec<f64> {
    let period = sample_count(1.0 / hertz) as usize;
    let mut noise = NoiseGenerator::new(seed);
    let mut line: Vec<f64> = (0..period).map(|_| noise.next_sample()).collect();
    for _ in 0..PLUCK_SMOOTHING_PASSES {
        line = smoothed(&line);
    }
    let attack_samples = f64::from(sample_count(PLUCK_ATTACK_SECONDS));
    (0..sample_count(length))
        .map(|index| {
            let position = index as usize % period;
            let previous = line[(position + period - 1) % period];
            let value = line[position];
            line[position] = PLUCK_DAMPING * (value + previous) / 2.0;
            value * (f64::from(index) / attack_samples).min(1.0)
        })
        .collect()
}

fn smoothed(line: &[f64]) -> Vec<f64> {
    let length = line.len();
    (0..length)
        .map(|position| f64::midpoint(line[position], line[(position + length - 1) % length]))
        .collect()
}

/// Lays the notes one after the other, `gap_seconds` apart, so that they
/// ring over each other.
fn mix(notes: &[Vec<f64>], gap_seconds: f64) -> Vec<f64> {
    let offsets: Vec<usize> = (0..notes.len())
        .map(|index| sample_count(gap_seconds * f64::from(index_to_u32(index))) as usize)
        .collect();
    let total = notes
        .iter()
        .zip(&offsets)
        .map(|(samples, offset)| offset + samples.len())
        .max()
        .unwrap_or_default()
        + 1;
    let mut mixed = vec![0.0; total];
    for (samples, offset) in notes.iter().zip(offsets) {
        for (index, value) in samples.iter().enumerate() {
            mixed[offset + index] += value;
        }
    }
    mixed
}

fn index_to_u32(index: usize) -> u32 {
    u32::try_from(index).unwrap_or(u32::MAX)
}

/// Cuts the sound where it has decayed: shortly after its last sample louder
/// than [`DECAYED_AMPLITUDE`] of its peak, leaving room for the end fade.
fn trim_decayed(mut samples: Vec<f64>) -> Vec<f64> {
    let threshold = peak_of(&samples) * DECAYED_AMPLITUDE;
    let audible_length = samples
        .iter()
        .rposition(|value| value.abs() > threshold)
        .map_or(0, |last_audible| last_audible + 1);
    samples.truncate(audible_length + sample_count(END_FADE_SECONDS) as usize);
    samples
}

fn pad_with_silence(mut samples: Vec<f64>) -> Vec<f64> {
    let padded_length = samples.len() + sample_count(PLAYBACK_PADDING_SECONDS) as usize;
    samples.resize(padded_length, 0.0);
    samples
}

/// Brings the end of the sound down to silence, so that it does not click.
fn fade_out(mut samples: Vec<f64>) -> Vec<f64> {
    let fade_length = sample_count(END_FADE_SECONDS) as usize;
    let length = samples.len();
    for step in 0..fade_length.min(length) {
        samples[length - 1 - step] *=
            f64::from(index_to_u32(step)) / f64::from(index_to_u32(fade_length));
    }
    samples
}

fn peak_of(samples: &[f64]) -> f64 {
    samples
        .iter()
        .fold(0.0_f64, |peak, value| peak.max(value.abs()))
}

fn normalize(samples: Vec<f64>) -> Vec<f64> {
    let peak = peak_of(&samples);
    if peak == 0.0 {
        return samples;
    }
    samples
        .into_iter()
        .map(|value| value / peak * PEAK_AMPLITUDE)
        .collect()
}

#[allow(clippy::cast_possible_truncation)]
fn to_pcm(value: f64) -> i16 {
    (value * FULL_SCALE) as i16
}

fn encode_wav(samples: &[f64], gain: f64) -> Vec<u8> {
    let bytes_per_sample = BITS_PER_SAMPLE / 8;
    let data_bytes =
        u32::try_from(samples.len() * usize::from(bytes_per_sample)).unwrap_or(u32::MAX);
    let byte_rate = SAMPLE_RATE * u32::from(CHANNEL_COUNT * bytes_per_sample);
    let mut wav = Vec::with_capacity((WAV_HEADER_BYTES + data_bytes) as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(WAV_HEADER_BYTES - 8 + data_bytes).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    wav.extend_from_slice(&PCM_FORMAT.to_le_bytes());
    wav.extend_from_slice(&CHANNEL_COUNT.to_le_bytes());
    wav.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&(CHANNEL_COUNT * bytes_per_sample).to_le_bytes());
    wav.extend_from_slice(&BITS_PER_SAMPLE.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_bytes.to_le_bytes());
    for sample in samples {
        wav.extend_from_slice(&to_pcm(sample * gain).to_le_bytes());
    }
    wav
}

#[cfg(test)]
mod tests {
    use super::*;

    const SYNTHESIZED: [StandbySound; 3] = [
        StandbySound::Chime,
        StandbySound::Marimba,
        StandbySound::Pluck,
    ];

    const FULL_VOLUME: u8 = 100;

    fn wav(sound: StandbySound, event: SoundEvent) -> &'static [u8] {
        synthesized_wav(sound, event, FULL_VOLUME).unwrap()
    }

    fn peak_of_pcm(samples: &[i16]) -> i32 {
        samples
            .iter()
            .map(|sample| i32::from(*sample).abs())
            .max()
            .unwrap()
    }

    fn samples(wav: &[u8]) -> Vec<i16> {
        wav[WAV_HEADER_BYTES as usize..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| i16::from_le_bytes(*pair))
            .collect()
    }

    fn read_u32(wav: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes(wav[offset..offset + 4].try_into().unwrap())
    }

    #[test]
    fn should_write_a_riff_wave_header_when_synthesizing() {
        for sound in SYNTHESIZED {
            let wav = wav(sound, SoundEvent::Woke);

            assert_eq!(
                (&wav[0..4], &wav[8..12], &wav[12..16], &wav[36..40]),
                (&b"RIFF"[..], &b"WAVE"[..], &b"fmt "[..], &b"data"[..]),
                "{sound:?}"
            );
        }
    }

    #[test]
    fn should_describe_mono_16_bit_pcm_at_44_1_khz_when_synthesizing() {
        let wav = wav(StandbySound::Pluck, SoundEvent::Woke);

        assert_eq!(
            (
                u16::from_le_bytes([wav[20], wav[21]]),
                u16::from_le_bytes([wav[22], wav[23]]),
                read_u32(wav, 24),
                u16::from_le_bytes([wav[34], wav[35]]),
            ),
            (1, 1, 44_100, 16)
        );
    }

    #[test]
    fn should_size_the_chunks_to_the_audio_when_synthesizing() {
        let wav = wav(StandbySound::Chime, SoundEvent::Woke);
        let data_bytes = read_u32(wav, 40) as usize;

        assert_eq!(
            (data_bytes, read_u32(wav, 4) as usize),
            (wav.len() - 44, wav.len() - 8)
        );
    }

    fn padding_samples() -> usize {
        sample_count(PLAYBACK_PADDING_SECONDS) as usize
    }

    fn all_sounds() -> impl Iterator<Item = (StandbySound, SoundEvent)> {
        SYNTHESIZED.into_iter().flat_map(|sound| {
            [SoundEvent::Woke, SoundEvent::WakeFailed]
                .into_iter()
                .map(move |event| (sound, event))
        })
    }

    #[test]
    fn should_have_decayed_under_minus_50_db_when_the_end_fade_starts() {
        for (sound, event) in all_sounds() {
            let samples = samples(wav(sound, event));
            let peak = f64::from(peak_of_pcm(&samples));
            let sound_length = samples.len() - padding_samples();
            let fade_start = sound_length - sample_count(END_FADE_SECONDS) as usize;

            let loudest_in_fade = samples[fade_start..sound_length]
                .iter()
                .map(|sample| f64::from(*sample).abs())
                .fold(0.0, f64::max);

            assert!(
                loudest_in_fade <= peak * DECAYED_AMPLITUDE,
                "{sound:?} {event:?} is still at {loudest_in_fade} of {peak} when its fade starts"
            );
        }
    }

    #[test]
    fn should_decay_before_the_cap_instead_of_being_cut_when_synthesizing() {
        for (sound, event) in all_sounds() {
            let sound_length = samples(wav(sound, event)).len() - padding_samples();

            let cap = sample_count(MAX_NOTE_SECONDS) as usize;

            assert!(
                sound_length < cap,
                "{sound:?} {event:?} lasts {sound_length} samples, the cap is {cap}"
            );
        }
    }

    #[test]
    fn should_end_with_the_padding_silence_when_synthesizing() {
        for (sound, event) in all_sounds() {
            let samples = samples(wav(sound, event));

            let tail = &samples[samples.len() - padding_samples()..];

            assert!(
                tail.iter().all(|sample| *sample == 0),
                "{sound:?} {event:?}"
            );
        }
    }

    #[test]
    fn should_stay_under_the_cap_plus_the_gap_and_the_padding_when_synthesizing() {
        for (sound, event) in all_sounds() {
            let voice = Voice::of(sound).unwrap();
            let longest = sample_count(MAX_NOTE_SECONDS + melody(voice, event).gap_seconds)
                as usize
                + padding_samples();

            assert!(
                samples(wav(sound, event)).len() <= longest,
                "{sound:?} {event:?}"
            );
        }
    }

    #[test]
    fn should_play_at_the_square_of_the_ratio_when_the_volume_is_lowered() {
        let full = peak_of_pcm(&samples(wav(StandbySound::Chime, SoundEvent::Woke)));

        let half = peak_of_pcm(&samples(
            synthesized_wav(StandbySound::Chime, SoundEvent::Woke, 50).unwrap(),
        ));

        assert!((half * 4 - full).abs() <= 4, "{half} {full}");
    }

    #[test]
    fn should_bring_the_volume_into_its_range_when_it_is_out_of_it() {
        let quietest = synthesized_wav(StandbySound::Pluck, SoundEvent::Woke, MIN_STANDBY_VOLUME);
        let loudest = synthesized_wav(StandbySound::Pluck, SoundEvent::Woke, MAX_STANDBY_VOLUME);

        assert_eq!(
            (
                synthesized_wav(StandbySound::Pluck, SoundEvent::Woke, 0),
                synthesized_wav(StandbySound::Pluck, SoundEvent::Woke, u8::MAX),
            ),
            (quietest, loudest)
        );
    }

    #[test]
    fn should_share_one_buffer_per_volume_when_asked_twice() {
        let first = synthesized_wav(StandbySound::Chime, SoundEvent::Woke, 35).unwrap();
        let second = synthesized_wav(StandbySound::Chime, SoundEvent::Woke, 35).unwrap();
        let other_volume = synthesized_wav(StandbySound::Chime, SoundEvent::Woke, 40).unwrap();

        assert!(std::ptr::eq(first, second) && !std::ptr::eq(first, other_volume));
    }

    #[test]
    fn should_synthesize_the_same_bytes_when_asked_twice() {
        for sound in SYNTHESIZED {
            let first = render(Voice::of(sound).unwrap(), SoundEvent::Woke);
            let second = render(Voice::of(sound).unwrap(), SoundEvent::Woke);

            assert_eq!(first, second, "{sound:?}");
        }
    }

    #[test]
    fn should_share_one_buffer_when_asked_twice_for_the_same_sound() {
        let first = wav(StandbySound::Marimba, SoundEvent::WakeFailed);
        let second = wav(StandbySound::Marimba, SoundEvent::WakeFailed);

        assert!(std::ptr::eq(first, second));
    }

    #[test]
    fn should_sound_different_when_the_tunnel_failed_to_wake() {
        for sound in SYNTHESIZED {
            assert_ne!(
                wav(sound, SoundEvent::Woke),
                wav(sound, SoundEvent::WakeFailed),
                "{sound:?}"
            );
        }
    }

    #[test]
    fn should_peak_near_half_scale_when_synthesizing() {
        for sound in SYNTHESIZED {
            for event in [SoundEvent::Woke, SoundEvent::WakeFailed] {
                let peak = peak_of_pcm(&samples(wav(sound, event)));

                assert!(
                    (16_200..=16_400).contains(&peak),
                    "{sound:?} {event:?} peaks at {peak}"
                );
            }
        }
    }

    #[test]
    fn should_end_on_silence_when_synthesizing() {
        for sound in SYNTHESIZED {
            for event in [SoundEvent::Woke, SoundEvent::WakeFailed] {
                assert_eq!(
                    samples(wav(sound, event)).last(),
                    Some(&0),
                    "{sound:?} {event:?}"
                );
            }
        }
    }

    #[test]
    fn should_sound_different_from_one_family_to_another() {
        let [chime, marimba, pluck] = SYNTHESIZED.map(|sound| wav(sound, SoundEvent::Woke));

        assert!(chime != marimba && marimba != pluck && chime != pluck);
    }

    #[test]
    fn should_not_synthesize_the_sound_of_the_system() {
        assert_eq!(
            synthesized_wav(StandbySound::System, SoundEvent::Woke, FULL_VOLUME),
            None
        );
    }
}
