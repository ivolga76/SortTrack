use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    audio_analysis::decode_mono, error::AppError, file_identity::file_identity,
    models::WaveformData, repository::AppState,
};

const ALGORITHM_VERSION: &str = "sorttrack-waveform-2";
const BINS: usize = 384;
const TARGET_RATE: u32 = 200;
const MAX_SECONDS: usize = 6 * 60 * 60;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CacheRecord {
    algorithm_version: String,
    file_id: String,
    size_bytes: u64,
    modified_at_ms: i64,
    file_identity: String,
    peaks: Vec<f32>,
}

pub fn load(state: &AppState, track_id: &str) -> Result<WaveformData, AppError> {
    let cache_dir = state.waveform_cache_dir()?;
    load_in(state, track_id, &cache_dir)
}

fn load_in(state: &AppState, track_id: &str, cache_dir: &Path) -> Result<WaveformData, AppError> {
    let (file_id, stored_path): (String, String) = state
        .connection()?
        .query_row(
            "SELECT id, canonical_path FROM track_files WHERE track_id = ?1 AND availability_status = 'available' ORDER BY CASE lower(extension) WHEN 'flac' THEN 0 ELSE 1 END, updated_at DESC, id LIMIT 1",
            [track_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::TrackUnavailable(track_id.to_owned()))?;
    let path = PathBuf::from(stored_path)
        .canonicalize()
        .map_err(|_| AppError::TrackUnavailable(track_id.to_owned()))?;
    if !path.is_file() {
        return Err(AppError::TrackUnavailable(track_id.to_owned()));
    }
    let metadata = path.metadata()?;
    let size_bytes = metadata.len();
    let modified_at_ms = metadata
        .modified()?
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    let identity = file_identity(&path)?;
    fs::create_dir_all(cache_dir)?;
    if file_id.len() > 64
        || !file_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
    {
        return Err(AppError::Task(
            "Некорректный идентификатор waveform-кэша".into(),
        ));
    }
    let cache_path = cache_dir.join(format!("{file_id}.json"));

    if let Ok(bytes) = fs::read(&cache_path)
        && let Ok(record) = serde_json::from_slice::<CacheRecord>(&bytes)
        && record.algorithm_version == ALGORITHM_VERSION
        && record.file_id == file_id
        && record.size_bytes == size_bytes
        && record.modified_at_ms == modified_at_ms
        && record.file_identity == identity
        && valid_peaks(&record.peaks)
    {
        return Ok(WaveformData {
            peaks: record.peaks,
            cache_hit: true,
            algorithm_version: ALGORITHM_VERSION.into(),
        });
    }

    let samples = decode_mono(&path, TARGET_RATE, MAX_SECONDS)
        .map_err(|reason| AppError::Task(format!("Не удалось построить waveform: {reason}")))?;
    let peaks = downsample(&samples, BINS);
    let record = CacheRecord {
        algorithm_version: ALGORITHM_VERSION.into(),
        file_id,
        size_bytes,
        modified_at_ms,
        file_identity: identity,
        peaks: peaks.clone(),
    };
    atomic_write_json(&cache_path, &record)?;
    Ok(WaveformData {
        peaks,
        cache_hit: false,
        algorithm_version: ALGORITHM_VERSION.into(),
    })
}

fn downsample(samples: &[f32], bins: usize) -> Vec<f32> {
    if samples.is_empty() {
        return vec![0.08; bins];
    }
    let samples_per_bin = samples.len().div_ceil(bins).max(1);
    let mut peaks = (0..bins)
        .map(|index| {
            let start = (index * samples_per_bin).min(samples.len());
            let end = ((index + 1) * samples_per_bin).min(samples.len());
            samples[start..end]
                .iter()
                .map(|sample| sample.abs())
                .fold(0.0f32, f32::max)
        })
        .collect::<Vec<_>>();
    let maximum = peaks.iter().copied().fold(0.0f32, f32::max);
    if maximum > f32::EPSILON {
        for peak in &mut peaks {
            *peak = (*peak / maximum).clamp(0.08, 1.0);
        }
    } else {
        peaks.fill(0.08);
    }
    peaks
}

fn valid_peaks(peaks: &[f32]) -> bool {
    peaks.len() == BINS
        && peaks
            .iter()
            .all(|peak| peak.is_finite() && (0.0..=1.0).contains(peak))
}

fn atomic_write_json(path: &Path, value: &CacheRecord) -> Result<(), AppError> {
    let bytes = serde_json::to_vec(value).map_err(|error| AppError::Task(error.to_string()))?;
    let temporary = path.with_extension(format!("{}.tmp", Uuid::new_v4()));
    let mut file = File::create(&temporary)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        if path.is_file() {
            return Ok(());
        }
        return Err(error.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{audio_metadata::AudioMetadata, repository::UpsertOutcome};

    #[test]
    fn waveform_is_bounded_and_cache_is_reused() {
        let root = std::env::temp_dir().join(format!("sorttrack-waveform-{}", Uuid::new_v4()));
        let cache = root.join("cache");
        fs::create_dir_all(&root).expect("root");
        let path = root.join("Silence.mp3");
        let frame_size = 417;
        let mut audio = vec![0u8; frame_size * 8];
        for frame in audio.chunks_exact_mut(frame_size) {
            frame[..4].copy_from_slice(&[0xff, 0xfb, 0x90, 0x64]);
        }
        fs::write(&path, audio).expect("mp3");
        let state = AppState::in_memory().expect("state");
        let root_id = state
            .register_root(&root.canonicalize().unwrap().to_string_lossy())
            .expect("library root");
        let canonical = path.canonicalize().unwrap();
        let metadata = canonical.metadata().unwrap();
        assert_eq!(
            state
                .upsert_file(
                    &root_id,
                    &canonical.to_string_lossy(),
                    "Silence.mp3",
                    "mp3",
                    metadata.len(),
                    metadata
                        .modified()
                        .unwrap()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as i64,
                    &file_identity(&canonical).unwrap(),
                    &AudioMetadata::default(),
                    "available",
                )
                .unwrap(),
            UpsertOutcome::Added
        );
        let track_id = state.list_tracks(1, 0, "").unwrap()[0].id.clone();
        let first = load_in(&state, &track_id, &cache).expect("first waveform");
        assert!(!first.cache_hit);
        assert!(valid_peaks(&first.peaks));
        let second = load_in(&state, &track_id, &cache).expect("cached waveform");
        assert!(second.cache_hit);
        assert_eq!(first.peaks, second.peaks);
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn downsample_normalizes_and_bounds_peaks() {
        let peaks = downsample(&[0.0, -0.5, 0.25, 1.0], 2);
        assert_eq!(peaks, vec![0.5, 1.0]);
        assert!(valid_peaks(&downsample(&[], BINS)));
    }
}
