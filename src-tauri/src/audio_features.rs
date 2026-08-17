use std::path::Path;

use rustfft::{FftPlanner, num_complex::Complex32};
use serde::{Deserialize, Serialize};

use crate::audio_analysis::{TARGET_RATE, decode_mono, estimate_bpm};

pub const EXTRACTOR_VERSION: &str = "sorttrack-audio-features-1";
const FFT_SIZE: usize = 2048;
const HOP: usize = 1024;
const MAX_WINDOWS: usize = 32;
const MAX_SECONDS: usize = 120;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioFeatureVector {
    pub values: Vec<f64>,
}

pub fn extract_file(path: &Path) -> Result<AudioFeatureVector, String> {
    let samples = decode_mono(path, TARGET_RATE, MAX_SECONDS)?;
    extract_samples(&samples, TARGET_RATE)
}

fn extract_samples(samples: &[f32], sample_rate: u32) -> Result<AudioFeatureVector, String> {
    if samples.len() < sample_rate as usize * 10 {
        return Err("Недостаточно аудио для извлечения признаков".into());
    }
    let mut frames = Vec::<(usize, f64)>::new();
    for start in (0..=samples.len().saturating_sub(FFT_SIZE)).step_by(HOP) {
        let rms = (samples[start..start + FFT_SIZE]
            .iter()
            .map(|value| (*value as f64).powi(2))
            .sum::<f64>()
            / FFT_SIZE as f64)
            .sqrt();
        frames.push((start, rms));
    }
    let max_rms = frames.iter().map(|(_, rms)| *rms).fold(0.0, f64::max);
    let active = frames
        .into_iter()
        .filter(|(_, rms)| *rms >= (max_rms * 0.05).max(0.001))
        .collect::<Vec<_>>();
    if active.len() < 4 {
        return Err("После исключения тишины осталось слишком мало аудио".into());
    }
    let selected = evenly_spaced(&active, MAX_WINDOWS);
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FFT_SIZE);
    let mut buffer = vec![Complex32::new(0.0, 0.0); FFT_SIZE];
    let mut rows = Vec::<Vec<f64>>::with_capacity(selected.len());
    let mut previous_spectrum: Option<Vec<f64>> = None;
    for (start, rms) in selected {
        let window = &samples[*start..*start + FFT_SIZE];
        let last = (FFT_SIZE - 1) as f32;
        for (index, sample) in window.iter().enumerate() {
            let hann = 0.5 - 0.5 * (2.0 * std::f32::consts::PI * index as f32 / last).cos();
            buffer[index] = Complex32::new(*sample * hann, 0.0);
        }
        fft.process(&mut buffer);
        let spectrum = buffer[1..FFT_SIZE / 2]
            .iter()
            .map(|value| value.norm_sqr() as f64)
            .collect::<Vec<_>>();
        let total = spectrum.iter().sum::<f64>().max(1e-12);
        let normalized = spectrum
            .iter()
            .map(|value| value / total)
            .collect::<Vec<_>>();
        let nyquist = sample_rate as f64 / 2.0;
        let frequency = |index: usize| (index + 1) as f64 * sample_rate as f64 / FFT_SIZE as f64;
        let centroid_hz = normalized
            .iter()
            .enumerate()
            .map(|(index, value)| frequency(index) * value)
            .sum::<f64>();
        let bandwidth = normalized
            .iter()
            .enumerate()
            .map(|(index, value)| (frequency(index) - centroid_hz).powi(2) * value)
            .sum::<f64>()
            .sqrt()
            / nyquist;
        let mut cumulative = 0.0;
        let mut rolloff = 0.0;
        for (index, value) in normalized.iter().enumerate() {
            cumulative += value;
            if cumulative >= 0.85 {
                rolloff = frequency(index) / nyquist;
                break;
            }
        }
        let arithmetic = spectrum.iter().sum::<f64>() / spectrum.len() as f64;
        let geometric = (spectrum
            .iter()
            .map(|value| (value + 1e-12).ln())
            .sum::<f64>()
            / spectrum.len() as f64)
            .exp();
        let flatness = (geometric / (arithmetic + 1e-12)).clamp(0.0, 1.0);
        let band = |low: f64, high: f64| {
            normalized
                .iter()
                .enumerate()
                .filter(|(index, _)| {
                    let hz = frequency(*index);
                    hz >= low && hz < high
                })
                .map(|(_, value)| value)
                .sum::<f64>()
        };
        let zero_crossing = window
            .windows(2)
            .filter(|pair| pair[0].is_sign_positive() != pair[1].is_sign_positive())
            .count() as f64
            / (FFT_SIZE - 1) as f64;
        let flux = previous_spectrum
            .as_ref()
            .map(|previous| {
                normalized
                    .iter()
                    .zip(previous)
                    .map(|(current, old)| (current - old).max(0.0))
                    .sum()
            })
            .unwrap_or(0.0);
        let bass = band(20.0, 250.0);
        let low_mid = band(250.0, 1000.0);
        let mid = band(1000.0, 4000.0);
        let high = band(4000.0, nyquist + 1.0);
        previous_spectrum = Some(normalized);
        rows.push(vec![
            *rms,
            zero_crossing,
            centroid_hz / nyquist,
            bandwidth,
            rolloff,
            flatness,
            bass,
            low_mid,
            mid,
            high,
            flux,
        ]);
    }
    let dimensions = rows[0].len();
    let mut values = Vec::with_capacity(dimensions * 2 + 3);
    for dimension in 0..dimensions {
        let mean = rows.iter().map(|row| row[dimension]).sum::<f64>() / rows.len() as f64;
        values.push(mean);
    }
    for dimension in 0..dimensions {
        let mean = values[dimension];
        let variance = rows
            .iter()
            .map(|row| (row[dimension] - mean).powi(2))
            .sum::<f64>()
            / rows.len() as f64;
        values.push(variance.sqrt());
    }
    let (bpm, bpm_score) = estimate_bpm(samples, sample_rate);
    values.push(bpm.unwrap_or(0.0) / 200.0);
    values.push(bpm_score);
    values.push((samples.len() as f64 / sample_rate as f64).ln_1p() / 8.0);
    Ok(AudioFeatureVector { values })
}

fn evenly_spaced<T>(values: &[T], limit: usize) -> Vec<&T> {
    if values.len() <= limit {
        return values.iter().collect();
    }
    (0..limit)
        .map(|index| &values[index * (values.len() - 1) / (limit - 1)])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spectral_centroid_separates_low_and_high_tones() {
        let seconds = 12;
        let tone = |frequency: f32| {
            (0..TARGET_RATE as usize * seconds)
                .map(|index| {
                    (2.0 * std::f32::consts::PI * frequency * index as f32 / TARGET_RATE as f32)
                        .sin()
                        * 0.5
                })
                .collect::<Vec<_>>()
        };
        let low = extract_samples(&tone(120.0), TARGET_RATE).expect("low");
        let high = extract_samples(&tone(2500.0), TARGET_RATE).expect("high");
        assert_eq!(low.values.len(), 25);
        assert!(
            high.values[2] > low.values[2] * 5.0,
            "{} <= {}",
            high.values[2],
            low.values[2]
        );
    }
}
