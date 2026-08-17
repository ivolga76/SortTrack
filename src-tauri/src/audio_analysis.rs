use std::{fs::File, path::Path};

use symphonia::core::{
    codecs::audio::AudioDecoderOptions,
    errors::Error as SymphoniaError,
    formats::{FormatOptions, TrackType, probe::Hint},
    io::MediaSourceStream,
    meta::MetadataOptions,
};

use crate::{audio_metadata::camelot_for_key, models::AudioAnalysisResult};

pub const ALGORITHM_VERSION: &str = "sorttrack-dsp-1";
pub(crate) const TARGET_RATE: u32 = 11_025;
const MAX_SECONDS: usize = 240;

pub fn analyze_file(path: &Path) -> Result<AudioAnalysisResult, String> {
    let samples = decode_mono(path, TARGET_RATE, MAX_SECONDS)?;
    if samples.len() < TARGET_RATE as usize * 10 {
        return Err("Недостаточно декодированного аудио для анализа".into());
    }
    let (bpm, bpm_score) = estimate_bpm(&samples, TARGET_RATE);
    let (musical_key, key_score) = estimate_key(&samples, TARGET_RATE);
    let camelot = musical_key.as_deref().and_then(camelot_for_key);
    Ok(AudioAnalysisResult {
        bpm,
        bpm_score,
        musical_key,
        key_score,
        camelot,
        algorithm_version: ALGORITHM_VERSION.into(),
    })
}

pub(crate) fn decode_mono(
    path: &Path,
    target_rate: u32,
    max_seconds: usize,
) -> Result<Vec<f32>, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let stream = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(extension) = path.extension().and_then(|value| value.to_str()) {
        hint.with_extension(extension);
    }
    let mut format = symphonia::default::get_probe()
        .probe(
            &hint,
            stream,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(|error| error.to_string())?;
    let track = format
        .default_track(TrackType::Audio)
        .ok_or_else(|| "В файле не найден аудиопоток".to_owned())?;
    let track_id = track.id;
    let codec_params = track
        .codec_params
        .as_ref()
        .and_then(|params| params.audio())
        .ok_or_else(|| "В файле отсутствуют параметры аудиокодека".to_owned())?
        .clone();
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(&codec_params, &AudioDecoderOptions::default())
        .map_err(|error| error.to_string())?;
    let mut output = Vec::with_capacity(target_rate as usize * max_seconds);
    let mut interleaved = Vec::<f32>::new();
    let mut phase = 0u64;

    loop {
        let packet = match format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => break,
            Err(error) => return Err(error.to_string()),
        };
        if packet.track_id != track_id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(error) => return Err(error.to_string()),
        };
        let sample_rate = decoded.spec().rate();
        let channels = decoded.spec().channels().count();
        if channels == 0 || sample_rate == 0 {
            continue;
        }
        interleaved.resize(decoded.samples_interleaved(), 0.0);
        decoded.copy_to_slice_interleaved(&mut interleaved);
        for frame in interleaved.chunks_exact(channels) {
            let mono = frame.iter().copied().sum::<f32>() / channels as f32;
            phase += target_rate as u64;
            if phase >= sample_rate as u64 {
                phase -= sample_rate as u64;
                output.push(mono);
                if output.len() >= target_rate as usize * max_seconds {
                    return Ok(output);
                }
            }
        }
    }
    Ok(output)
}

pub(crate) fn estimate_bpm(samples: &[f32], sample_rate: u32) -> (Option<f64>, f64) {
    const FRAME: usize = 1024;
    const HOP: usize = 256;
    if samples.len() < FRAME * 8 {
        return (None, 0.0);
    }
    let mut energy = Vec::new();
    let mut offset = 0usize;
    while offset + FRAME <= samples.len() {
        let value = samples[offset..offset + FRAME]
            .iter()
            .map(|sample| sample * sample)
            .sum::<f32>()
            / FRAME as f32;
        energy.push(value.sqrt());
        offset += HOP;
    }
    let mut onset = Vec::with_capacity(energy.len().saturating_sub(1));
    for pair in energy.windows(2) {
        onset.push((pair[1] - pair[0]).max(0.0));
    }
    let mean = onset.iter().sum::<f32>() / onset.len().max(1) as f32;
    for value in &mut onset {
        *value = (*value - mean * 0.35).max(0.0);
    }
    if onset.iter().copied().fold(0.0f32, f32::max) < 1e-5 {
        return (None, 0.0);
    }
    let min_lag = ((60.0 * sample_rate as f64) / (200.0 * HOP as f64)).floor() as usize;
    let max_lag = ((60.0 * sample_rate as f64) / (60.0 * HOP as f64)).ceil() as usize;
    let mut best_lag = 0usize;
    let mut best_score = 0.0f64;
    for lag in min_lag.max(1)..=max_lag.min(onset.len() / 2) {
        let base = normalized_correlation(&onset, lag);
        let half_time = if lag * 2 <= max_lag {
            normalized_correlation(&onset, lag * 2)
        } else {
            0.0
        };
        let double_time = if lag >= 2 {
            normalized_correlation(&onset, lag / 2)
        } else {
            0.0
        };
        let score = base + 0.35 * half_time + 0.15 * double_time;
        if score > best_score {
            best_score = score;
            best_lag = lag;
        }
    }
    if best_lag == 0 {
        return (None, 0.0);
    }
    let mut bpm = 60.0 * sample_rate as f64 / (HOP as f64 * best_lag as f64);
    if bpm < 85.0 {
        bpm *= 2.0;
    }
    if bpm > 200.0 {
        bpm *= 0.5;
    }
    (
        Some((bpm * 10.0).round() / 10.0),
        best_score.clamp(0.0, 1.0),
    )
}

fn normalized_correlation(values: &[f32], lag: usize) -> f64 {
    if lag == 0 || lag >= values.len() {
        return 0.0;
    }
    let mut product = 0.0f64;
    let mut left = 0.0f64;
    let mut right = 0.0f64;
    for index in lag..values.len() {
        let a = values[index] as f64;
        let b = values[index - lag] as f64;
        product += a * b;
        left += a * a;
        right += b * b;
    }
    if left <= f64::EPSILON || right <= f64::EPSILON {
        0.0
    } else {
        product / (left.sqrt() * right.sqrt())
    }
}

fn estimate_key(samples: &[f32], sample_rate: u32) -> (Option<String>, f64) {
    const WINDOW: usize = 4096;
    const MAX_WINDOWS: usize = 24;
    if samples.len() < WINDOW {
        return (None, 0.0);
    }
    let available = samples.len() / WINDOW;
    let step = (available / MAX_WINDOWS).max(1);
    let mut chroma = [0.0f64; 12];
    let mut used = 0usize;
    for window_index in (0..available).step_by(step).take(MAX_WINDOWS) {
        let window = &samples[window_index * WINDOW..window_index * WINDOW + WINDOW];
        let rms = (window.iter().map(|value| value * value).sum::<f32>() / WINDOW as f32).sqrt();
        if rms < 0.002 {
            continue;
        }
        for midi in 36..=95 {
            let frequency = 440.0 * 2f64.powf((midi as f64 - 69.0) / 12.0);
            chroma[midi % 12] += goertzel_power(window, frequency, sample_rate as f64);
        }
        used += 1;
    }
    if used == 0 || chroma.iter().sum::<f64>() <= f64::EPSILON {
        return (None, 0.0);
    }
    let total = chroma.iter().sum::<f64>();
    for value in &mut chroma {
        *value /= total;
    }
    const MAJOR: [f64; 12] = [
        6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88,
    ];
    const MINOR: [f64; 12] = [
        6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17,
    ];
    let mut candidates = Vec::with_capacity(24);
    for root in 0..12 {
        candidates.push((template_score(&chroma, &MAJOR, root), root, false));
        candidates.push((template_score(&chroma, &MINOR, root), root, true));
    }
    candidates.sort_by(|left, right| right.0.total_cmp(&left.0));
    let best = candidates[0];
    let second = candidates[1];
    let names = [
        "C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B",
    ];
    let mode = if best.2 { "minor" } else { "major" };
    let score = ((best.0 - second.0) / best.0.abs().max(1e-9) * 4.0).clamp(0.0, 1.0);
    (Some(format!("{} {mode}", names[best.1])), score)
}

fn goertzel_power(samples: &[f32], frequency: f64, sample_rate: f64) -> f64 {
    let omega = 2.0 * std::f64::consts::PI * frequency / sample_rate;
    let coefficient = 2.0 * omega.cos();
    let mut previous = 0.0f64;
    let mut previous_two = 0.0f64;
    let last = samples.len().saturating_sub(1).max(1) as f64;
    for (index, sample) in samples.iter().enumerate() {
        let window = 0.5 - 0.5 * (2.0 * std::f64::consts::PI * index as f64 / last).cos();
        let current = *sample as f64 * window + coefficient * previous - previous_two;
        previous_two = previous;
        previous = current;
    }
    (previous_two * previous_two + previous * previous - coefficient * previous * previous_two)
        .max(0.0)
        .sqrt()
}

fn template_score(chroma: &[f64; 12], template: &[f64; 12], root: usize) -> f64 {
    let mut dot = 0.0;
    let mut left = 0.0;
    let mut right = 0.0;
    for pitch in 0..12 {
        let expected = template[(pitch + 12 - root) % 12];
        dot += chroma[pitch] * expected;
        left += chroma[pitch] * chroma[pitch];
        right += expected * expected;
    }
    dot / (left.sqrt() * right.sqrt()).max(1e-9)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_a_steady_120_bpm_pulse_train() {
        let mut samples = vec![0.0f32; TARGET_RATE as usize * 40];
        let beat = (TARGET_RATE as f64 * 0.5) as usize;
        for start in (0..samples.len()).step_by(beat) {
            for offset in 0..200.min(samples.len() - start) {
                samples[start + offset] = (1.0 - offset as f32 / 200.0) * 0.8;
            }
        }
        let (bpm, score) = estimate_bpm(&samples, TARGET_RATE);
        assert!(
            bpm.is_some_and(|value| (value - 120.0).abs() < 3.0),
            "{bpm:?}"
        );
        assert!(score > 0.2, "{score}");
    }

    #[test]
    fn camelot_is_derived_from_the_estimated_key_name() {
        assert_eq!(camelot_for_key("A minor").as_deref(), Some("8A"));
    }
}
