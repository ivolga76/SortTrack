use std::{fs, path::Path, time::UNIX_EPOCH};

use rusqlite::{OptionalExtension, params};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    audio_metadata::AudioMetadata,
    error::AppError,
    file_identity::file_identity,
    models::{RelinkPreview, RelinkReport},
    repository::{AppState, split_filename},
};

struct PreparedRelink {
    preview: RelinkPreview,
    audio: AudioMetadata,
    existing_root_id: Option<String>,
    filename: String,
    extension: String,
}

pub fn preview(
    state: &AppState,
    track_id: &str,
    selected_path: &Path,
) -> Result<RelinkPreview, AppError> {
    Ok(prepare(state, track_id, selected_path)?.preview)
}

pub fn commit(
    state: &AppState,
    track_id: &str,
    selected_path: &Path,
    expected_state: &str,
) -> Result<RelinkReport, AppError> {
    let prepared = prepare(state, track_id, selected_path)?;
    if prepared.preview.expected_state != expected_state {
        return Err(AppError::Metadata(
            "Файл или запись изменились после проверки. Выполните предварительный просмотр ещё раз."
                .into(),
        ));
    }

    let mut connection = state.connection()?;
    let transaction = connection.transaction()?;
    let current: Option<(String, String, Option<String>, Option<String>)> = transaction
        .query_row(
            "SELECT canonical_path, availability_status, library_root_id, file_identity FROM track_files WHERE id = ?1 AND track_id = ?2",
            params![prepared.preview.track_file_id, track_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()?;
    let (old_path, status, old_root_id, old_identity) = current.ok_or_else(|| {
        AppError::Metadata("Запись отсутствующего файла больше не найдена".into())
    })?;
    if status != "missing" || old_path != prepared.preview.old_path {
        return Err(AppError::Metadata(
            "Состояние записи изменилось. Выполните предварительный просмотр ещё раз.".into(),
        ));
    }
    if Path::new(&old_path).try_exists()? {
        return Err(AppError::Metadata(
            "Исходный файл снова доступен; перепривязка не требуется".into(),
        ));
    }

    let normalized = prepared.preview.new_path.to_lowercase();
    let conflict: Option<String> = transaction
        .query_row(
            "SELECT id FROM track_files WHERE normalized_path_key = ?1 AND id <> ?2",
            params![normalized, prepared.preview.track_file_id],
            |row| row.get(0),
        )
        .optional()?;
    if conflict.is_some() {
        return Err(AppError::Metadata(
            "Выбранный файл уже зарегистрирован в библиотеке".into(),
        ));
    }

    let (root_id, registered_library_root) = match prepared.existing_root_id {
        Some(id) => (id, false),
        None => {
            let id = Uuid::new_v4().to_string();
            transaction.execute(
                "INSERT INTO library_roots(id, canonical_path) VALUES (?1, ?2)",
                params![id, prepared.preview.new_library_root],
            )?;
            (id, true)
        }
    };

    transaction.execute(
        "UPDATE track_files SET library_root_id = ?1, canonical_path = ?2, normalized_path_key = ?3, filename = ?4, extension = ?5, size_bytes = ?6, modified_at_ms = ?7, file_identity = ?8, sha256 = NULL, sha256_state = 'stale', acoustic_fingerprint = NULL, fingerprint_version = NULL, codec = ?9, duration_ms = ?10, bitrate_kbps = ?11, sample_rate_hz = ?12, bit_depth = ?13, channels = ?14, availability_status = ?15, metadata_reader_version = 1, last_scanned_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE id = ?16",
        params![root_id, prepared.preview.new_path, normalized, prepared.filename, prepared.extension, prepared.preview.size_bytes, prepared.preview.modified_at_ms, prepared.preview.file_identity, prepared.extension, prepared.audio.duration_ms, prepared.audio.bitrate_kbps, prepared.audio.sample_rate_hz, prepared.audio.bit_depth, prepared.audio.channels, prepared.preview.availability_status, prepared.preview.track_file_id],
    )?;
    transaction.execute(
        "DELETE FROM track_audio_analysis WHERE track_file_id = ?1",
        [&prepared.preview.track_file_id],
    )?;
    transaction.execute(
        "DELETE FROM audio_feature_cache WHERE track_file_id = ?1",
        [&prepared.preview.track_file_id],
    )?;

    let (fallback_artist, fallback_title) = split_filename(&prepared.filename);
    let title = prepared.audio.title.as_deref().unwrap_or(&fallback_title);
    let artist = prepared.audio.artist.as_deref().unwrap_or(&fallback_artist);
    transaction.execute(
        "UPDATE tracks SET preferred_title = ?1, preferred_artist = ?2, updated_at = CURRENT_TIMESTAMP WHERE id = ?3",
        params![title, artist, track_id],
    )?;
    state.write_metadata(
        &transaction,
        &prepared.preview.track_file_id,
        title,
        artist,
        &prepared.audio,
    )?;
    transaction.execute(
        "INSERT INTO file_relink_history(id, track_id, track_file_id, old_path, new_path, old_library_root_id, new_library_root_id, old_file_identity, new_file_identity, expected_state) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![Uuid::new_v4().to_string(), track_id, prepared.preview.track_file_id, old_path, prepared.preview.new_path, old_root_id, root_id, old_identity, prepared.preview.file_identity, expected_state],
    )?;
    transaction.commit()?;

    Ok(RelinkReport {
        track_id: track_id.to_owned(),
        track_file_id: prepared.preview.track_file_id,
        old_path: prepared.preview.old_path,
        new_path: prepared.preview.new_path,
        new_library_root: prepared.preview.new_library_root,
        registered_library_root,
        availability_status: prepared.preview.availability_status,
    })
}

fn prepare(
    state: &AppState,
    track_id: &str,
    selected_path: &Path,
) -> Result<PreparedRelink, AppError> {
    let canonical = selected_path.canonicalize().map_err(|error| {
        AppError::Metadata(format!("Не удалось открыть выбранный файл: {error}"))
    })?;
    if !canonical.is_file() {
        return Err(AppError::Metadata(
            "Выбранный путь не является файлом".into(),
        ));
    }
    let extension = canonical
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .filter(|value| matches!(value.as_str(), "mp3" | "flac"))
        .ok_or_else(|| AppError::Metadata("Можно выбрать только MP3 или FLAC".into()))?;
    let filename = canonical
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| AppError::Metadata("Имя файла не поддерживается".into()))?
        .to_owned();
    let new_path = canonical.to_string_lossy().into_owned();
    let metadata = fs::metadata(&canonical)?;
    let size_bytes = metadata.len();
    let modified_at_ms = metadata
        .modified()?
        .duration_since(UNIX_EPOCH)
        .map_err(|_| AppError::Metadata("Некорректное время изменения файла".into()))?
        .as_millis() as i64;
    let identity = file_identity(&canonical)?;

    let connection = state.connection()?;
    let stored: Option<(String, String, String, String, String)> = connection
        .query_row(
            "SELECT f.id, f.canonical_path, f.availability_status, coalesce(t.preferred_title, f.filename), coalesce(t.preferred_artist, '') FROM track_files f JOIN tracks t ON t.id = f.track_id WHERE f.track_id = ?1 AND f.availability_status = 'missing' ORDER BY CASE lower(f.extension) WHEN 'flac' THEN 0 ELSE 1 END, f.created_at LIMIT 1",
            [track_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .optional()?;
    let (track_file_id, old_path, old_status, title, artist) = stored
        .ok_or_else(|| AppError::Metadata("У трека нет записи с отсутствующим файлом".into()))?;
    if Path::new(&old_path).try_exists()? {
        return Err(AppError::Metadata(
            "Исходный файл снова доступен; сначала обновите библиотеку".into(),
        ));
    }
    let conflict: Option<String> = connection
        .query_row(
            "SELECT id FROM track_files WHERE normalized_path_key = ?1 AND id <> ?2",
            params![new_path.to_lowercase(), track_file_id],
            |row| row.get(0),
        )
        .optional()?;
    if conflict.is_some() {
        return Err(AppError::Metadata(
            "Выбранный файл уже зарегистрирован в библиотеке".into(),
        ));
    }

    let mut roots_statement = connection
        .prepare("SELECT id, canonical_path FROM library_roots WHERE status = 'active'")?;
    let roots = roots_statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(roots_statement);
    let root = roots
        .into_iter()
        .filter(|(_, path)| canonical.starts_with(Path::new(path)))
        .max_by_key(|(_, path)| Path::new(path).components().count());
    let (existing_root_id, new_library_root, will_register_library_root) = match root {
        Some((id, path)) => (Some(id), path, false),
        None => {
            let parent = canonical.parent().ok_or_else(|| {
                AppError::Metadata("Не удалось определить папку выбранного файла".into())
            })?;
            (None, parent.to_string_lossy().into_owned(), true)
        }
    };
    drop(connection);

    let (audio, availability_status, warning) = match AudioMetadata::read(&canonical) {
        Ok(audio) => (audio, "available".to_owned(), None),
        Err(reason) => (
            AudioMetadata::default(),
            "corrupted".to_owned(),
            Some(format!(
                "Аудиоданные не удалось прочитать. Файл будет отмечен как повреждённый: {reason}"
            )),
        ),
    };
    let expected_state = state_hash(&[
        track_id,
        &track_file_id,
        &old_path,
        &old_status,
        &new_path,
        &size_bytes.to_string(),
        &modified_at_ms.to_string(),
        &identity,
        &availability_status,
        &new_library_root,
        existing_root_id.as_deref().unwrap_or("new-root"),
    ]);

    Ok(PreparedRelink {
        preview: RelinkPreview {
            track_id: track_id.to_owned(),
            track_file_id,
            title,
            artist,
            old_path,
            new_path,
            new_library_root,
            will_register_library_root,
            format: extension.to_ascii_uppercase(),
            size_bytes,
            modified_at_ms,
            file_identity: identity,
            availability_status,
            warning,
            expected_state,
        },
        audio,
        existing_root_id,
        filename,
        extension,
    })
}

fn state_hash(parts: &[&str]) -> String {
    let mut hash = Sha256::new();
    for part in parts {
        hash.update((part.len() as u64).to_le_bytes());
        hash.update(part.as_bytes());
    }
    hash.finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::UpsertOutcome;

    fn temporary_directory() -> std::path::PathBuf {
        let directory = std::env::temp_dir().join(format!("sorttrack-relink-{}", Uuid::new_v4()));
        fs::create_dir_all(&directory).expect("temporary directory");
        directory
    }

    #[test]
    fn relink_preserves_track_playlist_and_confirmed_genre() {
        let directory = temporary_directory();
        let old_path = directory.join("Artist - Old.flac");
        fs::write(&old_path, b"old fake flac").expect("old file");
        let new_directory = directory.join("found");
        fs::create_dir_all(&new_directory).expect("new directory");
        let new_path = new_directory.join("Artist - Found.flac");
        fs::write(&new_path, b"new fake flac").expect("new file");

        let state = AppState::in_memory().expect("state");
        let root = state
            .register_root(&directory.canonicalize().unwrap().to_string_lossy())
            .expect("root");
        let old_canonical = old_path.canonicalize().unwrap();
        assert_eq!(
            state
                .upsert_file(
                    &root,
                    &old_canonical.to_string_lossy(),
                    "Artist - Old.flac",
                    "flac",
                    13,
                    1,
                    "old-identity",
                    &AudioMetadata::default(),
                    "available",
                )
                .unwrap(),
            UpsertOutcome::Added
        );
        let track_id = state.list_tracks(10, 0, "").unwrap()[0].id.clone();
        let playlist_id = state.create_playlist("Preserved").unwrap();
        state
            .place_track_in_playlist(&playlist_id, 0, &track_id, None, None)
            .unwrap();
        crate::genre_assistant::confirm(&state, &track_id, "Techno", Some("Acid Techno"), true)
            .unwrap();
        fs::remove_file(&old_path).unwrap();
        assert_eq!(state.mark_missing_files(&root).unwrap().0, 1);

        let preview = preview(&state, &track_id, &new_path).expect("preview");
        let report = commit(&state, &track_id, &new_path, &preview.expected_state).expect("commit");
        assert_eq!(report.track_id, track_id);
        assert_eq!(report.availability_status, "corrupted");
        let row = state.list_tracks(10, 0, "").unwrap().remove(0);
        assert_eq!(row.id, track_id);
        assert_eq!(row.path, new_path.canonicalize().unwrap().to_string_lossy());
        assert_eq!(row.genre.as_deref(), Some("Techno"));
        assert_eq!(row.subgenre.as_deref(), Some("Acid Techno"));
        assert_eq!(
            state.list_playlists().unwrap()[0].items[0].track.id,
            track_id
        );
        let connection = state.connection().unwrap();
        let history: u64 = connection
            .query_row("SELECT count(*) FROM file_relink_history", [], |row| {
                row.get(0)
            })
            .unwrap();
        let locked: bool = connection
            .query_row(
                "SELECT genre_locked FROM tracks WHERE id = ?1",
                [&track_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(history, 1);
        assert!(locked);
        drop(connection);
        fs::remove_dir_all(directory).ok();
    }

    #[test]
    fn relink_rejects_a_file_already_in_the_library() {
        let directory = temporary_directory();
        let state = AppState::in_memory().expect("state");
        let root = state
            .register_root(&directory.canonicalize().unwrap().to_string_lossy())
            .unwrap();
        let mut track_ids = Vec::new();
        for (name, identity) in [("Missing.flac", "one"), ("Existing.flac", "two")] {
            let path = directory.join(name);
            fs::write(&path, name.as_bytes()).unwrap();
            let canonical = path.canonicalize().unwrap();
            state
                .upsert_file(
                    &root,
                    &canonical.to_string_lossy(),
                    name,
                    "flac",
                    name.len() as u64,
                    1,
                    identity,
                    &AudioMetadata::default(),
                    "available",
                )
                .unwrap();
            track_ids.push(
                state
                    .list_tracks(1, 0, name.trim_end_matches(".flac"))
                    .unwrap()[0]
                    .id
                    .clone(),
            );
        }
        fs::remove_file(directory.join("Missing.flac")).unwrap();
        state.mark_missing_files(&root).unwrap();
        let error = preview(&state, &track_ids[0], &directory.join("Existing.flac")).unwrap_err();
        assert!(error.to_string().contains("уже зарегистрирован"));
        fs::remove_dir_all(directory).ok();
    }
}
