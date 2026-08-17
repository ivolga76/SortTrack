use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use chrono::Utc;
use rusqlite::params;
use serde::Serialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    error::AppError,
    file_copy::sha256_file,
    file_identity::file_identity,
    models::{DatasetClassCount, GenreDatasetAudit, GenreDatasetExportReport},
    repository::AppState,
};

const TAXONOMY_VERSION: u32 = 1;
const TARGET_TRACKS: u64 = 1000;
const BROAD_GENRES: [&str; 8] = [
    "Breaks",
    "Drum & Bass",
    "House",
    "Techno",
    "Trance",
    "Hardcore",
    "Hardstyle",
    "Electropunk",
];

#[derive(Debug)]
struct DatasetFile {
    track_id: String,
    title: String,
    artist: String,
    genre: String,
    subgenre: Option<String>,
    file_id: String,
    path: String,
    extension: String,
    size_bytes: u64,
    modified_at_ms: i64,
    expected_identity: Option<String>,
    root_path: Option<String>,
    cached_sha256: Option<String>,
    sha256_state: String,
}

#[derive(Debug)]
struct HashedFile {
    source: DatasetFile,
    sha256: String,
    recording_group: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DatasetManifest {
    schema_version: u32,
    taxonomy_version: u32,
    generated_at: String,
    split_policy: String,
    grouping_policy: String,
    entries: Vec<ManifestEntry>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ManifestEntry {
    track_id: String,
    path: String,
    format: String,
    size_bytes: u64,
    modified_at_ms: i64,
    sha256: String,
    recording_group_sha256: String,
    genre: String,
    subgenre: Option<String>,
    split: String,
}

pub fn audit(state: &AppState) -> Result<GenreDatasetAudit, AppError> {
    let connection = state.connection()?;
    let confirmed_tracks = connection.query_row(
        "SELECT count(*) FROM tracks WHERE confirmed_genre IS NOT NULL",
        [],
        |row| row.get(0),
    )?;
    let locked_tracks = connection.query_row(
        "SELECT count(*) FROM tracks WHERE confirmed_genre IS NOT NULL AND genre_locked = 1",
        [],
        |row| row.get(0),
    )?;
    let eligible_tracks = connection.query_row(
        "SELECT count(*) FROM tracks t WHERE t.confirmed_genre IS NOT NULL AND t.genre_locked = 1 AND EXISTS (SELECT 1 FROM track_files f WHERE f.track_id = t.id AND f.availability_status = 'available')",
        [],
        |row| row.get(0),
    )?;
    let unavailable_tracks = locked_tracks - eligible_tracks;
    let mut statement = connection.prepare(
        "SELECT confirmed_genre, count(*) FROM tracks t WHERE confirmed_genre IS NOT NULL AND genre_locked = 1 AND EXISTS (SELECT 1 FROM track_files f WHERE f.track_id = t.id AND f.availability_status = 'available') GROUP BY confirmed_genre ORDER BY confirmed_genre",
    )?;
    let classes = statement
        .query_map([], |row| {
            Ok(DatasetClassCount {
                genre: row.get(0)?,
                tracks: row.get(1)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let present = classes
        .iter()
        .map(|item| item.genre.clone())
        .collect::<HashSet<_>>();
    let missing_genres = BROAD_GENRES
        .iter()
        .filter(|genre| !present.contains::<str>(*genre))
        .map(|genre| (*genre).to_owned())
        .collect::<Vec<_>>();
    let mut warnings = Vec::new();
    if eligible_tracks < TARGET_TRACKS {
        warnings.push(format!(
            "До целевого объёма не хватает {} зафиксированных доступных треков",
            TARGET_TRACKS - eligible_tracks
        ));
    }
    if !missing_genres.is_empty() {
        warnings.push(
            "В разметке отсутствуют широкие жанры; оценка per-class recall будет неполной".into(),
        );
    }
    if classes.iter().any(|item| item.tracks < 30) {
        warnings.push(
            "В одном или нескольких классах меньше 30 треков; разбиение может быть нестабильным"
                .into(),
        );
    }
    if unavailable_tracks > 0 {
        warnings.push(format!(
            "Недоступных зафиксированных файлов: {unavailable_tracks}"
        ));
    }
    Ok(GenreDatasetAudit {
        confirmed_tracks,
        locked_tracks,
        eligible_tracks,
        unavailable_tracks,
        target_tracks: TARGET_TRACKS,
        classes,
        missing_genres,
        ready: eligible_tracks >= TARGET_TRACKS && present.len() == BROAD_GENRES.len(),
        warnings,
    })
}

pub fn export(state: &AppState, destination: &Path) -> Result<GenreDatasetExportReport, AppError> {
    if destination
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_lowercase)
        .as_deref()
        != Some("json")
    {
        return Err(AppError::Genre(
            "manifest должен иметь расширение .json".into(),
        ));
    }
    if destination.exists() {
        return Err(AppError::Genre(
            "manifest уже существует; перезапись запрещена".into(),
        ));
    }
    let files = load_files(state)?;
    if files.is_empty() {
        return Err(AppError::Genre(
            "нет доступных треков с зафиксированной ручной разметкой".into(),
        ));
    }
    let mut hashed_files = 0;
    let mut cached_hashes = 0;
    let mut grouped: HashMap<String, Vec<HashedFile>> = HashMap::new();
    for file in files {
        let (hash, cached) = hash_and_cache(state, &file)?;
        if cached {
            cached_hashes += 1;
        } else {
            hashed_files += 1;
        }
        let recording_group = recording_group(&file, &hash);
        grouped
            .entry(recording_group.clone())
            .or_default()
            .push(HashedFile {
                source: file,
                sha256: hash,
                recording_group,
            });
    }

    let mut conflicting_recordings = 0u64;
    let mut collapsed_recordings = 0u64;
    let mut entries = Vec::new();
    for group in grouped.into_values() {
        let labels = group
            .iter()
            .map(|item| (&item.source.genre, &item.source.subgenre))
            .collect::<HashSet<_>>();
        if labels.len() != 1 {
            conflicting_recordings += 1;
            continue;
        }
        collapsed_recordings += group.len().saturating_sub(1) as u64;
        let representative = group
            .into_iter()
            .min_by(|left, right| {
                let left_rank = if left.source.extension.eq_ignore_ascii_case("flac") {
                    0
                } else {
                    1
                };
                let right_rank = if right.source.extension.eq_ignore_ascii_case("flac") {
                    0
                } else {
                    1
                };
                left_rank
                    .cmp(&right_rank)
                    .then_with(|| left.source.path.cmp(&right.source.path))
            })
            .expect("non-empty recording group");
        let group_hash = hex_digest(representative.recording_group.as_bytes());
        entries.push(ManifestEntry {
            track_id: representative.source.track_id,
            path: representative.source.path,
            format: representative.source.extension.to_uppercase(),
            size_bytes: representative.source.size_bytes,
            modified_at_ms: representative.source.modified_at_ms,
            sha256: representative.sha256,
            recording_group_sha256: group_hash.clone(),
            genre: representative.source.genre,
            subgenre: representative.source.subgenre,
            split: split_for(&group_hash).into(),
        });
    }
    entries.sort_by(|left, right| {
        left.recording_group_sha256
            .cmp(&right.recording_group_sha256)
    });
    let manifest = DatasetManifest {
        schema_version: 1,
        taxonomy_version: TAXONOMY_VERSION,
        generated_at: Utc::now().to_rfc3339(),
        split_policy: "recording-group SHA-256: 70% train, 15% calibration, 15% test".into(),
        grouping_policy:
            "normalized artist + title; SHA-256 fallback; one representative file per group".into(),
        entries,
    };
    let bytes =
        serde_json::to_vec_pretty(&manifest).map_err(|error| AppError::Genre(error.to_string()))?;
    atomic_write(destination, &bytes)?;
    let manifest_sha256 = hex_digest(&bytes);
    let train_entries = manifest
        .entries
        .iter()
        .filter(|item| item.split == "train")
        .count() as u64;
    let calibration_entries = manifest
        .entries
        .iter()
        .filter(|item| item.split == "calibration")
        .count() as u64;
    let test_entries = manifest
        .entries
        .iter()
        .filter(|item| item.split == "test")
        .count() as u64;
    let mut counts = BTreeMap::<String, u64>::new();
    for entry in &manifest.entries {
        *counts.entry(entry.genre.clone()).or_default() += 1;
    }
    let class_counts = counts
        .into_iter()
        .map(|(genre, tracks)| DatasetClassCount { genre, tracks })
        .collect::<Vec<_>>();
    let mut warnings = vec!["Группировка между MP3 и FLAC использует artist + title; перед обучением проверьте спорные версии и ремиксы".into()];
    if conflicting_recordings > 0 {
        warnings.push(format!(
            "Исключено групп с конфликтующей разметкой: {conflicting_recordings}"
        ));
    }
    let report = GenreDatasetExportReport {
        output_path: destination.to_string_lossy().into_owned(),
        manifest_sha256: manifest_sha256.clone(),
        total_entries: manifest.entries.len() as u64,
        train_entries,
        calibration_entries,
        test_entries,
        hashed_files,
        cached_hashes,
        collapsed_recordings,
        conflicting_recordings,
        class_counts,
        warnings,
    };
    let connection = state.connection()?;
    connection.execute(
        "INSERT INTO genre_dataset_exports(id, destination_path, manifest_sha256, taxonomy_version, total_entries, train_entries, calibration_entries, test_entries, class_counts_json, report_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![Uuid::new_v4().to_string(), report.output_path, report.manifest_sha256, TAXONOMY_VERSION, report.total_entries, report.train_entries, report.calibration_entries, report.test_entries, serde_json::to_string(&report.class_counts).map_err(|error| AppError::Genre(error.to_string()))?, serde_json::to_string(&report).map_err(|error| AppError::Genre(error.to_string()))?],
    )?;
    Ok(report)
}

fn load_files(state: &AppState) -> Result<Vec<DatasetFile>, AppError> {
    let connection = state.connection()?;
    let mut statement = connection.prepare(
        "SELECT t.id, coalesce(m.title, t.preferred_title, f.filename), coalesce(m.artist, t.preferred_artist, ''), t.confirmed_genre, t.confirmed_subgenre, f.id, f.canonical_path, f.extension, f.size_bytes, f.modified_at_ms, f.file_identity, r.canonical_path, f.sha256, f.sha256_state FROM tracks t JOIN track_files f ON f.id = (SELECT preferred.id FROM track_files preferred WHERE preferred.track_id = t.id AND preferred.availability_status = 'available' ORDER BY CASE lower(preferred.extension) WHEN 'flac' THEN 0 ELSE 1 END, preferred.updated_at DESC LIMIT 1) LEFT JOIN track_metadata m ON m.track_file_id = f.id LEFT JOIN library_roots r ON r.id = f.library_root_id WHERE t.confirmed_genre IS NOT NULL AND t.genre_locked = 1 ORDER BY t.id",
    )?;
    Ok(statement
        .query_map([], |row| {
            Ok(DatasetFile {
                track_id: row.get(0)?,
                title: row.get(1)?,
                artist: row.get(2)?,
                genre: row.get(3)?,
                subgenre: row.get(4)?,
                file_id: row.get(5)?,
                path: row.get(6)?,
                extension: row.get(7)?,
                size_bytes: row.get(8)?,
                modified_at_ms: row.get(9)?,
                expected_identity: row.get(10)?,
                root_path: row.get(11)?,
                cached_sha256: row.get(12)?,
                sha256_state: row.get(13)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?)
}

fn hash_and_cache(state: &AppState, file: &DatasetFile) -> Result<(String, bool), AppError> {
    let (path, identity) = validated_path(file)?;
    if file.sha256_state == "ready" && file.cached_sha256.is_some() {
        return Ok((file.cached_sha256.clone().expect("checked"), true));
    }
    let hash = sha256_file(&path)?;
    verify_snapshot(file, &path)?;
    if file_identity(&path)? != identity {
        return Err(AppError::Genre(format!(
            "Файл изменился во время хеширования: {}",
            file.path
        )));
    }
    let connection = state.connection()?;
    connection.execute("UPDATE track_files SET sha256 = ?1, sha256_state = 'ready', file_identity = ?2, updated_at = CURRENT_TIMESTAMP WHERE id = ?3", params![hash, identity, file.file_id])?;
    Ok((hash, false))
}

fn validated_path(file: &DatasetFile) -> Result<(PathBuf, String), AppError> {
    let path = PathBuf::from(&file.path).canonicalize()?;
    if !path.is_file() {
        return Err(AppError::Genre(format!("Файл недоступен: {}", file.path)));
    }
    if let Some(root) = &file.root_path {
        let root = PathBuf::from(root).canonicalize()?;
        if !path.starts_with(root) {
            return Err(AppError::Genre(
                "Файл находится вне зарегистрированной библиотеки".into(),
            ));
        }
    }
    verify_snapshot(file, &path)?;
    let identity = file_identity(&path)?;
    if file
        .expected_identity
        .as_deref()
        .is_some_and(|expected| expected != identity)
    {
        return Err(AppError::Genre(format!(
            "Физическая идентичность файла изменилась: {}",
            file.path
        )));
    }
    Ok((path, identity))
}

fn verify_snapshot(file: &DatasetFile, path: &Path) -> Result<(), AppError> {
    let metadata = path.metadata()?;
    let modified = metadata
        .modified()?
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    if metadata.len() != file.size_bytes || modified != file.modified_at_ms {
        return Err(AppError::Genre(format!(
            "Файл изменился после сканирования: {}",
            file.path
        )));
    }
    Ok(())
}

fn recording_group(file: &DatasetFile, sha256: &str) -> String {
    let artist = normalize_text(&file.artist);
    let title = normalize_text(&file.title);
    if artist.is_empty() || title.is_empty() {
        format!("sha256:{sha256}")
    } else {
        format!("metadata:{artist}\0{title}")
    }
}

fn normalize_text(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn split_for(group_hash: &str) -> &'static str {
    let bucket = u32::from_str_radix(&group_hash[..8], 16).unwrap_or(0) % 100;
    if bucket < 70 {
        "train"
    } else if bucket < 85 {
        "calibration"
    } else {
        "test"
    }
}

fn hex_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn atomic_write(destination: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let parent = destination
        .parent()
        .ok_or_else(|| AppError::Genre("Не удалось определить папку manifest".into()))?;
    std::fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(".sorttrack-dataset-{}.tmp", Uuid::new_v4()));
    let result = (|| -> Result<(), AppError> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        if destination.exists() {
            return Err(AppError::Genre(
                "manifest уже существует; перезапись запрещена".into(),
            ));
        }
        std::fs::rename(&temporary, destination)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{audio_metadata::AudioMetadata, genre_assistant};

    #[test]
    fn audit_requires_locked_labels_and_export_collapses_same_recording() {
        let root = std::env::temp_dir().join(format!("sorttrack-dataset-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("root");
        let state = AppState::in_memory().expect("database");
        let library_root = state
            .register_root(&root.to_string_lossy())
            .expect("library root");
        for (index, extension) in ["mp3", "flac"].iter().enumerate() {
            let path = root.join(format!("Artist - Same.{extension}"));
            std::fs::write(&path, format!("audio-{index}")).expect("audio");
            let file = path.metadata().expect("metadata");
            let modified = file
                .modified()
                .expect("modified")
                .duration_since(UNIX_EPOCH)
                .expect("epoch")
                .as_millis() as i64;
            state
                .upsert_file(
                    &library_root,
                    &path.to_string_lossy(),
                    &format!("Artist - Same.{extension}"),
                    extension,
                    file.len(),
                    modified,
                    &file_identity(&path).expect("identity"),
                    &AudioMetadata::default(),
                    "available",
                )
                .expect("track");
        }
        let tracks = state.list_tracks(10, 0, "Same").expect("tracks");
        for track in &tracks {
            genre_assistant::confirm(&state, &track.id, "Techno", None, true).expect("label");
        }
        let audit = audit(&state).expect("audit");
        assert_eq!(audit.eligible_tracks, 2);
        let destination = root.join("manifest.json");
        let report = export(&state, &destination).expect("export");
        assert_eq!(report.total_entries, 1);
        assert_eq!(report.collapsed_recordings, 1);
        assert!(destination.exists());
        let manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&destination).expect("read")).expect("json");
        assert_eq!(manifest["entries"].as_array().expect("entries").len(), 1);
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
