//! Audio recording and speech-to-text functionality

use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Audio recorder for capturing microphone input
pub struct AudioRecorder {
    samples: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
}

impl AudioRecorder {
    /// Create a new audio recorder
    pub fn new() -> Result<Self> {
        Ok(Self {
            samples: Arc::new(Mutex::new(Vec::new())),
            sample_rate: 16000,
        })
    }

    /// Start recording audio from the default microphone
    pub fn start_recording(&mut self) -> Result<()> {
        let host = cpal::default_host();
        let device = host.default_input_device().context("No input device available")?;
        let config = device.default_input_config().context("Failed to get default input config")?;
        let stream_config: cpal::StreamConfig = config.clone().into();
        self.sample_rate = stream_config.sample_rate.0;
        let samples = Arc::clone(&self.samples);
        samples.lock().unwrap().clear();

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                self.build_stream::<f32>(&device, &stream_config, samples)?
            }
            cpal::SampleFormat::I16 => {
                self.build_stream::<i16>(&device, &stream_config, samples)?
            }
            cpal::SampleFormat::U16 => {
                self.build_stream::<u16>(&device, &stream_config, samples)?
            }
            _ => anyhow::bail!("Unsupported sample format"),
        };

        stream.play().context("Failed to start audio stream")?;
        std::mem::forget(stream);
        Ok(())
    }

    fn build_stream<T>(
        &self,
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        samples: Arc<Mutex<Vec<f32>>>,
    ) -> Result<cpal::Stream>
    where
        T: cpal::Sample + cpal::SizedSample,
        f32: cpal::FromSample<T>,
    {
        let channels = config.channels as usize;
        let stream = device.build_input_stream(
            config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                let mut samples = samples.lock().unwrap();
                for frame in data.chunks(channels) {
                    let sum: f32 = frame.iter().map(|&s| s.to_sample::<f32>()).sum();
                    samples.push(sum / channels as f32);
                }
            },
            |_err: cpal::StreamError| {},
            None,
        )?;
        Ok(stream)
    }

    /// Stop recording and save to WAV file
    pub fn stop_recording(&self, output_path: &PathBuf) -> Result<()> {
        let samples = self.samples.lock().unwrap();
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: self.sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(output_path, spec)?;
        for &sample in samples.iter() {
            let amplitude = (sample * i16::MAX as f32) as i16;
            writer.write_sample(amplitude)?;
        }
        writer.finalize()?;
        Ok(())
    }

    /// Get the recorded samples
    pub fn get_samples(&self) -> Vec<f32> {
        self.samples.lock().unwrap().clone()
    }
}

/// Download Whisper model if needed
fn ensure_whisper_model() -> Result<PathBuf> {
    let model_dir = std::env::current_dir()?.join("models");
    std::fs::create_dir_all(&model_dir)?;

    let model_path = model_dir.join("whisper-tiny.safetensors");

    if model_path.exists() {
        return Ok(model_path);
    }

    eprintln!("Downloading Whisper tiny model (39MB, this may take a moment)...");

    // Use HuggingFace Hub API for reliable downloads
    use hf_hub::api::sync::Api;
    let api = Api::new()?;
    let repo = api.model("openai/whisper-tiny".to_string());
    let downloaded_path = repo.get("model.safetensors")?;

    // Copy to our models directory
    std::fs::copy(&downloaded_path, &model_path)?;

    eprintln!("Model downloaded successfully");
    Ok(model_path)
}

/// Download tokenizer if needed
fn ensure_tokenizer() -> Result<PathBuf> {
    let model_dir = std::env::current_dir()?.join("models");
    std::fs::create_dir_all(&model_dir)?;

    let tokenizer_path = model_dir.join("tokenizer.json");

    if tokenizer_path.exists() {
        return Ok(tokenizer_path);
    }

    eprintln!("Downloading tokenizer...");

    // Use HuggingFace Hub API
    use hf_hub::api::sync::Api;
    let api = Api::new()?;
    let repo = api.model("openai/whisper-tiny".to_string());
    let downloaded_path = repo.get("tokenizer.json")?;

    // Copy to our models directory
    std::fs::copy(&downloaded_path, &tokenizer_path)?;

    eprintln!("Tokenizer downloaded successfully");
    Ok(tokenizer_path)
}

/// Transcribe audio using Candle Whisper
pub fn transcribe_audio(audio_path: &Path) -> Result<String> {
    use candle_core::{Device, IndexOp, Tensor};
    use candle_transformers::models::whisper::{self as m, Config};

    // Load audio
    let audio_data = load_audio_file(audio_path)?;

    // Ensure model and tokenizer are downloaded
    let model_path = ensure_whisper_model()?;
    let tokenizer_path = ensure_tokenizer()?;

    // Initialize device
    let device = Device::Cpu;

    // Load model with tiny config
    let config = Config {
        num_mel_bins: 80,
        max_source_positions: 1500,
        d_model: 384,
        encoder_attention_heads: 6,
        encoder_layers: 4,
        decoder_attention_heads: 6,
        decoder_layers: 4,
        vocab_size: 51865,
        max_target_positions: 448,
        suppress_tokens: vec![],
    };

    let vb = unsafe {
        candle_nn::VarBuilder::from_mmaped_safetensors(
            &[model_path],
            candle_core::DType::F32,
            &device,
        )?
    };

    let mut model = m::model::Whisper::load(&vb, config.clone())?;

    // Use mel_spec crate for mel spectrogram conversion
    use mel_spec::{mel, stft};

    // Compute STFT using Spectrogram
    let mut spectrogram = stft::Spectrogram::new(512, 160);
    let mut mel_processor = mel::MelSpectrogram::new(512, 16000.0, 80);

    // Process audio in chunks
    let mut mel_data: Vec<f32> = Vec::new();
    for chunk in audio_data.chunks(160) {
        if let Some(fft_frame) = spectrogram.add(chunk) {
            let mel_frame = mel_processor.add(&fft_frame);
            // Flatten mel frame
            for row in mel_frame.rows() {
                for &val in row.iter() {
                    mel_data.push(val as f32);
                }
            }
        }
    }

    // Pad or truncate to 3000 frames
    mel_data.resize(80 * 3000, 0.0);

    // Create mel tensor (1, 80, 3000)
    let mel = Tensor::from_vec(mel_data, (1, 80, 3000), &device)?;

    // Run encoder
    let encoder_output = model.encoder.forward(&mel, true)?;

    // Load tokenizer
    let tokenizer = tokenizers::Tokenizer::from_file(&tokenizer_path)
        .map_err(|e| anyhow::anyhow!("Tokenizer error: {}", e))?;

    // Decoder loop
    let sot_token = 50258u32;
    let eot_token = 50257u32;
    let mut tokens = vec![sot_token];

    for _ in 0..100 {
        let tokens_t = Tensor::new(&tokens[..], &device)?.unsqueeze(0)?;
        let logits = model.decoder.forward(&tokens_t, &encoder_output, true)?;

        let last_idx = logits.dim(1)? - 1;
        let logits = logits.i((0, last_idx))?;
        let next_token = logits.argmax(0)?.to_scalar::<u32>()?;

        if next_token == eot_token {
            break;
        }

        tokens.push(next_token);
    }

    // Decode tokens
    let text = tokenizer
        .decode(&tokens, true)
        .map_err(|e| anyhow::anyhow!("Decode error: {}", e))?;

    let text = text
        .trim()
        .replace("<|startoftranscript|>", "")
        .replace("<|endoftext|>", "")
        .trim()
        .to_string();

    if text.is_empty() {
        Ok("[No speech detected]".to_string())
    } else {
        Ok(text)
    }
}

/// Load audio file
fn load_audio_file(path: &Path) -> Result<Vec<f32>> {
    let mut reader = hound::WavReader::open(path)?;
    let spec = reader.spec();
    let mut samples = Vec::new();

    match spec.sample_format {
        hound::SampleFormat::Float => {
            for sample in reader.samples::<f32>() {
                samples.push(sample?);
            }
        }
        hound::SampleFormat::Int => match spec.bits_per_sample {
            16 => {
                for sample in reader.samples::<i16>() {
                    samples.push(sample? as f32 / i16::MAX as f32);
                }
            }
            32 => {
                for sample in reader.samples::<i32>() {
                    samples.push(sample? as f32 / i32::MAX as f32);
                }
            }
            _ => anyhow::bail!("Unsupported bit depth"),
        },
    }

    if spec.channels == 2 {
        samples = samples.chunks(2).map(|c| (c[0] + c[1]) / 2.0).collect();
    }

    if spec.sample_rate != 16000 {
        let ratio = spec.sample_rate as f32 / 16000.0;
        let target_len = (samples.len() as f32 / ratio) as usize;
        let mut resampled = Vec::with_capacity(target_len);
        for i in 0..target_len {
            let src_idx = (i as f32 * ratio) as usize;
            if src_idx < samples.len() {
                resampled.push(samples[src_idx]);
            }
        }
        samples = resampled;
    }

    Ok(samples)
}

/// Record and transcribe
pub async fn record_and_transcribe(duration_secs: u64) -> Result<String> {
    let temp_dir = std::env::temp_dir();
    let audio_path = temp_dir.join("dx_recording.wav");

    let mut recorder = AudioRecorder::new()?;
    recorder.start_recording()?;
    tokio::time::sleep(tokio::time::Duration::from_secs(duration_secs)).await;
    recorder.stop_recording(&audio_path)?;

    let audio_path_clone = audio_path.clone();
    let transcription =
        tokio::task::spawn_blocking(move || transcribe_audio(&audio_path_clone)).await??;

    let _ = std::fs::remove_file(&audio_path);
    Ok(transcription)
}
