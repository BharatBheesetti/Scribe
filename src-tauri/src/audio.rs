use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{WavSpec, WavWriter};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const SAMPLE_RATE: u32 = 16000;
const CHANNELS: u16 = 1;
const MAX_RECORDING_DURATION: Duration = Duration::from_secs(60);

/// Wrapper to make cpal::Stream usable across threads.
/// cpal::Stream is !Send on some platforms for safety, but on Windows (WASAPI)
/// it's safe as long as access is synchronized through a Mutex.
struct SendStream(cpal::Stream);
unsafe impl Send for SendStream {}

pub struct AudioRecorder {
    sample_rate: u32,
    channels: u16,
    samples: Arc<Mutex<Vec<f32>>>,
    stream: Option<SendStream>,
    start_time: Option<Instant>,
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            sample_rate: SAMPLE_RATE,
            channels: CHANNELS,
            samples: Arc::new(Mutex::new(Vec::new())),
            stream: None,
            start_time: None,
        }
    }

    pub fn start_recording(&mut self) -> Result<(), String> {
        // Clear previous samples
        self.samples.lock().unwrap().clear();
        self.start_time = Some(Instant::now());

        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("No input device found")?;

        let _config = device
            .default_input_config()
            .map_err(|e| format!("Failed to get input config: {}", e))?;

        let samples = Arc::clone(&self.samples);
        let start_time = self.start_time.unwrap();

        let stream = device
            .build_input_stream(
                &cpal::StreamConfig {
                    channels: self.channels,
                    sample_rate: cpal::SampleRate(self.sample_rate),
                    buffer_size: cpal::BufferSize::Fixed(2048),
                },
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    // Auto-stop after max duration
                    if start_time.elapsed() > MAX_RECORDING_DURATION {
                        return;
                    }
                    samples.lock().unwrap().extend_from_slice(data);
                },
                |err| eprintln!("Audio stream error: {}", err),
                None,
            )
            .map_err(|e| format!("Failed to build input stream: {}", e))?;

        stream
            .play()
            .map_err(|e| format!("Failed to start audio stream: {}", e))?;

        self.stream = Some(SendStream(stream));
        Ok(())
    }

    pub fn stop_recording(&mut self) -> Result<PathBuf, String> {
        // Stop stream
        if let Some(SendStream(stream)) = self.stream.take() {
            drop(stream);
        }

        // Get samples
        let samples = self.samples.lock().unwrap();
        if samples.is_empty() {
            return Err("No audio recorded".to_string());
        }

        // Generate unique filename
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join(format!("scribe_{}.wav", uuid::Uuid::new_v4()));

        // Write WAV file
        let spec = WavSpec {
            channels: self.channels,
            sample_rate: self.sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = WavWriter::create(&file_path, spec)
            .map_err(|e| format!("Failed to create WAV file: {}", e))?;

        for sample in samples.iter() {
            let amplitude = (sample * i16::MAX as f32) as i16;
            writer
                .write_sample(amplitude)
                .map_err(|e| format!("Failed to write sample: {}", e))?;
        }

        writer
            .finalize()
            .map_err(|e| format!("Failed to finalize WAV file: {}", e))?;

        Ok(file_path)
    }

    pub fn cancel_recording(&mut self) {
        if let Some(SendStream(stream)) = self.stream.take() {
            drop(stream);
        }
        self.samples.lock().unwrap().clear();
        self.start_time = None;
    }
}
