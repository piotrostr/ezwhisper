use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Sample, SampleFormat};
use std::io::Cursor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AudioDevice {
    pub name: String,
    pub device: Device,
}

pub fn list_input_devices() -> Vec<AudioDevice> {
    let host = cpal::default_host();
    host.input_devices()
        .map(|devices| {
            devices
                .filter_map(|d| {
                    d.name().ok().map(|name| AudioDevice {
                        name,
                        device: d,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn get_default_device() -> Option<AudioDevice> {
    let host = cpal::default_host();
    host.default_input_device().and_then(|d| {
        d.name().ok().map(|name| AudioDevice { name, device: d })
    })
}

pub struct AudioRecorder {
    samples: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    channels: u16,
    is_recording: Arc<AtomicBool>,
    stream: cpal::Stream,
}

fn build_stream(
    device: &Device,
    samples: Arc<Mutex<Vec<f32>>>,
    is_recording: Arc<AtomicBool>,
) -> Result<(cpal::Stream, u32, u16)> {
    let config = device
        .default_input_config()
        .context("failed to get default input config")?;

    let sample_rate = config.sample_rate().0;
    let channels = config.channels();

    let err_fn = |err| tracing::error!("audio stream error: {}", err);

    let stream = match config.sample_format() {
        SampleFormat::F32 => {
            let samples_c = Arc::clone(&samples);
            let is_rec_c = Arc::clone(&is_recording);
            device.build_input_stream(
                &config.into(),
                move |data: &[f32], _| {
                    if is_rec_c.load(Ordering::SeqCst) {
                        let mut samples = samples_c.lock().unwrap();
                        samples.extend_from_slice(data);
                    }
                },
                err_fn,
                None,
            )?
        }
        SampleFormat::I16 => {
            let samples_c = Arc::clone(&samples);
            let is_rec_c = Arc::clone(&is_recording);
            device.build_input_stream(
                &config.into(),
                move |data: &[i16], _| {
                    if is_rec_c.load(Ordering::SeqCst) {
                        let mut samples = samples_c.lock().unwrap();
                        samples.extend(data.iter().map(|&s| s.to_sample::<f32>()));
                    }
                },
                err_fn,
                None,
            )?
        }
        SampleFormat::U16 => {
            let samples_c = Arc::clone(&samples);
            let is_rec_c = Arc::clone(&is_recording);
            device.build_input_stream(
                &config.into(),
                move |data: &[u16], _| {
                    if is_rec_c.load(Ordering::SeqCst) {
                        let mut samples = samples_c.lock().unwrap();
                        samples.extend(data.iter().map(|&s| s.to_sample::<f32>()));
                    }
                },
                err_fn,
                None,
            )?
        }
        _ => anyhow::bail!("unsupported sample format"),
    };

    stream.play().context("failed to start audio stream")?;

    Ok((stream, sample_rate, channels))
}

impl AudioRecorder {
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .context("no input device available")?;

        let samples = Arc::new(Mutex::new(Vec::new()));
        let is_recording = Arc::new(AtomicBool::new(false));

        let (stream, sample_rate, channels) =
            build_stream(&device, Arc::clone(&samples), Arc::clone(&is_recording))?;

        tracing::info!(
            "audio stream ready: {} Hz, {} channels",
            sample_rate,
            channels
        );

        Ok(Self {
            samples,
            sample_rate,
            channels,
            is_recording,
            stream,
        })
    }

    #[allow(dead_code)]
    pub fn set_device(&mut self, device: &Device) -> Result<()> {
        self.is_recording.store(false, Ordering::SeqCst);

        let (stream, sample_rate, channels) =
            build_stream(device, Arc::clone(&self.samples), Arc::clone(&self.is_recording))?;

        self.stream = stream;
        self.sample_rate = sample_rate;
        self.channels = channels;

        tracing::info!(
            "switched audio device: {} Hz, {} channels",
            sample_rate,
            channels
        );

        Ok(())
    }

    pub fn start(&mut self) -> Result<()> {
        if self.is_recording.load(Ordering::SeqCst) {
            return Ok(());
        }

        {
            let mut samples = self.samples.lock().unwrap();
            samples.clear();
        }

        self.is_recording.store(true, Ordering::SeqCst);
        tracing::info!("recording started");

        Ok(())
    }

    pub fn stop(&mut self) -> Result<Vec<u8>> {
        self.is_recording.store(false, Ordering::SeqCst);

        let samples = {
            let samples = self.samples.lock().unwrap();
            samples.clone()
        };

        if samples.is_empty() {
            tracing::warn!("no audio samples recorded");
            return Ok(Vec::new());
        }

        tracing::info!("recording stopped: {} samples", samples.len());

        let wav_data = self.encode_wav(&samples)?;
        Ok(wav_data)
    }

    fn encode_wav(&self, samples: &[f32]) -> Result<Vec<u8>> {
        let spec = hound::WavSpec {
            channels: self.channels,
            sample_rate: self.sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer =
                hound::WavWriter::new(&mut cursor, spec).context("failed to create wav writer")?;

            for &sample in samples {
                let amplitude = (sample * i16::MAX as f32) as i16;
                writer
                    .write_sample(amplitude)
                    .context("failed to write sample")?;
            }

            writer.finalize().context("failed to finalize wav")?;
        }

        Ok(cursor.into_inner())
    }
}
