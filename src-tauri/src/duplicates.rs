use std::{
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use rusqlite::params;

use crate::{
    error::AppError,
    file_copy::sha256_file,
    file_identity::file_identity,
    models::{DuplicateFile, DuplicateGroup, DuplicateScanResult},
    repository::AppState,
};

#[derive(Debug)]
struct HashCandidate {
    id: String,
    path: String,
    size_bytes: u64,
    modified_at_ms: i64,
    file_identity: Option<String>,
    sha256_state: String,
    sha256: Option<String>,
    root_path: Option<String>,
}

pub fn scan_exact_duplicates(state: &AppState) -> Result<DuplicateScanResult, AppError> {
    let candidates = load_candidates(state)?;
    let mut hashed_files = 0;
    let mut cached_files = 0;
    let mut failed_files = 0;

    for candidate in &candidates {
        if candidate.sha256_state == "ready"
            && candidate.sha256.is_some()
            && candidate.file_identity.is_some()
        {
            match validated_path(candidate) {
                Ok(_) => {
                    cached_files += 1;
                    continue;
                }
                Err(_) => {
                    set_hash_state(state, &candidate.id, "failed", None, None)?;
                    failed_files += 1;
                    continue;
                }
            }
        }
        set_hash_state(state, &candidate.id, "queued", None, None)?;
        match hash_candidate(candidate) {
            Ok((hash, identity)) => {
                set_hash_state(state, &candidate.id, "ready", Some(&hash), Some(&identity))?;
                hashed_files += 1;
            }
            Err(_) => {
                set_hash_state(state, &candidate.id, "failed", None, None)?;
                failed_files += 1;
            }
        }
    }

    Ok(DuplicateScanResult {
        hashed_files,
        cached_files,
        failed_files,
        candidate_files: candidates.len() as u64,
        groups: list_exact_duplicates(state)?,
    })
}

pub fn list_exact_duplicates(state: &AppState) -> Result<Vec<DuplicateGroup>, AppError> {
    let connection = state.connection()?;
    let mut hash_statement = connection.prepare(
        "SELECT sha256, max(size_bytes) FROM track_files WHERE sha256_state = 'ready' AND sha256 IS NOT NULL GROUP BY sha256 HAVING count(*) > 1 ORDER BY max(size_bytes) DESC, sha256",
    )?;
    let hashes = hash_statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(hash_statement);

    let mut groups = Vec::with_capacity(hashes.len());
    for (sha256, size_bytes) in hashes {
        let mut statement = connection.prepare(
            "SELECT f.id, t.id, coalesce(m.title, t.preferred_title, f.filename), coalesce(m.artist, t.preferred_artist, ''), upper(f.extension), f.canonical_path, f.size_bytes, f.availability_status, m.album, m.album_artist, m.original_genre_tag, m.year, m.composer, m.comment, cast(m.bpm AS TEXT), m.musical_key FROM track_files f JOIN tracks t ON t.id = f.track_id LEFT JOIN track_metadata m ON m.track_file_id = f.id WHERE f.sha256 = ?1 AND f.sha256_state = 'ready' ORDER BY f.canonical_path",
        )?;
        let files = statement
            .query_map([&sha256], |row| {
                let populated = (8..=15)
                    .map(|index| row.get::<_, Option<String>>(index))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .filter(|value| value.as_deref().is_some_and(|text| !text.trim().is_empty()))
                    .count();
                Ok(DuplicateFile {
                    file_id: row.get(0)?,
                    track_id: row.get(1)?,
                    title: row.get(2)?,
                    artist: row.get(3)?,
                    format: row.get(4)?,
                    path: row.get(5)?,
                    size_bytes: row.get(6)?,
                    availability_status: row.get(7)?,
                    metadata_completeness: ((populated * 100) / 8) as u8,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let recommended = files
            .iter()
            .max_by_key(|file| recommendation_score(file))
            .ok_or_else(|| AppError::Task("empty duplicate group".into()))?;
        let reason = recommendation_reason(recommended, &files);
        groups.push(DuplicateGroup {
            sha256,
            size_bytes,
            recommended_file_id: recommended.file_id.clone(),
            recommendation_reason: reason,
            files,
        });
    }
    Ok(groups)
}

fn load_candidates(state: &AppState) -> Result<Vec<HashCandidate>, AppError> {
    let connection = state.connection()?;
    let mut statement = connection.prepare(
        "SELECT f.id, f.canonical_path, f.size_bytes, f.modified_at_ms, f.file_identity, f.sha256_state, f.sha256, r.canonical_path FROM track_files f LEFT JOIN library_roots r ON r.id = f.library_root_id WHERE f.availability_status IN ('available', 'corrupted') AND f.size_bytes IN (SELECT size_bytes FROM track_files WHERE availability_status IN ('available', 'corrupted') GROUP BY size_bytes HAVING count(*) > 1) ORDER BY f.size_bytes, f.id",
    )?;
    Ok(statement
        .query_map([], |row| {
            Ok(HashCandidate {
                id: row.get(0)?,
                path: row.get(1)?,
                size_bytes: row.get(2)?,
                modified_at_ms: row.get(3)?,
                file_identity: row.get(4)?,
                sha256_state: row.get(5)?,
                sha256: row.get(6)?,
                root_path: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?)
}

fn validated_path(candidate: &HashCandidate) -> Result<(PathBuf, String), AppError> {
    let path = PathBuf::from(&candidate.path).canonicalize()?;
    if !path.is_file() {
        return Err(AppError::TrackUnavailable(candidate.path.clone()));
    }
    if let Some(root) = &candidate.root_path {
        let root = PathBuf::from(root).canonicalize()?;
        if !path.starts_with(root) {
            return Err(AppError::TrackUnavailable(
                "file escaped its registered library root".into(),
            ));
        }
    }
    ensure_signature(&path, candidate.size_bytes, candidate.modified_at_ms)?;
    let identity = file_identity(&path)?;
    if candidate
        .file_identity
        .as_deref()
        .is_some_and(|stored| stored != identity)
    {
        return Err(AppError::TrackUnavailable(
            "file identity changed since the library scan".into(),
        ));
    }
    Ok((path, identity))
}

fn hash_candidate(candidate: &HashCandidate) -> Result<(String, String), AppError> {
    let (path, identity) = validated_path(candidate)?;
    let hash = sha256_file(&path)?;
    ensure_signature(&path, candidate.size_bytes, candidate.modified_at_ms)?;
    if file_identity(&path)? != identity {
        return Err(AppError::TrackUnavailable(
            "file identity changed while calculating SHA-256".into(),
        ));
    }
    Ok((hash, identity))
}

fn ensure_signature(path: &Path, size: u64, modified_at_ms: i64) -> Result<(), AppError> {
    let metadata = path.metadata()?;
    let actual_modified = metadata
        .modified()?
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    if metadata.len() != size || actual_modified != modified_at_ms {
        return Err(AppError::TrackUnavailable(
            "file changed since the library scan".into(),
        ));
    }
    Ok(())
}

fn set_hash_state(
    state: &AppState,
    file_id: &str,
    hash_state: &str,
    sha256: Option<&str>,
    file_identity: Option<&str>,
) -> Result<(), AppError> {
    let connection = state.connection()?;
    connection.execute(
        "UPDATE track_files SET sha256_state = ?1, sha256 = CASE WHEN ?1 = 'ready' THEN ?2 WHEN ?1 = 'failed' THEN NULL ELSE sha256 END, file_identity = coalesce(?3, file_identity), updated_at = CURRENT_TIMESTAMP WHERE id = ?4",
        params![hash_state, sha256, file_identity, file_id],
    )?;
    Ok(())
}

fn recommendation_score(file: &DuplicateFile) -> i64 {
    let integrity = if file.availability_status == "available" {
        10_000
    } else {
        0
    };
    let format = if file.format.eq_ignore_ascii_case("flac") {
        1_000
    } else {
        0
    };
    let simple_path_bonus = 500i64.saturating_sub(file.path.chars().count() as i64);
    integrity + format + file.metadata_completeness as i64 * 5 + simple_path_bonus
}

fn recommendation_reason(recommended: &DuplicateFile, files: &[DuplicateFile]) -> String {
    if recommended.availability_status == "available"
        && files
            .iter()
            .any(|file| file.availability_status != "available")
    {
        return "Рекомендован целостный и декодируемый файл".into();
    }
    if recommended.format.eq_ignore_ascii_case("flac")
        && files
            .iter()
            .any(|file| file.format.eq_ignore_ascii_case("mp3"))
    {
        return "Рекомендован целостный FLAC; удаление не выполняется".into();
    }
    if files
        .iter()
        .any(|file| file.metadata_completeness < recommended.metadata_completeness)
    {
        return format!(
            "Рекомендована наиболее полная запись метаданных: {}%",
            recommended.metadata_completeness
        );
    }
    "Файлы полностью одинаковы; рекомендован вариант с более простым путём".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio_metadata::AudioMetadata;
    use uuid::Uuid;

    #[test]
    fn finds_exact_duplicates_and_reuses_cached_hashes() {
        let root = std::env::temp_dir().join(format!("sorttrack-duplicates-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("root");
        let state = AppState::in_memory().expect("database");
        let library_root = state
            .register_root(&root.to_string_lossy())
            .expect("library root");
        for (index, (filename, bytes)) in [
            ("Duplicate A.mp3", b"identical bytes".as_slice()),
            ("Duplicate B.mp3", b"identical bytes".as_slice()),
            ("Unique.flac", b"different length".as_slice()),
        ]
        .into_iter()
        .enumerate()
        {
            let path = root.join(filename);
            std::fs::write(&path, bytes).expect("file");
            let metadata = path.metadata().expect("metadata");
            let modified_at_ms = metadata
                .modified()
                .expect("modified")
                .duration_since(UNIX_EPOCH)
                .expect("epoch")
                .as_millis() as i64;
            state
                .upsert_file(
                    &library_root,
                    &path.to_string_lossy(),
                    filename,
                    if filename.ends_with("flac") {
                        "flac"
                    } else {
                        "mp3"
                    },
                    metadata.len(),
                    modified_at_ms,
                    &file_identity(&path).expect("file identity"),
                    &AudioMetadata::default(),
                    "available",
                )
                .unwrap_or_else(|error| panic!("track {index}: {error}"));
        }

        let first = scan_exact_duplicates(&state).expect("first scan");
        assert_eq!(first.candidate_files, 2);
        assert_eq!(first.hashed_files, 2);
        assert_eq!(first.groups.len(), 1);
        assert_eq!(first.groups[0].files.len(), 2);
        let second = scan_exact_duplicates(&state).expect("cached scan");
        assert_eq!(second.hashed_files, 0);
        assert_eq!(second.cached_files, 2);
        assert_eq!(state.summary().expect("summary").duplicates, 1);

        let replaced = root.join("Duplicate B.mp3");
        std::fs::remove_file(&replaced).expect("remove cached duplicate");
        std::fs::write(&replaced, b"changed content").expect("replace with equal-length file");
        let after_replacement = scan_exact_duplicates(&state).expect("replacement scan");
        assert_eq!(after_replacement.cached_files, 1);
        assert_eq!(after_replacement.failed_files, 1);
        assert!(after_replacement.groups.is_empty());
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
