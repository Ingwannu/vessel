//! One voice processor sees the actual rendered stream; conversion does not reset its AEC.
//!
//! Foreign callback/decoder samples are validated here before processing.
//! Capture is emitted as 16 kHz mono PCM16. A separate unfiltered converter
//! adapts spoken/cue rates before the fixed device-rate echo reference.

use std::fmt;

use bytes::Bytes;
use cpal::Sample as _;
use sonora::config::{EchoCanceller, GainController2, NoiseSuppression};
use sonora::{AudioProcessing, Config};

use super::AudioError;
use super::format::{Channels, Format, NORMALIZED_RATE, RateHz};

pub(super) struct Captured(Bytes);

impl Captured {
    pub(super) fn pcm(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for Captured {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Captured")
            .field("bytes", &self.0.len())
            .finish()
    }
}

pub(super) struct Rendered(Vec<f32>);

impl Rendered {
    pub(super) fn samples(&self) -> &[f32] {
        &self.0
    }
    pub(super) fn into_samples(self) -> Vec<f32> {
        self.0
    }
}

impl fmt::Debug for Rendered {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Rendered")
            .field("samples", &self.0.len())
            .finish()
    }
}

pub(super) struct Processor {
    voice: AudioProcessing,
    converter: AudioProcessing,
    input: Format,
    output: Format,
    normalized: Format,
    in_left: Vec<f32>,
    in_right: Vec<f32>,
    mono: Vec<f32>,
    converted_left: Vec<f32>,
    converted_right: Vec<f32>,
    render_left: Vec<f32>,
    render_right: Vec<f32>,
}

impl fmt::Debug for Processor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Processor")
            .field("input", &self.input)
            .field("output", &self.output)
            .finish_non_exhaustive()
    }
}

impl Processor {
    pub(super) fn new(input: Format, output: Format) -> Result<Self, AudioError> {
        Self::configured(
            input,
            output,
            Config {
                echo_canceller: Some(EchoCanceller::default()),
                noise_suppression: Some(NoiseSuppression::default()),
                gain_controller2: Some(GainController2::default()),
                ..Config::default()
            },
        )
    }

    fn configured(input: Format, output: Format, config: Config) -> Result<Self, AudioError> {
        let normalized = Format::new(NORMALIZED_RATE, Channels::Mono)?;
        let mut voice = AudioProcessing::builder()
            .config(config)
            .capture_config(input.config())
            .render_config(output.config())
            .build();
        voice.initialize(
            input.config(),
            normalized.config(),
            output.config(),
            output.config(),
        );
        let converter = AudioProcessing::builder().build();
        Ok(Self {
            voice,
            converter,
            input,
            output,
            normalized,
            in_left: vec![0.0; input.frames().get()],
            in_right: vec![0.0; input.frames().get()],
            mono: vec![0.0; normalized.frames().get()],
            converted_left: vec![0.0; output.frames().get()],
            converted_right: vec![0.0; output.frames().get()],
            render_left: vec![0.0; output.frames().get()],
            render_right: vec![0.0; output.frames().get()],
        })
    }

    pub(super) fn capture(&mut self, samples: &[f32]) -> Result<Captured, AudioError> {
        validate_external(samples, self.input)?;
        match self.input.channels() {
            Channels::Mono => self.in_left.copy_from_slice(samples),
            Channels::Stereo => {
                for ((pair, left), right) in samples
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .zip(&mut self.in_left)
                    .zip(&mut self.in_right)
                {
                    let [a, b] = pair;
                    *left = *a;
                    *right = *b;
                }
            }
        }
        let output = &mut [self.mono.as_mut_slice()];
        let result = match self.input.channels() {
            Channels::Mono => self.voice.process_capture_f32_with_config(
                &[&self.in_left],
                &self.input.config(),
                &self.normalized.config(),
                output,
            ),
            Channels::Stereo => self.voice.process_capture_f32_with_config(
                &[&self.in_left, &self.in_right],
                &self.input.config(),
                &self.normalized.config(),
                output,
            ),
        };
        result.map_err(AudioError::Processor)?;
        validate(&self.mono, self.normalized)?;
        let mut pcm = Vec::with_capacity(self.mono.len().saturating_mul(size_of::<i16>()));
        for &value in &self.mono {
            // dasp's normalized float contract excludes +1; clip DSP overshoot
            // before conversion rather than wrapping a loud sample's sign.
            let sample = value.clamp(-1.0, 1.0_f32.next_down()).to_sample::<i16>();
            pcm.extend_from_slice(&sample.to_le_bytes());
        }
        Ok(Captured(pcm.into()))
    }

    pub(super) fn prepare_render(
        &mut self,
        mono: &[f32],
        rate: RateHz,
    ) -> Result<Rendered, AudioError> {
        let source = Format::new(rate, Channels::Mono)?;
        validate_external(mono, source)?;
        // Duplicate mono at the conversion edge: the DSP contract permits
        // output channels equal to input or mono, never an implicit upmix.
        let stereo_source = Format::new(rate, self.output.channels())?;
        self.convert_render(mono, stereo_source)?;
        let channels = usize::from(self.output.channels().get());
        let mut samples = Vec::with_capacity(self.output.frames().get().saturating_mul(channels));
        for (&left, &right) in self.converted_left.iter().zip(&self.converted_right) {
            if !left.is_finite() || !right.is_finite() {
                return Err(AudioError::NonFinite);
            }
            samples.push(left);
            match self.output.channels() {
                Channels::Mono => {}
                Channels::Stereo => samples.push(right),
            }
        }
        Ok(Rendered(samples))
    }
    fn convert_render(&mut self, mono: &[f32], stereo_source: Format) -> Result<(), AudioError> {
        let result = match self.output.channels() {
            Channels::Mono => self.converter.process_render_f32_with_config(
                &[mono],
                &stereo_source.config(),
                &self.output.config(),
                &mut [&mut self.converted_left],
            ),
            Channels::Stereo => self.converter.process_render_f32_with_config(
                &[mono, mono],
                &stereo_source.config(),
                &self.output.config(),
                &mut [&mut self.converted_left, &mut self.converted_right],
            ),
        };
        result.map_err(AudioError::Processor)
    }

    // Feed only samples the physical output callback actually wrote. Prepared
    // audio may still be interrupted or discarded before reaching the speaker.
    pub(super) fn reference_render(&mut self, samples: &[f32]) -> Result<(), AudioError> {
        validate_external(samples, self.output)?;
        match self.output.channels() {
            Channels::Mono => self.render_left.copy_from_slice(samples),
            Channels::Stereo => {
                for ((pair, left), right) in samples
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .zip(&mut self.render_left)
                    .zip(&mut self.render_right)
                {
                    let [a, b] = pair;
                    *left = *a;
                    *right = *b;
                }
            }
        }
        let result = match self.output.channels() {
            Channels::Mono => self.voice.process_render_f32_with_config(
                &[&self.render_left],
                &self.output.config(),
                &self.output.config(),
                &mut [&mut self.converted_left],
            ),
            Channels::Stereo => self.voice.process_render_f32_with_config(
                &[&self.render_left, &self.render_right],
                &self.output.config(),
                &self.output.config(),
                &mut [&mut self.converted_left, &mut self.converted_right],
            ),
        };
        result.map_err(AudioError::Processor)
    }
}

fn validate_external(samples: &[f32], format: Format) -> Result<(), AudioError> {
    validate(samples, format)?;
    if samples.iter().any(|sample| !(-1.0..=1.0).contains(sample)) {
        return Err(AudioError::Range);
    }
    Ok(())
}

fn validate(samples: &[f32], format: Format) -> Result<(), AudioError> {
    let expected = format
        .frames()
        .get()
        .saturating_mul(usize::from(format.channels().get()));
    if samples.len() != expected {
        return Err(AudioError::Frame);
    }
    if samples.iter().any(|sample| !sample.is_finite()) {
        return Err(AudioError::NonFinite);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
