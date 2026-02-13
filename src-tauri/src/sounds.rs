use std::sync::Arc;

/// Holds pre-generated WAV buffers for start and stop sounds.
/// Clone-cheap via Arc. Immutable after construction.
#[derive(Clone)]
pub struct SoundEffects {
    start_wav: Arc<Vec<u8>>,
    stop_wav: Arc<Vec<u8>>,
}

impl SoundEffects {
    pub fn new() -> Self {
        Self {
            // 880 Hz, 120ms, 10ms fade-in, 30ms fade-out, 0.3 amplitude
            start_wav: Arc::new(generate_wav(880.0, 120, 10, 30, 0.3)),
            // 440 Hz, 150ms, 10ms fade-in, 50ms fade-out, 0.3 amplitude
            stop_wav: Arc::new(generate_wav(440.0, 150, 10, 50, 0.3)),
        }
    }

    pub fn play_start_sound(&self) {
        play_wav(&self.start_wav);
    }

    pub fn play_stop_sound(&self) {
        play_wav(&self.stop_wav);
    }
}

/// Generate a 16-bit PCM mono WAV file as a byte vector.
///
/// - freq_hz: sine wave frequency (e.g. 880.0 for A5)
/// - duration_ms: total sound duration in milliseconds
/// - fade_in_ms: linear fade-in duration (prevents click at start)
/// - fade_out_ms: linear fade-out duration (prevents click at end)
/// - amplitude: peak amplitude 0.0-1.0 (0.3 = 30% volume)
fn generate_wav(
    freq_hz: f64,
    duration_ms: u32,
    fade_in_ms: u32,
    fade_out_ms: u32,
    amplitude: f64,
) -> Vec<u8> {
    const SAMPLE_RATE: u32 = 44100;
    const BITS_PER_SAMPLE: u16 = 16;
    const NUM_CHANNELS: u16 = 1;

    let num_samples = (SAMPLE_RATE as u64 * duration_ms as u64 / 1000) as usize;
    let data_size = (num_samples * (BITS_PER_SAMPLE / 8) as usize) as u32;
    let file_size = 36 + data_size; // RIFF header (12) + fmt chunk (24) + data header (8) - 8 for RIFF prefix

    let byte_rate = SAMPLE_RATE * NUM_CHANNELS as u32 * (BITS_PER_SAMPLE / 8) as u32;
    let block_align = NUM_CHANNELS * (BITS_PER_SAMPLE / 8);

    let mut wav = Vec::with_capacity(44 + data_size as usize);

    // --- RIFF header (12 bytes) ---
    // ALL multi-byte integers in WAV format are LITTLE-ENDIAN.
    // Use .to_le_bytes() for every u16 and u32 value.
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&file_size.to_le_bytes());       // u32 LE
    wav.extend_from_slice(b"WAVE");

    // --- fmt sub-chunk (24 bytes) ---
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());           // sub-chunk size (16 for PCM)
    wav.extend_from_slice(&1u16.to_le_bytes());            // audio format (1 = PCM)
    wav.extend_from_slice(&NUM_CHANNELS.to_le_bytes());    // u16 LE
    wav.extend_from_slice(&SAMPLE_RATE.to_le_bytes());     // u32 LE
    wav.extend_from_slice(&byte_rate.to_le_bytes());       // u32 LE
    wav.extend_from_slice(&block_align.to_le_bytes());     // u16 LE
    wav.extend_from_slice(&BITS_PER_SAMPLE.to_le_bytes()); // u16 LE

    // --- data sub-chunk ---
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_size.to_le_bytes());       // u32 LE

    // --- PCM samples (i16 LE) ---
    let duration_secs = duration_ms as f64 / 1000.0;
    let fade_in_secs = fade_in_ms as f64 / 1000.0;
    let fade_out_secs = fade_out_ms as f64 / 1000.0;

    for i in 0..num_samples {
        let t = i as f64 / SAMPLE_RATE as f64;

        // Envelope: linear fade-in, sustain, linear fade-out
        let env = envelope(t, duration_secs, fade_in_secs, fade_out_secs);

        // Sine wave scaled by amplitude and envelope, then to i16 range
        let sample_f64 = amplitude * env * (2.0 * std::f64::consts::PI * freq_hz * t).sin();
        let sample_i16 = (sample_f64 * 32767.0).clamp(-32768.0, 32767.0) as i16;

        wav.extend_from_slice(&sample_i16.to_le_bytes()); // i16 LE
    }

    wav
}

/// Linear envelope with fade-in and fade-out.
/// Returns 0.0-1.0 multiplier for the given time position.
///
/// - t: current time in seconds
/// - duration: total duration in seconds
/// - fade_in: fade-in time in seconds (linear ramp 0->1)
/// - fade_out: fade-out time in seconds (linear ramp 1->0)
fn envelope(t: f64, duration: f64, fade_in: f64, fade_out: f64) -> f64 {
    if t < fade_in {
        // Linear ramp up from 0 to 1
        t / fade_in
    } else if t > duration - fade_out {
        // Linear ramp down from 1 to 0
        (duration - t) / fade_out
    } else {
        // Sustain at full volume
        1.0
    }
}

/// Play a WAV byte buffer through the default audio output device.
/// Non-blocking (SND_ASYNC). Non-fatal on failure (SND_NODEFAULT + log).
///
/// Uses PlaySoundA (not PlaySoundW) because SND_MEMORY expects a raw byte
/// pointer. PCSTR maps naturally to &[u8]; PCWSTR would require a misaligned
/// cast from *const u8 to *const u16.
#[cfg(target_os = "windows")]
fn play_wav(wav_data: &[u8]) {
    use windows::Win32::Media::Audio::{
        PlaySoundA, SND_ASYNC, SND_MEMORY, SND_NODEFAULT,
    };
    use windows::core::PCSTR;

    unsafe {
        let result = PlaySoundA(
            PCSTR(wav_data.as_ptr()),
            None,
            SND_MEMORY | SND_ASYNC | SND_NODEFAULT,
        );
        if !result.as_bool() {
            eprintln!("PlaySound failed (no audio output device or sound disabled)");
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn play_wav(_wav_data: &[u8]) {
    // No-op on non-Windows platforms.
    // Future: use platform-specific APIs (NSSound on macOS, PulseAudio on Linux).
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_buffers_have_correct_structure() {
        let sounds = SoundEffects::new();

        // Start WAV: 44 header + (44100 * 120 / 1000) * 2 bytes = 44 + 10584 = 10628
        assert!(sounds.start_wav.len() > 44, "Start WAV too small: {}", sounds.start_wav.len());
        assert!(sounds.stop_wav.len() > 44, "Stop WAV too small: {}", sounds.stop_wav.len());

        // Verify RIFF header magic bytes
        assert_eq!(&sounds.start_wav[0..4], b"RIFF", "Missing RIFF header");
        assert_eq!(&sounds.start_wav[8..12], b"WAVE", "Missing WAVE marker");
        assert_eq!(&sounds.stop_wav[0..4], b"RIFF", "Missing RIFF header (stop)");
        assert_eq!(&sounds.stop_wav[8..12], b"WAVE", "Missing WAVE marker (stop)");
    }

    #[test]
    fn wav_header_fields_are_correct() {
        let wav = generate_wav(440.0, 100, 10, 20, 0.3);

        // fmt chunk marker
        assert_eq!(&wav[12..16], b"fmt ", "Missing fmt chunk");

        // Audio format: PCM = 1 (u16 LE at offset 20)
        assert_eq!(u16::from_le_bytes([wav[20], wav[21]]), 1, "Not PCM format");

        // Channels: 1 mono (u16 LE at offset 22)
        assert_eq!(u16::from_le_bytes([wav[22], wav[23]]), 1, "Not mono");

        // Sample rate: 44100 (u32 LE at offset 24)
        assert_eq!(
            u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]),
            44100,
            "Wrong sample rate"
        );

        // Bits per sample: 16 (u16 LE at offset 34)
        assert_eq!(u16::from_le_bytes([wav[34], wav[35]]), 16, "Not 16-bit");

        // data chunk marker
        assert_eq!(&wav[36..40], b"data", "Missing data chunk");
    }

    #[test]
    fn wav_data_has_correct_length() {
        let wav = generate_wav(440.0, 100, 10, 20, 0.3);

        // Expected samples for 100ms at 44100 Hz
        let expected_samples = (44100u64 * 100 / 1000) as usize;
        let expected_data_bytes = expected_samples * 2; // 16-bit = 2 bytes/sample
        let expected_total = 44 + expected_data_bytes;

        assert_eq!(wav.len(), expected_total,
            "WAV total size mismatch: expected {}, got {}", expected_total, wav.len());

        // Verify data chunk size field matches actual data
        let data_size = u32::from_le_bytes([wav[40], wav[41], wav[42], wav[43]]) as usize;
        assert_eq!(data_size, expected_data_bytes, "data chunk size field mismatch");
    }

    #[test]
    fn wav_samples_are_not_all_zero() {
        // This catches the BLOCKER-6 bug: if amplitude is not scaled by 32767,
        // all samples would truncate to zero.
        let wav = generate_wav(440.0, 100, 10, 20, 0.3);

        // Check PCM data region (after 44-byte header)
        let pcm_data = &wav[44..];
        let max_abs_sample = pcm_data
            .chunks(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]).unsigned_abs())
            .max()
            .unwrap_or(0);

        // At 0.3 amplitude, peak should be ~0.3 * 32767 = ~9830
        assert!(max_abs_sample > 5000,
            "Samples appear too quiet (max abs = {}). Check i16 scaling.", max_abs_sample);
        assert!(max_abs_sample < 15000,
            "Samples too loud (max abs = {}). Amplitude should be ~30%.", max_abs_sample);
    }

    #[test]
    fn envelope_produces_correct_shape() {
        // Fade-in: ramp from 0 to 1 in first 10ms
        assert!((envelope(0.0, 0.1, 0.01, 0.02) - 0.0).abs() < 0.001, "Envelope should start at 0");
        assert!((envelope(0.005, 0.1, 0.01, 0.02) - 0.5).abs() < 0.001, "Mid fade-in should be 0.5");
        assert!((envelope(0.01, 0.1, 0.01, 0.02) - 1.0).abs() < 0.001, "End of fade-in should be 1.0");

        // Sustain: full volume in the middle
        assert!((envelope(0.05, 0.1, 0.01, 0.02) - 1.0).abs() < 0.001, "Sustain should be 1.0");

        // Fade-out: ramp from 1 to 0 in last 20ms
        assert!((envelope(0.08, 0.1, 0.01, 0.02) - 1.0).abs() < 0.001, "Start of fade-out");
        assert!((envelope(0.09, 0.1, 0.01, 0.02) - 0.5).abs() < 0.001, "Mid fade-out should be ~0.5");
    }

    #[test]
    fn start_and_stop_sounds_differ() {
        let sounds = SoundEffects::new();
        // Different frequency + duration means different buffer lengths and content
        assert_ne!(sounds.start_wav.len(), sounds.stop_wav.len(),
            "Start and stop WAVs should have different lengths (different durations)");
        // Also verify they are not identical bytes
        assert_ne!(*sounds.start_wav, *sounds.stop_wav,
            "Start and stop WAVs must not be identical");
    }
}
