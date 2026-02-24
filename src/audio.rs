use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

use anyhow::Result;
use audio_gate::NoiseGate;
use cpal::traits::DeviceTrait;
use cpal::{BufferSize, Device, Stream, StreamConfig};
use ringbuf::{
    traits::{Consumer, Producer, Split},
    HeapRb,
};

/// Atomic f32 stored as bit-cast u32 for lock-free access in callbacks.
pub struct AtomicF32(AtomicU32);

impl AtomicF32 {
    pub fn new(val: f32) -> Self {
        Self(AtomicU32::new(val.to_bits()))
    }

    pub fn load(&self) -> f32 {
        f32::from_bits(self.0.load(Ordering::Relaxed))
    }

    pub fn store(&self, val: f32) {
        self.0.store(val.to_bits(), Ordering::Relaxed);
    }
}

/// Shared parameters between GUI/main thread and audio callback.
pub struct AudioParams {
    pub volume: AtomicF32,
    pub noise_gate_enabled: AtomicBool,
    pub noise_gate_threshold: AtomicF32,
    pub highpass_enabled: AtomicBool,
    pub lowpass_enabled: AtomicBool,
}

pub struct AudioEngine {
    pub input_stream: Stream,
    pub output_stream: Stream,
    _params: Arc<AudioParams>,
}

impl AudioEngine {
    pub fn build(
        input_device: &Device,
        output_device: &Device,
        sample_rate: u32,
        buffer_size: u32,
        in_channels: u16,
        out_channels: u16,
        volume: f32,
    ) -> Result<(Self, Arc<AudioParams>)> {
        let in_config = StreamConfig {
            channels: in_channels,
            sample_rate,
            buffer_size: BufferSize::Fixed(buffer_size),
        };

        let out_config = StreamConfig {
            channels: out_channels,
            sample_rate,
            buffer_size: BufferSize::Fixed(buffer_size),
        };

        let ring_capacity = (buffer_size as usize) * 4;
        let ring = HeapRb::<f32>::new(ring_capacity);
        let (mut producer, mut consumer) = ring.split();

        for _ in 0..buffer_size {
            let _ = producer.try_push(0.0f32);
        }

        let default_gate_thresh: f32 = -36.0;

        let params = Arc::new(AudioParams {
            volume: AtomicF32::new(volume),
            noise_gate_enabled: AtomicBool::new(false),
            noise_gate_threshold: AtomicF32::new(default_gate_thresh),
            highpass_enabled: AtomicBool::new(false),
            lowpass_enabled: AtomicBool::new(false),
        });
        let params_in = Arc::clone(&params);

        let sr = sample_rate as f32;
        let dt = 1.0 / sr;

        // High-pass filter state (100 Hz — remove rumble, plosives, AC hum)
        let mut hp_prev_input: f32 = 0.0;
        let mut hp_prev_output: f32 = 0.0;
        let rc_hp = 1.0 / (2.0 * std::f32::consts::PI * 100.0);
        let alpha_hp = rc_hp / (rc_hp + dt);

        // Low-pass filter state (8 kHz — remove hiss above voice range)
        let mut lp_prev_output: f32 = 0.0;
        let rc_lp = 1.0 / (2.0 * std::f32::consts::PI * 8000.0);
        let alpha_lp = dt / (rc_lp + dt);

        // Noise gate (audio-gate crate v0.2)
        let mut gate = NoiseGate::new(
            default_gate_thresh,
            default_gate_thresh - 10.0,
            sr,
            1,      // mono
            80.0,   // release rate ms
            1.0,    // attack rate ms (near-instant open)
            150.0,  // hold time ms (bridge syllable gaps)
        );
        let mut gate_thresh_cached = default_gate_thresh;

        // Pre-allocated buffer for batch noise gate processing
        let mut mono_buf: Vec<f32> = Vec::with_capacity(buffer_size as usize * 2);

        // ──────────────────────────────────────────────────────────────
        // Input callback
        //
        // Signal chain:
        //   1. Mix to mono
        //   2. High-pass 100 Hz (remove rumble/plosives)
        //   3. Low-pass 8 kHz (remove hiss above voice range)
        //   4. Noise gate (silence between words)
        //   5. Volume + push to ring buffer
        // ──────────────────────────────────────────────────────────────
        let input_stream = input_device.build_input_stream(
            &in_config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let ch = in_channels as usize;
                let vol = params_in.volume.load();
                let hp_on = params_in.highpass_enabled.load(Ordering::Relaxed);
                let lp_on = params_in.lowpass_enabled.load(Ordering::Relaxed);
                let gate_on = params_in.noise_gate_enabled.load(Ordering::Relaxed);
                let gate_thresh = params_in.noise_gate_threshold.load();

                // Update noise gate if threshold changed
                if gate_on && (gate_thresh - gate_thresh_cached).abs() > 0.1 {
                    gate_thresh_cached = gate_thresh;
                    gate.update(
                        gate_thresh,
                        gate_thresh - 10.0,
                        80.0,
                        1.0,
                        150.0,
                    );
                }

                // Mix to mono → high-pass → low-pass → into mono_buf
                mono_buf.clear();
                for frame in data.chunks_exact(ch) {
                    let mut sample: f32 = frame.iter().sum::<f32>() / ch as f32;

                    // High-pass (remove rumble)
                    if hp_on {
                        let out = alpha_hp * (hp_prev_output + sample - hp_prev_input);
                        hp_prev_input = sample;
                        hp_prev_output = out;
                        sample = out;
                    }

                    // Low-pass (remove hiss)
                    if lp_on {
                        lp_prev_output += alpha_lp * (sample - lp_prev_output);
                        sample = lp_prev_output;
                    }

                    mono_buf.push(sample);
                }

                // Noise gate (batch process)
                if gate_on {
                    gate.process_frame(&mut mono_buf);
                }

                // Volume + push to ring buffer
                for &s in &mono_buf {
                    let _ = producer.try_push(s * vol);
                }
            },
            |err| eprintln!("input error: {err}"),
            None,
        )?;

        let output_stream = output_device.build_output_stream(
            &out_config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let ch = out_channels as usize;
                for frame in data.chunks_exact_mut(ch) {
                    let sample = consumer.try_pop().unwrap_or(0.0);
                    for s in frame.iter_mut() {
                        *s = sample;
                    }
                }
            },
            |err| eprintln!("output error: {err}"),
            None,
        )?;

        let params_handle = Arc::clone(&params);
        Ok((
            Self {
                input_stream,
                output_stream,
                _params: params,
            },
            params_handle,
        ))
    }
}
