use crate::core::{AudioFormat, AudioSource, StreamState};

use std::sync::{Arc, Mutex};
use tracing::trace_span;

pub struct LowPass {
    buffer: Vec<f32>,
    format: AudioFormat,
    rc: f32,
    source: Arc<Mutex<dyn AudioSource + Send>>,
}

impl LowPass {
    pub fn new(source: Arc<Mutex<dyn AudioSource + Send>>, cutoff: f32) -> Self {
        let format = source.lock().unwrap().format();
        let buffer = Vec::new();
        let rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff);
        LowPass {
            buffer,
            format,
            rc,
            source,
        }
    }

    pub fn set_cutoff(&mut self, cutoff: f32) {
        self.rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff);
    }

    pub fn cutoff(&self) -> f32 {
        1.0 / (2.0 * std::f32::consts::PI * self.rc)
    }
}

impl AudioSource for LowPass {
    fn format(&mut self) -> AudioFormat {
        self.format
    }

    fn read(&mut self, samples: &mut [f32]) -> StreamState {
        let span = trace_span!("LowPass::read");
        let _span = span.enter();

        let result = self.source.lock().unwrap().read(samples);
        let written = match result {
            StreamState::Good => samples.len(),
            StreamState::Finished(n) => n,
            StreamState::Underrun(n) => n,
        };
        if written == 0 {
            return result;
        }
        self.buffer.resize(samples.len(), 0.0);

        match self.format {
            AudioFormat::Mono(sample_rate) => {
                let dt = 1.0 / sample_rate as f32;
                filter_mono(
                    &mut samples[..written],
                    &mut self.buffer[..written],
                    dt,
                    self.rc,
                );
            }
            AudioFormat::Stereo(sample_rate) => {
                let dt = 1.0 / sample_rate as f32;
                filter_stereo(
                    &mut samples[..written],
                    &mut self.buffer[..written],
                    dt,
                    self.rc,
                );
            }
        }

        result
    }
}

fn filter_mono(samples: &mut [f32], buffer: &mut [f32], dt: f32, rc: f32) {
    assert!(!samples.is_empty() && !buffer.is_empty());
    assert!(buffer.len() >= samples.len());

    let a = rc / (rc + dt);

    buffer[0] = buffer[buffer.len() - 1] + a * (samples[0] - buffer[buffer.len() - 1]);
    for i in 1..buffer.len() {
        buffer[i] = buffer[i - 1] + a * (samples[i] - buffer[i - 1]);
    }
    samples.copy_from_slice(&buffer[..samples.len()]);
}

fn filter_stereo(samples: &mut [f32], buffer: &mut [f32], dt: f32, rc: f32) {
    assert!(!samples.is_empty() && !buffer.is_empty());
    assert!(samples.len() % 2 == 0 && buffer.len() % 2 == 0);
    assert!(buffer.len() >= samples.len());

    let a = dt / (rc + dt);

    buffer[0] = buffer[buffer.len() - 2] + a * (samples[0] - buffer[buffer.len() - 2]);
    buffer[1] = buffer[buffer.len() - 1] + a * (samples[1] - buffer[buffer.len() - 1]);
    for i in 2..buffer.len() {
        buffer[i] = buffer[i - 2] + a * (samples[i] - buffer[i - 2]);
    }
    samples.copy_from_slice(&buffer[..samples.len()]);
}
