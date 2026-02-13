use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const OUTPUT_SAMPLE_RATE: u32 = 16000;
/// Minimum mic volume (scalar 0.0-1.0) required for capture.
/// If below this, the mic will be boosted to MIC_TARGET_VOLUME.
/// Set high (0.8) because even at 60% volume, Realtek mics produce
/// very quiet output (max amplitude < 0.001).
const MIC_MIN_VOLUME: f32 = 0.8;
/// Target volume when auto-boosting a quiet/muted mic.
const MIC_TARGET_VOLUME: f32 = 1.0;

pub struct AudioRecorder {
    capture_sample_rate: Arc<Mutex<u32>>,
    capture_channels: Arc<Mutex<u16>>,
    samples: Arc<Mutex<Vec<f32>>>,
    /// Handle to the dedicated audio thread -- dropping it signals shutdown
    audio_thread: Option<std::thread::JoinHandle<()>>,
    stop_signal: Arc<Mutex<bool>>,
    start_time: Option<Instant>,
    /// Lock-free RMS audio level for VU meter (0.0-1.0 stored as f32 bits).
    /// Written by audio callback, read by UI polling timer via Arc on AppState.
    audio_level: Arc<AtomicU32>,
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            capture_sample_rate: Arc::new(Mutex::new(0)),
            capture_channels: Arc::new(Mutex::new(0)),
            samples: Arc::new(Mutex::new(Vec::new())),
            audio_thread: None,
            stop_signal: Arc::new(Mutex::new(false)),
            start_time: None,
            audio_level: Arc::new(AtomicU32::new(0f32.to_bits())),
        }
    }

    pub fn start_recording(&mut self) -> Result<(), String> {
        // Clear previous state
        self.samples.lock().unwrap().clear();
        *self.stop_signal.lock().unwrap() = false;
        self.start_time = Some(Instant::now());
        self.audio_level.store(0f32.to_bits(), Ordering::Relaxed);

        let samples = Arc::clone(&self.samples);
        let stop_signal = Arc::clone(&self.stop_signal);
        let capture_rate = Arc::clone(&self.capture_sample_rate);
        let capture_ch = Arc::clone(&self.capture_channels);
        let audio_level = Arc::clone(&self.audio_level);

        // Use a channel to get the result back from the audio thread
        let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();

        // Spawn a DEDICATED THREAD for all cpal/WASAPI operations.
        // This avoids COM threading conflicts with Tauri's winit
        // (which uses COINIT_APARTMENTTHREADED). A fresh thread gets
        // its own COM initialization via cpal's WASAPI backend.
        let handle = std::thread::spawn(move || {
            // Ensure microphone is not muted and volume is adequate.
            // This MUST run on the audio thread (fresh COM apartment).
            #[cfg(target_os = "windows")]
            ensure_mic_volume();

            let host = cpal::default_host();
            println!("Audio host: {:?}", host.id());

            let device = match host.default_input_device() {
                Some(d) => d,
                None => {
                    let _ = tx.send(Err("No input device found".to_string()));
                    return;
                }
            };

            println!("Input device: {:?}", device.name().unwrap_or_default());

            let default_config = match device.default_input_config() {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(Err(format!("Failed to get input config: {}", e)));
                    return;
                }
            };

            println!("Device default config: {:?}", default_config);

            let native_sample_rate = default_config.sample_rate().0;
            let native_channels = default_config.channels();

            *capture_rate.lock().unwrap() = native_sample_rate;
            *capture_ch.lock().unwrap() = native_channels;

            println!(
                "Recording at native: {}Hz, {}ch (will convert to {}Hz mono for output)",
                native_sample_rate, native_channels, OUTPUT_SAMPLE_RATE
            );

            let audio_level_for_callback = Arc::clone(&audio_level);

            let stream = match device.build_input_stream(
                &cpal::StreamConfig {
                    channels: native_channels,
                    sample_rate: cpal::SampleRate(native_sample_rate),
                    buffer_size: cpal::BufferSize::Default,
                },
                {
                    let audio_level = audio_level_for_callback;
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        // Compute RMS for VU meter (lock-free, no allocation)
                        // NOTE: RMS is computed on interleaved multi-channel data. For stereo
                        // mics, this under-reports by ~sqrt(channels). Acceptable for qualitative
                        // VU display -- the JS gain factor compensates. If per-channel accuracy
                        // matters later, downmix to mono here first.
                        if !data.is_empty() {
                            let rms = compute_rms(data);
                            let level = rms.min(1.0);
                            audio_level.store(level.to_bits(), Ordering::Relaxed);
                        }
                        // Store samples for transcription (existing behavior)
                        samples.lock().unwrap().extend_from_slice(data);
                    }
                },
                |err| eprintln!("Audio stream error: {}", err),
                None,
            ) {
                Ok(s) => s,
                Err(e) => {
                    let _ = tx.send(Err(format!("Failed to build input stream: {}", e)));
                    return;
                }
            };

            if let Err(e) = stream.play() {
                let _ = tx.send(Err(format!("Failed to start audio stream: {}", e)));
                return;
            }

            println!("Recording started successfully (dedicated audio thread)");
            let _ = tx.send(Ok(()));

            // Keep this thread (and the stream) alive until stop is signaled
            loop {
                std::thread::sleep(Duration::from_millis(50));
                if *stop_signal.lock().unwrap() {
                    break;
                }
            }

            // Stream is dropped here when the thread exits, stopping capture
            drop(stream);
            println!("Audio thread exiting, stream dropped");
        });

        // Wait for the audio thread to report success or failure
        let result = rx.recv().map_err(|e| format!("Audio thread died: {}", e))?;
        result?;

        self.audio_thread = Some(handle);
        Ok(())
    }

    /// Stop recording and return processed audio as 16kHz mono f32 samples,
    /// ready for direct input to whisper-rs (no WAV file needed).
    pub fn stop_recording(&mut self) -> Result<Vec<f32>, String> {
        // Signal the audio thread to stop
        *self.stop_signal.lock().unwrap() = true;
        self.audio_level.store(0f32.to_bits(), Ordering::Relaxed);

        // Wait for the audio thread to finish
        if let Some(handle) = self.audio_thread.take() {
            let _ = handle.join();
        }

        // Get raw samples
        let raw_samples = self.samples.lock().unwrap();
        if raw_samples.is_empty() {
            return Err("No audio recorded".to_string());
        }

        let ch = *self.capture_channels.lock().unwrap();
        let rate = *self.capture_sample_rate.lock().unwrap();

        println!(
            "Captured {} raw samples ({}ch @ {}Hz)",
            raw_samples.len(),
            ch,
            rate
        );

        // Check for all-zero audio (mic permission or COM issue)
        let max_abs = raw_samples.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
        println!("Max amplitude in raw samples: {}", max_abs);
        if is_silence(&raw_samples, 1e-7) {
            eprintln!("WARNING: All samples are zero -- microphone may not be capturing audio");
        }

        // Convert to mono by averaging channels
        let mono_samples = to_mono(&raw_samples, ch);

        // Resample to 16kHz if needed
        let resampled = if rate != OUTPUT_SAMPLE_RATE {
            resample(&mono_samples, rate, OUTPUT_SAMPLE_RATE)
        } else {
            mono_samples
        };

        let duration_secs = resampled.len() as f64 / OUTPUT_SAMPLE_RATE as f64;
        println!(
            "Audio ready: {} samples @ {}Hz mono ({:.1}s)",
            resampled.len(),
            OUTPUT_SAMPLE_RATE,
            duration_secs
        );

        Ok(resampled)
    }

    /// Returns the elapsed duration since recording started, or None if not recording.
    #[allow(dead_code)]
    pub fn recording_duration(&self) -> Option<Duration> {
        self.start_time.map(|t| t.elapsed())
    }

    pub fn cancel_recording(&mut self) {
        *self.stop_signal.lock().unwrap() = true;
        self.audio_level.store(0f32.to_bits(), Ordering::Relaxed);
        if let Some(handle) = self.audio_thread.take() {
            let _ = handle.join();
        }
        self.samples.lock().unwrap().clear();
        self.start_time = None;
    }

    /// Returns a clone of the audio level Arc for sharing with AppState.
    /// Call this BEFORE moving the recorder into Arc<Mutex<...>>.
    pub fn audio_level_arc(&self) -> Arc<AtomicU32> {
        Arc::clone(&self.audio_level)
    }

    /// Returns the current RMS audio level (0.0 to 1.0, clamped).
    /// Lock-free read -- safe to call from any thread at any frequency.
    #[allow(dead_code)]
    pub fn current_level(&self) -> f32 {
        f32::from_bits(self.audio_level.load(Ordering::Relaxed))
    }
}

impl Drop for AudioRecorder {
    fn drop(&mut self) {
        self.audio_level.store(0f32.to_bits(), Ordering::Relaxed);
        // Use lock().ok() instead of unwrap() to avoid panic-in-drop
        // if the Mutex is poisoned from a prior panic on another thread.
        if let Ok(mut signal) = self.stop_signal.lock() {
            *signal = true;
        }
        if let Some(handle) = self.audio_thread.take() {
            let _ = handle.join();
        }
    }
}

/// Compute the root-mean-square of an audio sample buffer.
/// Returns 0.0 for empty input (avoids division by zero).
pub fn compute_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|&s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

/// Ensure the default capture endpoint (microphone) is not muted and has
/// adequate volume. On Windows, WASAPI captures silence if the endpoint
/// is muted even though the hardware mic is working.
///
/// This must be called from the audio thread where COM is initialized.
#[cfg(target_os = "windows")]
fn ensure_mic_volume() {
    use windows::Win32::Media::Audio::*;
    use windows::Win32::Media::Audio::Endpoints::*;
    use windows::Win32::System::Com::*;

    let result: Result<(), Box<dyn std::error::Error>> = (|| {
        unsafe {
            // COM should already be initialized by cpal, but ensure it.
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;

            let device = enumerator.GetDefaultAudioEndpoint(eCapture, eConsole)?;

            let volume: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;

            let scalar = volume.GetMasterVolumeLevelScalar()?;
            let muted = volume.GetMute()?.as_bool();

            println!(
                "Mic volume: {:.0}%, muted: {}",
                scalar * 100.0,
                muted
            );

            let needs_fix = muted || scalar < MIC_MIN_VOLUME;

            if needs_fix {
                if muted {
                    println!("Microphone is MUTED -- unmuting");
                    volume.SetMute(false, std::ptr::null())?;
                }
                if scalar < MIC_MIN_VOLUME {
                    println!(
                        "Microphone volume too low ({:.0}%) -- boosting to {:.0}%",
                        scalar * 100.0,
                        MIC_TARGET_VOLUME * 100.0
                    );
                    volume.SetMasterVolumeLevelScalar(MIC_TARGET_VOLUME, std::ptr::null())?;
                }

                // Verify
                let new_scalar = volume.GetMasterVolumeLevelScalar()?;
                let new_muted = volume.GetMute()?.as_bool();
                println!(
                    "Mic volume after fix: {:.0}%, muted: {}",
                    new_scalar * 100.0,
                    new_muted
                );
            }
        }
        Ok(())
    })();

    if let Err(e) = result {
        // Non-fatal: if we can't check volume, still try to record.
        // The user will get the existing "all samples are zero" warning.
        eprintln!("Warning: Could not check mic volume: {}", e);
    }
}

/// Convert multi-channel audio to mono by averaging all channels per frame.
/// Single-channel input is returned as-is.
fn to_mono(samples: &[f32], channels: u16) -> Vec<f32> {
    if channels <= 1 {
        samples.to_vec()
    } else {
        samples
            .chunks(channels as usize)
            .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
            .collect()
    }
}

/// Check if audio samples are effectively silent (all near-zero).
/// Returns true if the max absolute amplitude is below the threshold.
fn is_silence(samples: &[f32], threshold: f32) -> bool {
    let max_abs = samples.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
    max_abs < threshold
}

/// Simple linear interpolation resampler
fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    let ratio = from_rate as f64 / to_rate as f64;
    let output_len = (samples.len() as f64 / ratio) as usize;
    let mut output = Vec::with_capacity(output_len);

    for i in 0..output_len {
        let src_pos = i as f64 * ratio;
        let src_idx = src_pos as usize;
        let frac = src_pos - src_idx as f64;

        let sample = if src_idx + 1 < samples.len() {
            samples[src_idx] as f64 * (1.0 - frac) + samples[src_idx + 1] as f64 * frac
        } else if src_idx < samples.len() {
            samples[src_idx] as f64
        } else {
            0.0
        };

        output.push(sample as f32);
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    // ================================================================
    // RESAMPLING — User hears correct audio regardless of their hardware
    // ================================================================

    #[test]
    fn resample_48khz_to_16khz_preserves_duration() {
        // UX: User has a 48kHz mic (very common on modern PCs).
        // After resampling to 16kHz for whisper, the audio duration must be
        // the same — otherwise transcription is time-shifted or truncated.
        let input_rate = 48000;
        let output_rate = 16000;
        let duration_secs = 2.0;
        let input_len = (input_rate as f64 * duration_secs) as usize;
        let input: Vec<f32> = (0..input_len).map(|i| (i as f32 * 0.001).sin()).collect();

        let output = resample(&input, input_rate, output_rate);

        let expected_len = (output_rate as f64 * duration_secs) as usize;
        // Allow ±1 sample tolerance for rounding
        assert!(
            (output.len() as i64 - expected_len as i64).abs() <= 1,
            "48kHz→16kHz: expected ~{} samples, got {} (duration mismatch)",
            expected_len, output.len()
        );
    }

    #[test]
    fn resample_44100_to_16khz_preserves_duration() {
        // UX: User has a 44.1kHz mic (common on older/consumer hardware).
        let input_rate = 44100;
        let output_rate = 16000;
        let duration_secs = 1.5;
        let input_len = (input_rate as f64 * duration_secs) as usize;
        let input: Vec<f32> = (0..input_len).map(|i| (i as f32 * 0.001).sin()).collect();

        let output = resample(&input, input_rate, output_rate);

        let expected_len = (output_rate as f64 * duration_secs) as usize;
        assert!(
            (output.len() as i64 - expected_len as i64).abs() <= 1,
            "44.1kHz→16kHz: expected ~{} samples, got {}",
            expected_len, output.len()
        );
    }

    #[test]
    fn resample_16khz_to_16khz_is_identity() {
        // UX: User's mic natively runs at 16kHz (some USB mics).
        // Resampling should be a no-op — output equals input exactly.
        let input: Vec<f32> = vec![0.1, 0.5, -0.3, 0.7, -0.9];
        let output = resample(&input, 16000, 16000);

        assert_eq!(output.len(), input.len(), "Identity resample should preserve length");
        for (i, (&a, &b)) in input.iter().zip(output.iter()).enumerate() {
            assert!(
                (a - b).abs() < 1e-6,
                "Sample {} differs: input={}, output={}",
                i, a, b
            );
        }
    }

    #[test]
    fn resample_empty_input_returns_empty() {
        // Edge case: no audio captured (stop immediately after start).
        let output = resample(&[], 48000, 16000);
        assert!(output.is_empty(), "Empty input should produce empty output");
    }

    #[test]
    fn resample_preserves_sine_wave_shape() {
        // UX: Resampling shouldn't introduce major distortion — a sine wave
        // should still look like a sine wave after downsampling.
        let input_rate = 48000u32;
        let output_rate = 16000u32;
        let freq = 440.0; // A4 note
        let duration = 0.01; // 10ms — enough for a few cycles
        let input_len = (input_rate as f64 * duration) as usize;

        // Generate 440Hz sine at 48kHz
        let input: Vec<f32> = (0..input_len)
            .map(|i| (2.0 * std::f64::consts::PI * freq * i as f64 / input_rate as f64).sin() as f32)
            .collect();

        let output = resample(&input, input_rate, output_rate);

        // Verify output has energy (not all zeros)
        let max_abs = output.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
        assert!(max_abs > 0.5, "Resampled sine wave should have significant amplitude, got {}", max_abs);

        // Verify output length is correct
        let expected_len = (output_rate as f64 * duration) as usize;
        assert!(
            (output.len() as i64 - expected_len as i64).abs() <= 1,
            "Output length {} doesn't match expected {}",
            output.len(), expected_len
        );
    }

    // ================================================================
    // MONO DOWNMIX — User's stereo mic produces correct mono for whisper
    // ================================================================

    #[test]
    fn mono_downmix_stereo_averages_channels() {
        // UX: User has a stereo mic. Whisper needs mono. The downmix
        // should average L+R so both channels contribute equally.
        let stereo: Vec<f32> = vec![
            0.6, 0.4,   // frame 1: L=0.6, R=0.4 → mono=0.5
            -0.2, 0.2,  // frame 2: L=-0.2, R=0.2 → mono=0.0
            1.0, -1.0,  // frame 3: L=1.0, R=-1.0 → mono=0.0
        ];

        let mono = to_mono(&stereo, 2);

        assert_eq!(mono.len(), 3, "3 stereo frames → 3 mono samples");
        assert!((mono[0] - 0.5).abs() < 1e-6, "Frame 1: expected 0.5, got {}", mono[0]);
        assert!((mono[1] - 0.0).abs() < 1e-6, "Frame 2: expected 0.0, got {}", mono[1]);
        assert!((mono[2] - 0.0).abs() < 1e-6, "Frame 3: expected 0.0, got {}", mono[2]);
    }

    #[test]
    fn mono_downmix_single_channel_passthrough() {
        // UX: User's mic is already mono. No processing needed — samples
        // should pass through unchanged.
        let input: Vec<f32> = vec![0.1, -0.5, 0.8, -0.3];
        let output = to_mono(&input, 1);

        assert_eq!(output, input, "Mono input should pass through unchanged");
    }

    #[test]
    fn mono_downmix_6ch_surround() {
        // Edge case: User somehow has a 6-channel input device.
        // Each frame of 6 samples should be averaged into 1 mono sample.
        let samples: Vec<f32> = vec![
            0.1, 0.2, 0.3, 0.4, 0.5, 0.6,  // frame 1: avg = 0.35
            -0.6, -0.5, -0.4, -0.3, -0.2, -0.1,  // frame 2: avg = -0.35
        ];

        let mono = to_mono(&samples, 6);

        assert_eq!(mono.len(), 2);
        assert!((mono[0] - 0.35).abs() < 1e-6, "6ch frame 1: expected 0.35, got {}", mono[0]);
        assert!((mono[1] - (-0.35)).abs() < 1e-6, "6ch frame 2: expected -0.35, got {}", mono[1]);
    }

    #[test]
    fn mono_downmix_empty_input() {
        let output = to_mono(&[], 2);
        assert!(output.is_empty());
    }

    // ================================================================
    // SILENCE DETECTION — Warn user when mic is muted/broken
    // ================================================================

    #[test]
    fn silence_detected_when_all_samples_near_zero() {
        // UX: Mic is muted at the OS level. WASAPI delivers zeros faithfully.
        // App should detect this and warn the user their mic isn't working.
        let silent: Vec<f32> = vec![1e-8, -1e-9, 5e-8, 0.0, -1e-8];
        assert!(is_silence(&silent, 1e-7), "Near-zero samples should be detected as silence");
    }

    #[test]
    fn non_silence_when_samples_have_amplitude() {
        // UX: Mic is working normally. Samples have real audio content.
        // Should NOT trigger the silence warning.
        let audio: Vec<f32> = vec![0.001, -0.002, 0.005, -0.01, 0.1];
        assert!(!is_silence(&audio, 1e-7), "Normal audio should NOT be flagged as silence");
    }

    #[test]
    fn silence_threshold_boundary() {
        // Samples exactly at 1e-7 should be silent, just above should not be.
        let at_threshold: Vec<f32> = vec![9.9e-8];
        assert!(is_silence(&at_threshold, 1e-7), "Sample at threshold boundary should be silence");

        let above_threshold: Vec<f32> = vec![1.1e-7];
        assert!(!is_silence(&above_threshold, 1e-7), "Sample above threshold should not be silence");
    }

    #[test]
    fn silence_empty_input() {
        // Empty audio has max_abs = 0.0 which is below any positive threshold.
        assert!(is_silence(&[], 1e-7), "Empty input should be considered silence");
    }

    // ================================================================
    // RMS AUDIO LEVEL — VU meter displays correct level to user (F1)
    // ================================================================

    #[test]
    fn test_rms_silence() {
        // UX: Mic is capturing but room is silent. VU bar should stay at zero.
        let silence = vec![0.0f32; 1000];
        let rms = compute_rms(&silence);
        assert!(
            rms.abs() < 0.001,
            "RMS of silence should be 0.0, got {}",
            rms
        );
    }

    #[test]
    fn test_rms_full_scale() {
        // UX: Theoretical maximum signal — every sample at +1.0.
        // RMS should be exactly 1.0 (DC signal at full scale).
        let full_scale = vec![1.0f32; 500];
        let rms = compute_rms(&full_scale);
        assert!(
            (rms - 1.0).abs() < 0.001,
            "RMS of all-ones should be 1.0, got {}",
            rms
        );
    }

    #[test]
    fn test_rms_half_scale() {
        // Constant signal at 0.5 amplitude. RMS of a constant = that constant.
        let half = vec![0.5f32; 500];
        let rms = compute_rms(&half);
        assert!(
            (rms - 0.5).abs() < 0.001,
            "RMS of all-0.5 should be 0.5, got {}",
            rms
        );
    }

    #[test]
    fn test_rms_sine_wave() {
        // UX: User speaks a pure tone into the mic. The RMS of a sine wave
        // is amplitude / sqrt(2) ~= 0.7071. This validates the math is correct
        // for the most common waveform shape in real audio.
        let num_samples = 1000;
        let sine: Vec<f32> = (0..num_samples)
            .map(|i| {
                let t = i as f32 / num_samples as f32;
                (2.0 * std::f32::consts::PI * t * 10.0).sin() // 10 full cycles
            })
            .collect();

        let rms = compute_rms(&sine);
        let expected = 1.0 / (2.0f32).sqrt(); // 0.7071...

        assert!(
            (rms - expected).abs() < 0.001,
            "RMS of unit sine should be ~0.7071, got {} (expected {})",
            rms, expected
        );
    }

    #[test]
    fn test_rms_mixed_positive_negative() {
        // Squaring makes negative values positive, so RMS treats +0.5 and -0.5
        // identically. This ensures the formula handles negative samples correctly.
        let mixed = vec![0.5f32, -0.5, 0.5, -0.5, 0.5, -0.5];
        let rms = compute_rms(&mixed);

        // All values have magnitude 0.5, so RMS = sqrt(mean(0.25)) = sqrt(0.25) = 0.5
        assert!(
            (rms - 0.5).abs() < 0.001,
            "RMS of alternating +/-0.5 should be 0.5, got {}",
            rms
        );
    }

    #[test]
    fn test_rms_empty_samples() {
        // Edge case: audio callback fires with zero-length buffer (can happen
        // during device reconfiguration). Must not panic from division by zero.
        let rms = compute_rms(&[]);
        assert!(
            rms.abs() < 0.001,
            "RMS of empty slice should be 0.0 (no div by zero), got {}",
            rms
        );
    }

    #[test]
    fn test_atomic_level_store_and_load() {
        // Validates the lock-free AtomicU32 transport used between the audio
        // callback thread and the UI polling timer. f32 values must survive
        // the to_bits/from_bits round-trip through AtomicU32 without corruption.
        use std::sync::atomic::{AtomicU32, Ordering};

        let atom = AtomicU32::new(0f32.to_bits());

        // Test a range of representative VU meter values
        let test_values: &[f32] = &[0.0, 0.001, 0.1, 0.5, 0.707, 1.0];

        for &value in test_values {
            atom.store(value.to_bits(), Ordering::Relaxed);
            let loaded = f32::from_bits(atom.load(Ordering::Relaxed));
            assert!(
                (loaded - value).abs() < f32::EPSILON,
                "AtomicU32 round-trip failed for {}: stored {}, loaded {}",
                value, value, loaded
            );
        }

        // Also verify that the bit pattern for 0.0 is exactly 0u32,
        // which is what we initialize with in AudioRecorder::new()
        assert_eq!(0f32.to_bits(), 0u32, "0.0f32 should have bit pattern 0u32");
    }
}
