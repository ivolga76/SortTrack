use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Mutex,
};

use rusqlite::{Connection, OptionalExtension, params};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    audio_metadata::AudioMetadata,
    error::AppError,
    genre_assistant,
    models::{
        BulkRatingReport, BulkRatingReportItem, LibraryColumnLayout, LibraryRootRemovalPreview,
        LibraryRootRemovalReport, LibraryRootSettings, LibrarySummary, LibraryTableLayout,
        PlaylistDetail, PlaylistItemRow, PlaylistRemovalPreview, PlaylistRemovalReport,
        PlaylistSnapshotReport, SmartPlaylistRule, TrackPage, TrackRow,
    },
};

#[derive(Clone)]
pub struct AppState {
    connection: std::sync::Arc<Mutex<Connection>>,
    database_path: Option<PathBuf>,
}

pub const CURRENT_SCHEMA_VERSION: u32 = 13;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpsertOutcome {
    Added,
    Updated,
    Moved,
    Restored,
    Unchanged,
}

type ExistingFileRow = (
    String,
    String,
    Option<String>,
    u64,
    i64,
    Option<String>,
    String,
);
type StoredFileSignature = (Option<String>, u64, i64, Option<String>, u32, String);

const LIBRARY_COLUMN_IDS: [(&str, u16, u16, u16); 6] = [
    ("track", 320, 220, 520),
    ("genre", 170, 100, 300),
    ("bpm", 72, 56, 120),
    ("key", 130, 90, 220),
    ("format", 82, 68, 130),
    ("duration", 76, 62, 130),
];

fn default_library_table_layout() -> LibraryTableLayout {
    LibraryTableLayout {
        columns: LIBRARY_COLUMN_IDS
            .iter()
            .map(|(id, width, _, _)| LibraryColumnLayout {
                id: (*id).into(),
                visible: true,
                width: *width,
            })
            .collect(),
    }
}

fn validate_library_table_layout(layout: &LibraryTableLayout) -> Result<(), AppError> {
    if layout.columns.len() != LIBRARY_COLUMN_IDS.len() {
        return Err(AppError::Task(
            "Library table layout must contain every supported column exactly once".into(),
        ));
    }
    for (id, _, minimum, maximum) in LIBRARY_COLUMN_IDS {
        let matches: Vec<_> = layout
            .columns
            .iter()
            .filter(|column| column.id == id)
            .collect();
        if matches.len() != 1 {
            return Err(AppError::Task(format!(
                "Library table column '{id}' is missing or duplicated"
            )));
        }
        if !(minimum..=maximum).contains(&matches[0].width) {
            return Err(AppError::Task(format!(
                "Library table column '{id}' has an unsafe width"
            )));
        }
    }
    if !layout
        .columns
        .iter()
        .any(|column| column.id == "track" && column.visible)
    {
        return Err(AppError::Task(
            "The main track column cannot be hidden".into(),
        ));
    }
    if !layout.columns.iter().any(|column| column.visible) {
        return Err(AppError::Task(
            "At least one library table column must remain visible".into(),
        ));
    }
    Ok(())
}

impl AppState {
    pub fn open(database_path: &Path) -> Result<Self, AppError> {
        if let Some(parent) = database_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let connection = Connection::open(database_path)?;
        connection.execute_batch(
            "PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL; PRAGMA busy_timeout = 5000;",
        )?;
        let version: u32 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version == 0 {
            connection.execute_batch(include_str!("../migrations/0001_initial.sql"))?;
        }
        if version < 2 {
            connection.execute_batch(include_str!("../migrations/0002_metadata_reader.sql"))?;
        }
        if version < 3 {
            connection.execute_batch(include_str!("../migrations/0003_copy_recovery.sql"))?;
        }
        if version < 4 {
            connection.execute_batch(include_str!("../migrations/0004_audio_analysis.sql"))?;
        }
        if version < 5 {
            connection.execute_batch(include_str!("../migrations/0005_genre_assistant.sql"))?;
        }
        if version < 6 {
            connection.execute_batch(include_str!("../migrations/0006_genre_dataset.sql"))?;
        }
        if version < 7 {
            connection.execute_batch(include_str!("../migrations/0007_genre_model.sql"))?;
        }
        if version < 8 {
            connection.execute_batch(include_str!("../migrations/0008_file_relink.sql"))?;
        }
        if version < 9 {
            connection.execute_batch(include_str!("../migrations/0009_track_search_index.sql"))?;
        }
        if version < 10 {
            connection.execute_batch(include_str!("../migrations/0010_ui_preferences.sql"))?;
        }
        if version < 11 {
            connection.execute_batch(include_str!("../migrations/0011_smart_playlists.sql"))?;
        }
        if version < 12 {
            connection.execute_batch(include_str!(
                "../migrations/0012_smart_playlist_filters.sql"
            ))?;
        }
        if version < 13 {
            connection
                .execute_batch(include_str!("../migrations/0013_track_rating_history.sql"))?;
        }
        Ok(Self {
            connection: std::sync::Arc::new(Mutex::new(connection)),
            database_path: Some(database_path.canonicalize()?),
        })
    }

    #[cfg(test)]
    pub fn in_memory() -> Result<Self, AppError> {
        let connection = Connection::open_in_memory()?;
        connection.execute_batch(include_str!("../migrations/0001_initial.sql"))?;
        connection.execute_batch(include_str!("../migrations/0002_metadata_reader.sql"))?;
        connection.execute_batch(include_str!("../migrations/0003_copy_recovery.sql"))?;
        connection.execute_batch(include_str!("../migrations/0004_audio_analysis.sql"))?;
        connection.execute_batch(include_str!("../migrations/0005_genre_assistant.sql"))?;
        connection.execute_batch(include_str!("../migrations/0006_genre_dataset.sql"))?;
        connection.execute_batch(include_str!("../migrations/0007_genre_model.sql"))?;
        connection.execute_batch(include_str!("../migrations/0008_file_relink.sql"))?;
        connection.execute_batch(include_str!("../migrations/0009_track_search_index.sql"))?;
        connection.execute_batch(include_str!("../migrations/0010_ui_preferences.sql"))?;
        connection.execute_batch(include_str!("../migrations/0011_smart_playlists.sql"))?;
        connection.execute_batch(include_str!(
            "../migrations/0012_smart_playlist_filters.sql"
        ))?;
        connection.execute_batch(include_str!("../migrations/0013_track_rating_history.sql"))?;
        Ok(Self {
            connection: std::sync::Arc::new(Mutex::new(connection)),
            database_path: None,
        })
    }

    pub(crate) fn connection(&self) -> Result<std::sync::MutexGuard<'_, Connection>, AppError> {
        self.connection
            .lock()
            .map_err(|_| AppError::StateUnavailable)
    }

    pub(crate) fn database_path(&self) -> Result<&Path, AppError> {
        self.database_path
            .as_deref()
            .ok_or_else(|| AppError::Task("Операция недоступна для временной базы".into()))
    }

    pub(crate) fn waveform_cache_dir(&self) -> Result<PathBuf, AppError> {
        let database_directory = self
            .database_path()?
            .parent()
            .ok_or_else(|| AppError::Task("Не удалось определить папку базы данных".into()))?;
        let application_directory = database_directory.parent().ok_or_else(|| {
            AppError::Task("Не удалось определить системную папку SortTrack".into())
        })?;
        Ok(application_directory.join("cache").join("waveforms"))
    }

    pub(crate) fn cover_cache_dir(&self) -> Result<PathBuf, AppError> {
        let database_directory = self
            .database_path()?
            .parent()
            .ok_or_else(|| AppError::Task("Не удалось определить папку базы данных".into()))?;
        let application_directory = database_directory.parent().ok_or_else(|| {
            AppError::Task("Не удалось определить системную папку SortTrack".into())
        })?;
        Ok(application_directory.join("cache").join("covers"))
    }

    pub fn summary(&self) -> Result<LibrarySummary, AppError> {
        let connection = self.connection()?;
        let tracks = connection.query_row("SELECT count(*) FROM tracks", [], |row| row.get(0))?;
        let incoming = connection.query_row(
            "SELECT count(*) FROM tracks WHERE confirmed_genre IS NULL",
            [],
            |row| row.get(0),
        )?;

        let duplicates = connection.query_row(
            "SELECT count(*) FROM (SELECT sha256 FROM track_files WHERE sha256_state = 'ready' AND sha256 IS NOT NULL GROUP BY sha256 HAVING count(*) > 1)",
            [],
            |row| row.get(0),
        )?;
        Ok(LibrarySummary {
            tracks,
            incoming,
            needs_review: incoming,
            duplicates,
        })
    }

    pub fn library_table_layout(&self) -> Result<LibraryTableLayout, AppError> {
        let connection = self.connection()?;
        let stored: Option<String> = connection
            .query_row(
                "SELECT value_json FROM app_settings WHERE key = 'library.table.layout.v1'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        let Some(stored) = stored else {
            return Ok(default_library_table_layout());
        };
        let layout: LibraryTableLayout = serde_json::from_str(&stored)?;
        validate_library_table_layout(&layout)?;
        Ok(layout)
    }

    pub fn save_library_table_layout(
        &self,
        layout: LibraryTableLayout,
    ) -> Result<LibraryTableLayout, AppError> {
        validate_library_table_layout(&layout)?;
        let value = serde_json::to_string(&layout)?;
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO app_settings(key, value_json) VALUES ('library.table.layout.v1', ?1) ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json, updated_at = CURRENT_TIMESTAMP",
            [value],
        )?;
        Ok(layout)
    }

    pub fn register_root(&self, canonical_path: &str) -> Result<String, AppError> {
        let connection = self.connection()?;
        if let Some(id) = connection
            .query_row(
                "SELECT id FROM library_roots WHERE canonical_path = ?1",
                [canonical_path],
                |row| row.get(0),
            )
            .optional()?
        {
            return Ok(id);
        }

        let id = Uuid::new_v4().to_string();
        connection.execute(
            "INSERT INTO library_roots(id, canonical_path) VALUES (?1, ?2)",
            params![id, canonical_path],
        )?;
        Ok(id)
    }

    pub fn list_library_roots(&self) -> Result<Vec<LibraryRootSettings>, AppError> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT id, canonical_path, scan_subfolders, exclusions_json, status, last_scanned_at FROM library_roots ORDER BY canonical_path COLLATE NOCASE",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, bool>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })?;
        rows.map(|row| {
            let (id, path, recursive, exclusions_json, status, last_scanned_at) = row?;
            let exclusions = serde_json::from_str::<Vec<String>>(&exclusions_json)?;
            Ok(LibraryRootSettings {
                id,
                path,
                recursive,
                exclusions,
                status,
                last_scanned_at,
            })
        })
        .collect()
    }

    pub fn set_library_root_enabled(
        &self,
        root_id: &str,
        expected_status: &str,
        enabled: bool,
    ) -> Result<(), AppError> {
        if !matches!(expected_status, "active" | "unavailable" | "disabled") {
            return Err(AppError::InvalidLibraryRoot(
                "неизвестный ожидаемый статус".into(),
            ));
        }

        let connection = self.connection()?;
        let path = connection
            .query_row(
                "SELECT canonical_path FROM library_roots WHERE id = ?1",
                [root_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| AppError::InvalidLibraryRoot("папка не зарегистрирована".into()))?;

        if enabled && !std::path::Path::new(&path).is_dir() {
            return Err(AppError::InvalidLibraryRoot(format!(
                "папка недоступна или больше не является каталогом: {path}"
            )));
        }

        let next_status = if enabled { "active" } else { "disabled" };
        let changed = connection.execute(
            "UPDATE library_roots SET status = ?1 WHERE id = ?2 AND status = ?3",
            params![next_status, root_id, expected_status],
        )?;
        if changed != 1 {
            return Err(AppError::InvalidLibraryRoot(
                "статус папки изменился; обновите список и повторите действие".into(),
            ));
        }
        Ok(())
    }

    pub fn refresh_library_root_availability(&self) -> Result<Vec<LibraryRootSettings>, AppError> {
        let active_roots = {
            let connection = self.connection()?;
            let mut statement = connection
                .prepare("SELECT id, canonical_path FROM library_roots WHERE status = 'active'")?;
            statement
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<Result<Vec<_>, _>>()?
        };

        let unavailable_ids = active_roots
            .into_iter()
            .filter_map(|(id, path)| (!std::path::Path::new(&path).is_dir()).then_some(id))
            .collect::<Vec<_>>();
        if !unavailable_ids.is_empty() {
            let mut connection = self.connection()?;
            let transaction = connection.transaction()?;
            for root_id in unavailable_ids {
                transaction.execute(
                    "UPDATE library_roots SET status = 'unavailable' WHERE id = ?1 AND status = 'active'",
                    [root_id],
                )?;
            }
            transaction.commit()?;
        }
        self.list_library_roots()
    }

    pub fn preview_library_root_removal(
        &self,
        root_id: &str,
    ) -> Result<LibraryRootRemovalPreview, AppError> {
        let connection = self.connection()?;
        let preview = connection
            .query_row(
                "SELECT r.canonical_path, r.status, COUNT(f.id) FROM library_roots r LEFT JOIN track_files f ON f.library_root_id = r.id WHERE r.id = ?1 GROUP BY r.id, r.canonical_path, r.status",
                [root_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, u64>(2)?)),
            )
            .optional()?
            .ok_or_else(|| AppError::InvalidLibraryRoot("папка не зарегистрирована".into()))?;
        if preview.1 != "disabled" {
            return Err(AppError::InvalidLibraryRoot(
                "перед удалением регистрации папку необходимо отключить".into(),
            ));
        }
        Ok(LibraryRootRemovalPreview {
            root_id: root_id.into(),
            path: preview.0,
            linked_files: preview.2,
        })
    }

    pub fn remove_library_root_registration(
        &self,
        root_id: &str,
        expected_linked_files: u64,
    ) -> Result<LibraryRootRemovalReport, AppError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let current = transaction
            .query_row(
                "SELECT r.canonical_path, r.status, COUNT(f.id) FROM library_roots r LEFT JOIN track_files f ON f.library_root_id = r.id WHERE r.id = ?1 GROUP BY r.id, r.canonical_path, r.status",
                [root_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, u64>(2)?)),
            )
            .optional()?
            .ok_or_else(|| AppError::InvalidLibraryRoot("папка не зарегистрирована".into()))?;
        if current.1 != "disabled" || current.2 != expected_linked_files {
            return Err(AppError::InvalidLibraryRoot(
                "состояние папки изменилось; создайте новый предварительный просмотр".into(),
            ));
        }
        let deleted = transaction.execute(
            "DELETE FROM library_roots WHERE id = ?1 AND status = 'disabled'",
            [root_id],
        )?;
        if deleted != 1 {
            return Err(AppError::InvalidLibraryRoot(
                "регистрация папки не была удалена".into(),
            ));
        }
        transaction.commit()?;
        Ok(LibraryRootRemovalReport {
            root_id: root_id.into(),
            path: current.0,
            detached_files: current.2,
        })
    }

    pub fn update_root_scan_settings(
        &self,
        root_id: &str,
        recursive: bool,
        exclusions: &[String],
    ) -> Result<(), AppError> {
        let exclusions_json = serde_json::to_string(exclusions)?;
        let connection = self.connection()?;
        connection.execute(
            "UPDATE library_roots SET scan_subfolders = ?1, exclusions_json = ?2, last_scanned_at = CURRENT_TIMESTAMP WHERE id = ?3",
            params![recursive, exclusions_json, root_id],
        )?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)] // Atomic scanner upsert mirrors the persisted file signature.
    pub fn upsert_file(
        &self,
        root_id: &str,
        canonical_path: &str,
        filename: &str,
        extension: &str,
        size_bytes: u64,
        modified_at_ms: i64,
        file_identity: &str,
        audio: &AudioMetadata,
        availability_status: &str,
    ) -> Result<UpsertOutcome, AppError> {
        let connection = self.connection()?;
        let normalized = canonical_path.to_lowercase();
        let existing: Option<ExistingFileRow> = connection
            .query_row(
                "SELECT id, track_id, library_root_id, size_bytes, modified_at_ms, file_identity, availability_status FROM track_files WHERE normalized_path_key = ?1",
                [&normalized],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?)),
            )
            .optional()?;

        let (fallback_artist, fallback_title) = split_filename(filename);
        let title = audio.title.as_deref().unwrap_or(&fallback_title);
        let artist = audio.artist.as_deref().unwrap_or(&fallback_artist);

        if let Some((id, track_id, old_root_id, old_size, old_modified, old_identity, old_status)) =
            existing
        {
            if old_size == size_bytes
                && old_modified == modified_at_ms
                && old_identity.as_deref() == Some(file_identity)
                && old_status == availability_status
                && old_root_id.as_deref() == Some(root_id)
            {
                return Ok(UpsertOutcome::Unchanged);
            }
            connection.execute(
                "UPDATE track_files SET library_root_id = ?1, size_bytes = ?2, modified_at_ms = ?3, file_identity = ?4, sha256 = NULL, sha256_state = 'stale', codec = ?5, duration_ms = ?6, bitrate_kbps = ?7, sample_rate_hz = ?8, bit_depth = ?9, channels = ?10, availability_status = ?11, metadata_reader_version = 1, last_scanned_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE id = ?12",
                params![root_id, size_bytes, modified_at_ms, file_identity, extension, audio.duration_ms, audio.bitrate_kbps, audio.sample_rate_hz, audio.bit_depth, audio.channels, availability_status, id],
            )?;
            connection.execute(
                "DELETE FROM track_audio_analysis WHERE track_file_id = ?1",
                [&id],
            )?;
            connection.execute(
                "DELETE FROM audio_feature_cache WHERE track_file_id = ?1",
                [&id],
            )?;
            connection.execute(
                "UPDATE tracks SET preferred_title = ?1, preferred_artist = ?2, updated_at = CURRENT_TIMESTAMP WHERE id = ?3",
                params![title, artist, track_id],
            )?;
            self.write_metadata(&connection, &id, title, artist, audio)?;
            return Ok(if old_status == "missing" {
                UpsertOutcome::Restored
            } else {
                UpsertOutcome::Updated
            });
        }

        let mut identity_statement = connection.prepare(
            "SELECT id, track_id, canonical_path FROM track_files WHERE library_root_id = ?1 AND file_identity = ?2 AND normalized_path_key <> ?3 LIMIT 2",
        )?;
        let identity_matches = identity_statement
            .query_map(params![root_id, file_identity, normalized], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        drop(identity_statement);
        if let [(id, track_id, old_path)] = identity_matches.as_slice()
            && !Path::new(old_path).try_exists()?
        {
            connection.execute(
                "UPDATE track_files SET canonical_path = ?1, normalized_path_key = ?2, filename = ?3, extension = ?4, size_bytes = ?5, modified_at_ms = ?6, codec = ?7, duration_ms = ?8, bitrate_kbps = ?9, sample_rate_hz = ?10, bit_depth = ?11, channels = ?12, availability_status = ?13, last_scanned_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE id = ?14",
                params![canonical_path, normalized, filename, extension, size_bytes, modified_at_ms, extension, audio.duration_ms, audio.bitrate_kbps, audio.sample_rate_hz, audio.bit_depth, audio.channels, availability_status, id],
            )?;
            connection.execute(
                "UPDATE tracks SET preferred_title = ?1, preferred_artist = ?2, updated_at = CURRENT_TIMESTAMP WHERE id = ?3",
                params![title, artist, track_id],
            )?;
            self.write_metadata(&connection, id, title, artist, audio)?;
            return Ok(UpsertOutcome::Moved);
        }

        let track_id = Uuid::new_v4().to_string();
        let file_id = Uuid::new_v4().to_string();
        connection.execute(
            "INSERT INTO tracks(id, preferred_title, preferred_artist) VALUES (?1, ?2, ?3)",
            params![track_id, title, artist],
        )?;
        connection.execute(
            "INSERT INTO track_files(id, track_id, library_root_id, canonical_path, normalized_path_key, filename, extension, size_bytes, modified_at_ms, file_identity, codec, duration_ms, bitrate_kbps, sample_rate_hz, bit_depth, channels, availability_status, metadata_reader_version) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, 1)",
            params![file_id, track_id, root_id, canonical_path, normalized, filename, extension, size_bytes, modified_at_ms, file_identity, extension, audio.duration_ms, audio.bitrate_kbps, audio.sample_rate_hz, audio.bit_depth, audio.channels, availability_status],
        )?;
        self.write_metadata(&connection, &file_id, title, artist, audio)?;
        Ok(UpsertOutcome::Added)
    }

    pub fn reconcile_file_registration(
        &self,
        root_id: &str,
        canonical_path: &str,
        size_bytes: u64,
        modified_at_ms: i64,
        file_identity: &str,
    ) -> Result<bool, AppError> {
        let connection = self.connection()?;
        let normalized = canonical_path.to_lowercase();
        let signature: Option<StoredFileSignature> = connection
            .query_row(
                "SELECT library_root_id, size_bytes, modified_at_ms, file_identity, metadata_reader_version, availability_status FROM track_files WHERE normalized_path_key = ?1",
                [&normalized],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
            )
            .optional()?;
        match signature {
            Some((stored_root, size, modified, identity, 1, status))
                if size == size_bytes
                    && modified == modified_at_ms
                    && identity.as_deref() == Some(file_identity)
                    && status != "missing" =>
            {
                if stored_root.as_deref() != Some(root_id) {
                    connection.execute(
                        "UPDATE track_files SET library_root_id = ?1, updated_at = CURRENT_TIMESTAMP WHERE normalized_path_key = ?2",
                        params![root_id, normalized],
                    )?;
                }
                Ok(false)
            }
            _ => Ok(true),
        }
    }

    pub fn mark_missing_files(
        &self,
        root_id: &str,
    ) -> Result<(u64, Vec<(String, String)>), AppError> {
        let files = {
            let connection = self.connection()?;
            let mut statement = connection.prepare(
                "SELECT id, canonical_path FROM track_files WHERE library_root_id = ?1 AND availability_status <> 'missing'",
            )?;
            statement
                .query_map([root_id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<Result<Vec<_>, _>>()?
        };
        let mut missing = Vec::new();
        let mut errors = Vec::new();
        for (id, path) in files {
            match Path::new(&path).try_exists() {
                Ok(false) => missing.push((id, path)),
                Ok(true) => {}
                Err(error) => errors.push((path, error.to_string())),
            }
        }
        if missing.is_empty() {
            return Ok((0, errors));
        }
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let mut marked = 0u64;
        for (id, path) in missing {
            if !Path::new(&path).try_exists()? {
                marked += transaction.execute(
                    "UPDATE track_files SET availability_status = 'missing', updated_at = CURRENT_TIMESTAMP WHERE id = ?1 AND availability_status <> 'missing'",
                    [id],
                )? as u64;
            }
        }
        transaction.commit()?;
        Ok((marked, errors))
    }

    pub(crate) fn write_metadata(
        &self,
        connection: &Connection,
        file_id: &str,
        title: &str,
        artist: &str,
        audio: &AudioMetadata,
    ) -> Result<(), AppError> {
        connection.execute(
            "INSERT INTO track_metadata(track_file_id, title, artist, album, album_artist, track_number, disc_number, original_genre_tag, year, composer, comment, bpm, musical_key, camelot, metadata_fingerprint) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15) ON CONFLICT(track_file_id) DO UPDATE SET title=excluded.title, artist=excluded.artist, album=excluded.album, album_artist=excluded.album_artist, track_number=excluded.track_number, disc_number=excluded.disc_number, original_genre_tag=excluded.original_genre_tag, year=excluded.year, composer=excluded.composer, comment=excluded.comment, bpm=excluded.bpm, musical_key=excluded.musical_key, camelot=excluded.camelot, metadata_fingerprint=excluded.metadata_fingerprint, read_at=CURRENT_TIMESTAMP, updated_at=CURRENT_TIMESTAMP",
            params![file_id, title, artist, audio.album, audio.album_artist, audio.track_number, audio.disc_number, audio.original_genre_tag, audio.year, audio.composer, audio.comment, audio.bpm, audio.musical_key, audio.camelot, audio.fingerprint()],
        )?;
        Ok(())
    }

    pub fn list_tracks(
        &self,
        limit: u32,
        offset: u32,
        search: &str,
    ) -> Result<Vec<TrackRow>, AppError> {
        let connection = self.connection()?;
        let pattern = format!("%{}%", search.replace('%', "\\%").replace('_', "\\_"));
        let mut statement = connection.prepare(
                "SELECT t.id, coalesce(m.title, t.preferred_title, f.filename), coalesce(m.artist, t.preferred_artist, ''), coalesce(m.album, ''), t.confirmed_genre, t.confirmed_subgenre, coalesce(m.bpm, a.bpm), coalesce(m.musical_key, a.musical_key), coalesce(m.camelot, a.camelot), coalesce(f.duration_ms, 0), upper(f.extension), f.bitrate_kbps, f.canonical_path, t.rating, t.created_at, f.availability_status FROM tracks t JOIN track_files f ON f.track_id = t.id LEFT JOIN track_metadata m ON m.track_file_id = f.id LEFT JOIN track_audio_analysis a ON a.track_file_id = f.id WHERE (?1 = '' OR m.title LIKE ?2 ESCAPE '\\' OR m.artist LIKE ?2 ESCAPE '\\' OR f.filename LIKE ?2 ESCAPE '\\') ORDER BY t.created_at DESC LIMIT ?3 OFFSET ?4",
        )?;

        let rows =
            statement.query_map(params![search, pattern, limit, offset], track_row_from_sql)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
    }

    #[allow(clippy::too_many_arguments)] // IPC query fields stay explicit and independently validated.
    pub fn query_tracks(
        &self,
        limit: u32,
        offset: u32,
        search: &str,
        genre: Option<&str>,
        only_incoming: bool,
        min_bpm: Option<f64>,
        max_bpm: Option<f64>,
        min_rating: Option<u8>,
        camelot: Option<&str>,
        format_filter: Option<&str>,
        file_status: &str,
        sort_by: &str,
        descending: bool,
    ) -> Result<TrackPage, AppError> {
        let limit = limit.clamp(1, 200);
        for bpm in [min_bpm, max_bpm].into_iter().flatten() {
            if !bpm.is_finite() || !(0.0..=400.0).contains(&bpm) {
                return Err(AppError::Task(
                    "BPM filter must be between 0 and 400".into(),
                ));
            }
        }
        if matches!((min_bpm, max_bpm), (Some(minimum), Some(maximum)) if minimum > maximum) {
            return Err(AppError::Task("minimum BPM filter exceeds maximum".into()));
        }
        if min_rating.is_some_and(|rating| !(1..=5).contains(&rating)) {
            return Err(AppError::Task(
                "minimum rating filter must be between 1 and 5".into(),
            ));
        }
        if camelot.is_some_and(|value| !is_valid_camelot(value)) {
            return Err(AppError::Task("invalid Camelot filter".into()));
        }
        if format_filter.is_some_and(|value| !matches!(value, "mp3" | "flac")) {
            return Err(AppError::Task("invalid audio format filter".into()));
        }
        if !matches!(file_status, "all" | "available" | "problems" | "missing") {
            return Err(AppError::Task("invalid file status filter".into()));
        }
        let connection = self.connection()?;
        let fts_query = build_fts_query(search);
        let has_search = !fts_query.is_empty();
        let from_and_where = " FROM tracks t JOIN track_files f ON f.id = (SELECT preferred.id FROM track_files preferred WHERE preferred.track_id = t.id ORDER BY CASE preferred.availability_status WHEN 'available' THEN 0 WHEN 'corrupted' THEN 1 WHEN 'unreadable' THEN 2 ELSE 3 END, CASE lower(preferred.extension) WHEN 'flac' THEN 0 ELSE 1 END, preferred.updated_at DESC, preferred.id LIMIT 1) LEFT JOIN track_metadata m ON m.track_file_id = f.id LEFT JOIN track_audio_analysis a ON a.track_file_id = f.id WHERE (?1 = 0 OR t.id IN (SELECT track_id FROM track_search WHERE track_search MATCH ?2)) AND (?3 IS NULL OR t.confirmed_genre = ?3) AND (?4 = 0 OR t.confirmed_genre IS NULL) AND (?5 IS NULL OR coalesce(m.bpm, a.bpm) >= ?5) AND (?6 IS NULL OR coalesce(m.bpm, a.bpm) <= ?6) AND (?7 IS NULL OR t.rating >= ?7) AND (?8 IS NULL OR coalesce(m.camelot, a.camelot) = ?8) AND (?9 IS NULL OR lower(f.extension) = ?9) AND (?10 = 'all' OR (?10 = 'available' AND f.availability_status = 'available') OR (?10 = 'problems' AND f.availability_status IN ('corrupted', 'unreadable')) OR (?10 = 'missing' AND f.availability_status = 'missing'))";
        let total: u64 = connection.query_row(
            &format!("SELECT count(*){from_and_where}"),
            params![
                has_search,
                fts_query,
                genre,
                only_incoming,
                min_bpm,
                max_bpm,
                min_rating,
                camelot,
                format_filter,
                file_status
            ],
            |row| row.get(0),
        )?;
        let order_expression = match sort_by {
            "title" => "coalesce(m.title, t.preferred_title, f.filename) COLLATE NOCASE",
            "artist" => "coalesce(m.artist, t.preferred_artist, '') COLLATE NOCASE",
            "bpm" => "coalesce(m.bpm, a.bpm)",
            _ => "t.created_at",
        };
        let direction = if descending { "DESC" } else { "ASC" };
        let sql = format!(
            "SELECT t.id, coalesce(m.title, t.preferred_title, f.filename), coalesce(m.artist, t.preferred_artist, ''), coalesce(m.album, ''), t.confirmed_genre, t.confirmed_subgenre, coalesce(m.bpm, a.bpm), coalesce(m.musical_key, a.musical_key), coalesce(m.camelot, a.camelot), coalesce(f.duration_ms, 0), upper(f.extension), f.bitrate_kbps, f.canonical_path, t.rating, t.created_at, f.availability_status{from_and_where} ORDER BY {order_expression} {direction}, t.id ASC LIMIT ?11 OFFSET ?12"
        );
        let mut statement = connection.prepare(&sql)?;
        let rows = statement.query_map(
            params![
                has_search,
                fts_query,
                genre,
                only_incoming,
                min_bpm,
                max_bpm,
                min_rating,
                camelot,
                format_filter,
                file_status,
                limit,
                offset
            ],
            track_row_from_sql,
        )?;
        let items = rows.collect::<Result<Vec<_>, _>>()?;
        Ok(TrackPage {
            items,
            total,
            limit,
            offset,
        })
    }

    pub fn locate_track_page(
        &self,
        track_id: &str,
        page_size: u32,
    ) -> Result<Option<u32>, AppError> {
        let page_size = page_size.clamp(1, 200) as u64;
        let connection = self.connection()?;
        let position: Option<u64> = connection
            .query_row(
                "WITH ordered AS (SELECT id, row_number() OVER (ORDER BY created_at DESC, id ASC) - 1 AS position FROM tracks) SELECT position FROM ordered WHERE id = ?1",
                [track_id],
                |row| row.get(0),
            )
            .optional()?;
        Ok(position.map(|value| ((value / page_size) * page_size) as u32))
    }

    pub fn set_track_rating(
        &self,
        track_id: &str,
        expected_rating: u8,
        next_rating: u8,
    ) -> Result<(), AppError> {
        if expected_rating > 5 || next_rating > 5 {
            return Err(AppError::Metadata("rating must be between 0 and 5".into()));
        }
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let current_rating = transaction
            .query_row(
                "SELECT rating FROM tracks WHERE id = ?1",
                [track_id],
                |row| row.get::<_, u8>(0),
            )
            .optional()?
            .ok_or_else(|| AppError::Metadata("track not found".into()))?;
        if current_rating != expected_rating {
            return Err(AppError::Metadata(
                "rating changed; refresh the library and try again".into(),
            ));
        }
        if current_rating == next_rating {
            return Ok(());
        }
        let changed = transaction.execute(
            "UPDATE tracks SET rating = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2 AND rating = ?3",
            params![next_rating, track_id, expected_rating],
        )?;
        if changed != 1 {
            return Err(AppError::Metadata(
                "rating changed; refresh the library and try again".into(),
            ));
        }
        transaction.execute(
            "INSERT INTO track_rating_history(id, track_id, before_rating, after_rating, source) VALUES (?1, ?2, ?3, ?4, 'manual')",
            params![Uuid::new_v4().to_string(), track_id, current_rating, next_rating],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn bulk_set_track_rating(
        &self,
        track_ids: &[String],
        target_rating: u8,
    ) -> Result<BulkRatingReport, AppError> {
        if track_ids.is_empty() || track_ids.len() > 1000 {
            return Err(AppError::Metadata(
                "select between 1 and 1000 tracks".into(),
            ));
        }
        if target_rating > 5 {
            return Err(AppError::Metadata("rating must be between 0 and 5".into()));
        }
        let mut seen = HashSet::new();
        let unique_ids = track_ids
            .iter()
            .filter(|track_id| seen.insert((*track_id).clone()))
            .collect::<Vec<_>>();
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let mut items = Vec::with_capacity(unique_ids.len());
        for track_id in unique_ids {
            let current = transaction
                .query_row(
                    "SELECT rating, coalesce(preferred_title, id) FROM tracks WHERE id = ?1",
                    [track_id],
                    |row| Ok((row.get::<_, u8>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()?;
            let Some((before_rating, title)) = current else {
                items.push(BulkRatingReportItem {
                    track_id: track_id.clone(),
                    title: track_id.clone(),
                    status: "failed".into(),
                    reason: "track not found".into(),
                });
                continue;
            };
            if before_rating == target_rating {
                items.push(BulkRatingReportItem {
                    track_id: track_id.clone(),
                    title,
                    status: "unchanged".into(),
                    reason: "rating already has the requested value".into(),
                });
                continue;
            }
            transaction.execute(
                "UPDATE tracks SET rating = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
                params![target_rating, track_id],
            )?;
            transaction.execute(
                "INSERT INTO track_rating_history(id, track_id, before_rating, after_rating, source) VALUES (?1, ?2, ?3, ?4, 'bulk')",
                params![Uuid::new_v4().to_string(), track_id, before_rating, target_rating],
            )?;
            items.push(BulkRatingReportItem {
                track_id: track_id.clone(),
                title,
                status: "saved".into(),
                reason: "rating saved and recorded in history".into(),
            });
        }
        transaction.commit()?;
        Ok(BulkRatingReport {
            target_rating,
            total_tracks: items.len() as u64,
            saved_tracks: items.iter().filter(|item| item.status == "saved").count() as u64,
            unchanged_tracks: items
                .iter()
                .filter(|item| item.status == "unchanged")
                .count() as u64,
            failed_tracks: items.iter().filter(|item| item.status == "failed").count() as u64,
            items,
        })
    }

    pub fn playback_path(&self, track_id: &str) -> Result<std::path::PathBuf, AppError> {
        let connection = self.connection()?;
        let stored_path: Option<String> = connection
            .query_row(
                "SELECT canonical_path FROM track_files WHERE track_id = ?1 AND availability_status = 'available' AND lower(extension) IN ('mp3', 'flac') ORDER BY CASE lower(extension) WHEN 'flac' THEN 0 ELSE 1 END, created_at LIMIT 1",
                [track_id],
                |row| row.get(0),
            )
            .optional()?;
        let stored_path =
            stored_path.ok_or_else(|| AppError::TrackUnavailable(track_id.to_owned()))?;
        let path = std::path::PathBuf::from(&stored_path)
            .canonicalize()
            .map_err(|_| AppError::TrackUnavailable(format!("{track_id}: file is missing")))?;
        if !path.is_file()
            || !matches!(
                path.extension()
                    .and_then(|value| value.to_str())
                    .map(str::to_ascii_lowercase)
                    .as_deref(),
                Some("mp3" | "flac")
            )
        {
            return Err(AppError::TrackUnavailable(track_id.to_owned()));
        }
        Ok(path)
    }

    pub fn list_playlists(&self) -> Result<Vec<PlaylistDetail>, AppError> {
        let connection = self.connection()?;
        let mut playlist_statement =
            connection.prepare("SELECT id, name, kind FROM playlists ORDER BY created_at, id")?;
        let playlists = playlist_statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        drop(playlist_statement);

        let mut result = Vec::with_capacity(playlists.len());
        for (playlist_id, name, kind) in playlists {
            let smart_rule = if kind == "smart" {
                Some(load_smart_playlist_rule(&connection, &playlist_id)?)
            } else {
                None
            };
            let items = if let Some(rule) = &smart_rule {
                smart_playlist_items(&connection, &playlist_id, rule)?
            } else {
                manual_playlist_items(&connection, &playlist_id)?
            };
            result.push(PlaylistDetail {
                id: playlist_id,
                name,
                kind,
                smart_rule,
                items,
            });
        }
        Ok(result)
    }

    pub fn create_playlist(&self, requested_name: &str) -> Result<String, AppError> {
        let name = validate_playlist_name(requested_name)?;
        let connection = self.connection()?;
        let id = Uuid::new_v4().to_string();
        connection.execute(
            "INSERT INTO playlists(id, name) VALUES (?1, ?2)",
            params![id, name],
        )?;
        Ok(id)
    }

    pub fn create_smart_playlist(
        &self,
        requested_name: &str,
        rule: SmartPlaylistRule,
    ) -> Result<String, AppError> {
        let name = validate_playlist_name(requested_name)?;
        validate_smart_playlist_rule(&rule)?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let id = Uuid::new_v4().to_string();
        transaction.execute(
            "INSERT INTO playlists(id, name, kind) VALUES (?1, ?2, 'smart')",
            params![id, name],
        )?;
        transaction.execute(
            "INSERT INTO smart_playlist_rules(playlist_id, genre, subgenre, min_bpm, max_bpm, min_rating, camelot, limit_count, sort_by, descending) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![id, rule.genre, rule.subgenre, rule.min_bpm, rule.max_bpm, rule.min_rating, rule.camelot, rule.limit_count, rule.sort_by, rule.descending],
        )?;
        transaction.commit()?;
        Ok(id)
    }

    pub fn snapshot_smart_playlist(
        &self,
        smart_playlist_id: &str,
        requested_name: &str,
    ) -> Result<PlaylistSnapshotReport, AppError> {
        let name = validate_playlist_name(requested_name)?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let kind = transaction
            .query_row(
                "SELECT kind FROM playlists WHERE id = ?1",
                [smart_playlist_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| AppError::InvalidPlaylist("playlist not found".into()))?;
        if kind != "smart" {
            return Err(AppError::InvalidPlaylist(
                "only a smart playlist can be saved as a snapshot".into(),
            ));
        }
        let rule = load_smart_playlist_rule(&transaction, smart_playlist_id)?;
        let items = smart_playlist_items(&transaction, smart_playlist_id, &rule)?;
        let playlist_id = Uuid::new_v4().to_string();
        transaction.execute(
            "INSERT INTO playlists(id, name, kind) VALUES (?1, ?2, 'manual')",
            params![playlist_id, name],
        )?;
        for (position, item) in items.iter().enumerate() {
            transaction.execute(
                "INSERT INTO playlist_items(id, playlist_id, track_id, position) VALUES (?1, ?2, ?3, ?4)",
                params![Uuid::new_v4().to_string(), playlist_id, item.track.id, position as u32],
            )?;
        }
        transaction.commit()?;
        Ok(PlaylistSnapshotReport {
            playlist_id,
            name: name.to_owned(),
            item_count: items.len() as u64,
        })
    }

    pub fn update_smart_playlist_rule(
        &self,
        smart_playlist_id: &str,
        expected_rule: SmartPlaylistRule,
        next_rule: SmartPlaylistRule,
    ) -> Result<(), AppError> {
        validate_smart_playlist_rule(&expected_rule)?;
        validate_smart_playlist_rule(&next_rule)?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let kind = transaction
            .query_row(
                "SELECT kind FROM playlists WHERE id = ?1",
                [smart_playlist_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| AppError::InvalidPlaylist("playlist not found".into()))?;
        if kind != "smart" {
            return Err(AppError::InvalidPlaylist(
                "only a smart playlist rule can be updated".into(),
            ));
        }
        let current_rule = load_smart_playlist_rule(&transaction, smart_playlist_id)?;
        if current_rule != expected_rule {
            return Err(AppError::InvalidPlaylist(
                "smart playlist rule changed; reopen the editor".into(),
            ));
        }
        transaction.execute(
            "UPDATE smart_playlist_rules SET genre = ?1, subgenre = ?2, min_bpm = ?3, max_bpm = ?4, min_rating = ?5, camelot = ?6, limit_count = ?7, sort_by = ?8, descending = ?9 WHERE playlist_id = ?10",
            params![next_rule.genre, next_rule.subgenre, next_rule.min_bpm, next_rule.max_bpm, next_rule.min_rating, next_rule.camelot, next_rule.limit_count, next_rule.sort_by, next_rule.descending, smart_playlist_id],
        )?;
        transaction.execute(
            "UPDATE playlists SET updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
            [smart_playlist_id],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn rename_playlist(&self, playlist_id: &str, requested_name: &str) -> Result<(), AppError> {
        let name = validate_playlist_name(requested_name)?;
        let connection = self.connection()?;
        let changed = connection.execute(
            "UPDATE playlists SET name = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            params![name, playlist_id],
        )?;
        if changed == 0 {
            return Err(AppError::InvalidPlaylist("playlist not found".into()));
        }
        Ok(())
    }

    pub fn preview_playlist_removal(
        &self,
        playlist_id: &str,
    ) -> Result<PlaylistRemovalPreview, AppError> {
        let connection = self.connection()?;
        let name = connection
            .query_row(
                "SELECT name FROM playlists WHERE id = ?1",
                [playlist_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| AppError::InvalidPlaylist("playlist not found".into()))?;
        let items = playlist_item_state(&connection, playlist_id)?;
        Ok(PlaylistRemovalPreview {
            playlist_id: playlist_id.into(),
            name: name.clone(),
            item_count: items.len() as u64,
            expected_state: playlist_removal_state(&name, &items),
        })
    }

    pub fn remove_playlist(
        &self,
        playlist_id: &str,
        expected_state: &str,
    ) -> Result<PlaylistRemovalReport, AppError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let name = transaction
            .query_row(
                "SELECT name FROM playlists WHERE id = ?1",
                [playlist_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| AppError::InvalidPlaylist("playlist not found".into()))?;
        let items = playlist_item_state(&transaction, playlist_id)?;
        if playlist_removal_state(&name, &items) != expected_state {
            return Err(AppError::InvalidPlaylist(
                "playlist changed; create a new removal preview".into(),
            ));
        }
        let deleted = transaction.execute("DELETE FROM playlists WHERE id = ?1", [playlist_id])?;
        if deleted != 1 {
            return Err(AppError::InvalidPlaylist("playlist was not removed".into()));
        }
        transaction.commit()?;
        Ok(PlaylistRemovalReport {
            playlist_id: playlist_id.into(),
            name,
            removed_items: items.len() as u64,
        })
    }

    pub fn remove_playlist_item(&self, playlist_id: &str, item_id: &str) -> Result<(), AppError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let removed = transaction.execute(
            "DELETE FROM playlist_items WHERE id = ?1 AND playlist_id = ?2",
            params![item_id, playlist_id],
        )?;
        if removed == 0 {
            return Err(AppError::InvalidPlaylist("playlist item not found".into()));
        }
        let ids = playlist_item_ids(&transaction, playlist_id)?;
        resequence(&transaction, playlist_id, &ids)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn place_track_in_playlist(
        &self,
        target_playlist_id: &str,
        target_position: u32,
        track_id: &str,
        item_id: Option<&str>,
        source_playlist_id: Option<&str>,
    ) -> Result<(), AppError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let target_exists: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM playlists WHERE id = ?1 AND kind = 'manual')",
            [target_playlist_id],
            |row| row.get(0),
        )?;
        let track_exists: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM tracks WHERE id = ?1)",
            [track_id],
            |row| row.get(0),
        )?;
        if !target_exists || !track_exists {
            return Err(AppError::InvalidPlaylist(
                "playlist or track not found".into(),
            ));
        }

        let mut target_items = playlist_items(&transaction, target_playlist_id)?;
        let same_playlist = source_playlist_id == Some(target_playlist_id);
        let moved_item_id = if let Some(item_id) = item_id {
            let source_id = source_playlist_id
                .ok_or_else(|| AppError::InvalidPlaylist("source playlist is required".into()))?;
            let valid_source: bool = transaction.query_row(
                "SELECT EXISTS(SELECT 1 FROM playlist_items WHERE id = ?1 AND playlist_id = ?2 AND track_id = ?3)",
                params![item_id, source_id, track_id],
                |row| row.get(0),
            )?;
            if !valid_source {
                return Err(AppError::InvalidPlaylist("source item not found".into()));
            }
            item_id.to_owned()
        } else {
            Uuid::new_v4().to_string()
        };

        if target_items
            .iter()
            .any(|(id, existing_track)| existing_track == track_id && id != &moved_item_id)
        {
            return Ok(());
        }

        if same_playlist {
            target_items.retain(|(id, _)| id != &moved_item_id);
        } else if let (Some(source_id), Some(item_id)) = (source_playlist_id, item_id) {
            transaction.execute(
                "UPDATE playlist_items SET position = position + 1000000 WHERE playlist_id = ?1",
                [source_id],
            )?;
            transaction.execute("DELETE FROM playlist_items WHERE id = ?1", [item_id])?;
            let source_ids = playlist_item_ids(&transaction, source_id)?;
            resequence_shifted(&transaction, &source_ids)?;
        }

        let position = (target_position as usize).min(target_items.len());
        target_items.insert(position, (moved_item_id.clone(), track_id.to_owned()));
        transaction.execute(
            "UPDATE playlist_items SET position = position + 1000000 WHERE playlist_id = ?1",
            [target_playlist_id],
        )?;
        for (position, (id, item_track_id)) in target_items.iter().enumerate() {
            let changed = transaction.execute(
                "UPDATE playlist_items SET position = ?1 WHERE id = ?2",
                params![position as u32, id],
            )?;
            if changed == 0 {
                transaction.execute(
                    "INSERT INTO playlist_items(id, playlist_id, track_id, position) VALUES (?1, ?2, ?3, ?4)",
                    params![id, target_playlist_id, item_track_id, position as u32],
                )?;
            }
        }
        transaction.execute(
            "UPDATE playlists SET updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
            [target_playlist_id],
        )?;
        transaction.commit()?;
        Ok(())
    }
}

fn manual_playlist_items(
    connection: &Connection,
    playlist_id: &str,
) -> Result<Vec<PlaylistItemRow>, AppError> {
    let mut statement = connection.prepare(
        "SELECT pi.id, pi.position, t.id, coalesce(m.title, t.preferred_title, f.filename), coalesce(m.artist, t.preferred_artist, ''), coalesce(m.album, ''), t.confirmed_genre, t.confirmed_subgenre, coalesce(m.bpm, a.bpm), coalesce(m.musical_key, a.musical_key), coalesce(m.camelot, a.camelot), coalesce(f.duration_ms, 0), upper(f.extension), f.bitrate_kbps, f.canonical_path, t.rating, t.created_at, f.availability_status FROM playlist_items pi JOIN tracks t ON t.id = pi.track_id JOIN track_files f ON f.id = (SELECT preferred.id FROM track_files preferred WHERE preferred.track_id = t.id ORDER BY CASE preferred.availability_status WHEN 'available' THEN 0 WHEN 'corrupted' THEN 1 WHEN 'unreadable' THEN 2 ELSE 3 END, CASE lower(preferred.extension) WHEN 'flac' THEN 0 ELSE 1 END, preferred.updated_at DESC, preferred.id LIMIT 1) LEFT JOIN track_metadata m ON m.track_file_id = f.id LEFT JOIN track_audio_analysis a ON a.track_file_id = f.id WHERE pi.playlist_id = ?1 ORDER BY pi.position",
    )?;
    Ok(statement
        .query_map([playlist_id], |row| {
            let track_id: String = row.get(2)?;
            Ok(PlaylistItemRow {
                id: row.get(0)?,
                position: row.get(1)?,
                track: TrackRow {
                    id: track_id.clone(),
                    title: row.get(3)?,
                    artist: row.get(4)?,
                    album: row.get(5)?,
                    genre: row.get(6)?,
                    subgenre: row.get(7)?,
                    bpm: row.get(8)?,
                    musical_key: row.get(9)?,
                    camelot: row.get(10)?,
                    duration_seconds: row.get::<_, u64>(11)? / 1000,
                    format: row.get(12)?,
                    bitrate_kbps: row.get(13)?,
                    path: row.get(14)?,
                    rating: row.get(15)?,
                    added_at: row.get(16)?,
                    analysis_state: analysis_state(&row.get::<_, String>(17)?),
                    cover_color: color_for_id(&track_id),
                },
            })
        })?
        .collect::<Result<Vec<_>, _>>()?)
}

fn load_smart_playlist_rule(
    connection: &Connection,
    playlist_id: &str,
) -> Result<SmartPlaylistRule, AppError> {
    connection
        .query_row(
            "SELECT genre, subgenre, min_bpm, max_bpm, min_rating, camelot, limit_count, sort_by, descending FROM smart_playlist_rules WHERE playlist_id = ?1",
            [playlist_id],
            |row| {
                Ok(SmartPlaylistRule {
                    genre: row.get(0)?,
                    subgenre: row.get(1)?,
                    min_bpm: row.get(2)?,
                    max_bpm: row.get(3)?,
                    min_rating: row.get(4)?,
                    camelot: row.get(5)?,
                    limit_count: row.get(6)?,
                    sort_by: row.get(7)?,
                    descending: row.get(8)?,
                })
            },
        )
        .map_err(AppError::from)
}

fn smart_playlist_items(
    connection: &Connection,
    playlist_id: &str,
    rule: &SmartPlaylistRule,
) -> Result<Vec<PlaylistItemRow>, AppError> {
    validate_smart_playlist_rule(rule)?;
    let order = match rule.sort_by.as_str() {
        "title" => "coalesce(m.title, t.preferred_title, f.filename) COLLATE NOCASE",
        "bpm" => "coalesce(m.bpm, a.bpm)",
        _ => "t.created_at",
    };
    let direction = if rule.descending { "DESC" } else { "ASC" };
    let sql = format!(
        "SELECT t.id, coalesce(m.title, t.preferred_title, f.filename), coalesce(m.artist, t.preferred_artist, ''), coalesce(m.album, ''), t.confirmed_genre, t.confirmed_subgenre, coalesce(m.bpm, a.bpm), coalesce(m.musical_key, a.musical_key), coalesce(m.camelot, a.camelot), coalesce(f.duration_ms, 0), upper(f.extension), f.bitrate_kbps, f.canonical_path, t.rating, t.created_at, f.availability_status FROM tracks t JOIN track_files f ON f.id = (SELECT preferred.id FROM track_files preferred WHERE preferred.track_id = t.id ORDER BY CASE preferred.availability_status WHEN 'available' THEN 0 WHEN 'corrupted' THEN 1 WHEN 'unreadable' THEN 2 ELSE 3 END, CASE lower(preferred.extension) WHEN 'flac' THEN 0 ELSE 1 END, preferred.updated_at DESC, preferred.id LIMIT 1) LEFT JOIN track_metadata m ON m.track_file_id = f.id LEFT JOIN track_audio_analysis a ON a.track_file_id = f.id WHERE f.availability_status <> 'missing' AND (?1 IS NULL OR t.confirmed_genre = ?1) AND (?2 IS NULL OR t.confirmed_subgenre = ?2) AND (?3 IS NULL OR coalesce(m.bpm, a.bpm) >= ?3) AND (?4 IS NULL OR coalesce(m.bpm, a.bpm) <= ?4) AND (?5 IS NULL OR t.rating >= ?5) AND (?6 IS NULL OR coalesce(m.camelot, a.camelot) = ?6) ORDER BY {order} {direction}, t.id ASC LIMIT ?7"
    );
    let mut statement = connection.prepare(&sql)?;
    let tracks = statement
        .query_map(
            params![
                rule.genre,
                rule.subgenre,
                rule.min_bpm,
                rule.max_bpm,
                rule.min_rating,
                rule.camelot,
                rule.limit_count
            ],
            track_row_from_sql,
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(tracks
        .into_iter()
        .enumerate()
        .map(|(position, track)| PlaylistItemRow {
            id: format!("smart-{playlist_id}-{}", track.id),
            position: position as u32,
            track,
        })
        .collect())
}

fn validate_smart_playlist_rule(rule: &SmartPlaylistRule) -> Result<(), AppError> {
    if rule
        .genre
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
    {
        return Err(AppError::InvalidPlaylist("genre cannot be empty".into()));
    }
    if rule
        .subgenre
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
    {
        return Err(AppError::InvalidPlaylist("subgenre cannot be empty".into()));
    }
    match (rule.genre.as_deref(), rule.subgenre.as_deref()) {
        (Some(genre), subgenre) => genre_assistant::validate(genre, subgenre)
            .map_err(|error| AppError::InvalidPlaylist(error.to_string()))?,
        (None, Some(_)) => {
            return Err(AppError::InvalidPlaylist(
                "subgenre requires a genre".into(),
            ));
        }
        (None, None) => {}
    }
    if rule.genre.is_none()
        && rule.min_bpm.is_none()
        && rule.max_bpm.is_none()
        && rule.min_rating.is_none()
        && rule.camelot.is_none()
    {
        return Err(AppError::InvalidPlaylist(
            "at least one smart playlist filter is required".into(),
        ));
    }
    for bpm in [rule.min_bpm, rule.max_bpm].into_iter().flatten() {
        if !bpm.is_finite() || !(20.0..=300.0).contains(&bpm) {
            return Err(AppError::InvalidPlaylist(
                "BPM must be between 20 and 300".into(),
            ));
        }
    }
    if matches!((rule.min_bpm, rule.max_bpm), (Some(minimum), Some(maximum)) if minimum > maximum) {
        return Err(AppError::InvalidPlaylist(
            "minimum BPM exceeds maximum BPM".into(),
        ));
    }
    if rule
        .min_rating
        .is_some_and(|rating| !(1..=5).contains(&rating))
    {
        return Err(AppError::InvalidPlaylist(
            "minimum rating must be between 1 and 5".into(),
        ));
    }
    if rule
        .camelot
        .as_deref()
        .is_some_and(|value| !is_valid_camelot(value))
    {
        return Err(AppError::InvalidPlaylist(
            "Camelot key must be between 1A and 12B".into(),
        ));
    }
    if !(1..=1000).contains(&rule.limit_count)
        || !matches!(rule.sort_by.as_str(), "added" | "title" | "bpm")
    {
        return Err(AppError::InvalidPlaylist(
            "invalid smart playlist rule".into(),
        ));
    }
    Ok(())
}

fn is_valid_camelot(value: &str) -> bool {
    let (number, suffix) = if let Some(number) = value.strip_suffix('A') {
        (number, 'A')
    } else if let Some(number) = value.strip_suffix('B') {
        (number, 'B')
    } else {
        return false;
    };
    number
        .parse::<u8>()
        .is_ok_and(|parsed| (1..=12).contains(&parsed) && value == format!("{parsed}{suffix}"))
}

fn validate_playlist_name(value: &str) -> Result<&str, AppError> {
    let name = value.trim();
    if name.is_empty() || name.chars().count() > 100 || name.chars().any(char::is_control) {
        return Err(AppError::InvalidPlaylist(
            "name must contain 1 to 100 visible characters".into(),
        ));
    }
    Ok(name)
}

fn playlist_items(
    transaction: &rusqlite::Transaction<'_>,
    playlist_id: &str,
) -> Result<Vec<(String, String)>, AppError> {
    let mut statement = transaction.prepare(
        "SELECT id, track_id FROM playlist_items WHERE playlist_id = ?1 ORDER BY position",
    )?;
    Ok(statement
        .query_map([playlist_id], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()?)
}

fn playlist_item_ids(
    transaction: &rusqlite::Transaction<'_>,
    playlist_id: &str,
) -> Result<Vec<String>, AppError> {
    Ok(playlist_items(transaction, playlist_id)?
        .into_iter()
        .map(|(id, _)| id)
        .collect())
}

fn playlist_item_state(
    connection: &Connection,
    playlist_id: &str,
) -> Result<Vec<(String, u32)>, AppError> {
    let mut statement = connection.prepare(
        "SELECT id, position FROM playlist_items WHERE playlist_id = ?1 ORDER BY position, id",
    )?;
    Ok(statement
        .query_map([playlist_id], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()?)
}

fn playlist_removal_state(name: &str, items: &[(String, u32)]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    for (id, position) in items {
        hasher.update([0]);
        hasher.update(id.as_bytes());
        hasher.update(position.to_le_bytes());
    }
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn resequence(
    transaction: &rusqlite::Transaction<'_>,
    playlist_id: &str,
    ids: &[String],
) -> Result<(), AppError> {
    transaction.execute(
        "UPDATE playlist_items SET position = position + 1000000 WHERE playlist_id = ?1",
        [playlist_id],
    )?;
    resequence_shifted(transaction, ids)
}

fn resequence_shifted(
    transaction: &rusqlite::Transaction<'_>,
    ids: &[String],
) -> Result<(), AppError> {
    for (position, id) in ids.iter().enumerate() {
        transaction.execute(
            "UPDATE playlist_items SET position = ?1 WHERE id = ?2",
            params![position as u32, id],
        )?;
    }
    Ok(())
}

pub(crate) fn split_filename(filename: &str) -> (String, String) {
    let stem = Path::new(filename)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(filename);
    if let Some((artist, title)) = stem.split_once(" - ") {
        (artist.trim().to_string(), title.trim().to_string())
    } else {
        (String::new(), stem.to_string())
    }
}

fn track_row_from_sql(row: &rusqlite::Row<'_>) -> rusqlite::Result<TrackRow> {
    let format: String = row.get(10)?;
    let id: String = row.get(0)?;
    Ok(TrackRow {
        id: id.clone(),
        title: row.get(1)?,
        artist: row.get(2)?,
        album: row.get(3)?,
        genre: row.get(4)?,
        subgenre: row.get(5)?,
        bpm: row.get(6)?,
        musical_key: row.get(7)?,
        camelot: row.get(8)?,
        duration_seconds: row.get::<_, u64>(9)? / 1000,
        format,
        bitrate_kbps: row.get(11)?,
        path: row.get(12)?,
        rating: row.get(13)?,
        added_at: row.get(14)?,
        analysis_state: analysis_state(&row.get::<_, String>(15)?),
        cover_color: color_for_id(&id),
    })
}

fn build_fts_query(search: &str) -> String {
    search
        .split_whitespace()
        .filter_map(|part| {
            let token: String = part
                .chars()
                .filter(|character| character.is_alphanumeric())
                .collect();
            (!token.is_empty()).then(|| format!("\"{}\"*", token.replace('"', "\"\"")))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn color_for_id(id: &str) -> String {
    const COLORS: [&str; 6] = [
        "#d5ff45", "#ff6f61", "#80d8ff", "#be8cff", "#ffd166", "#78e08f",
    ];
    let index = id.bytes().fold(0usize, |sum, value| sum + value as usize) % COLORS.len();
    COLORS[index].to_string()
}

fn analysis_state(availability_status: &str) -> String {
    match availability_status {
        "missing" => "missing",
        "corrupted" | "unreadable" => "needs-review",
        _ => "queued",
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_the_anonymized_v13_characterization_profile() {
        let state = AppState::in_memory().expect("database");
        state
            .connection()
            .expect("connection")
            .execute_batch(include_str!("../tests/fixtures/profile_v13_seed.sql"))
            .expect("v13 fixture");

        let connection = state.connection().expect("fixture connection");
        let version: u32 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("schema version");
        let integrity: String = connection
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .expect("integrity check");
        let foreign_key_errors: u64 = connection
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })
            .expect("foreign key check");
        drop(connection);

        assert_eq!(version, CURRENT_SCHEMA_VERSION);
        assert_eq!(integrity, "ok");
        assert_eq!(foreign_key_errors, 0);
        assert_eq!(state.summary().expect("summary").tracks, 3);

        let page = state
            .query_tracks(
                10,
                0,
                "Synthetic",
                None,
                false,
                None,
                None,
                None,
                None,
                None,
                "all",
                "title",
                false,
            )
            .expect("fixture query");
        assert_eq!(page.total, 2);
        assert_eq!(page.items[0].title, "Synthetic Break");
        assert_eq!(page.items[1].title, "Synthetic Pulse");

        let playlists = state.list_playlists().expect("fixture playlists");
        assert_eq!(playlists.len(), 2);
        assert_eq!(
            playlists
                .iter()
                .find(|playlist| playlist.id == "fixture-playlist-manual")
                .expect("manual playlist")
                .items
                .len(),
            2
        );
        assert_eq!(
            playlists
                .iter()
                .find(|playlist| playlist.id == "fixture-playlist-smart")
                .expect("smart playlist")
                .items
                .len(),
            1
        );
    }

    #[test]
    fn parses_artist_and_title_from_filename() {
        assert_eq!(
            split_filename("Artist - Track.flac"),
            ("Artist".into(), "Track".into())
        );
    }

    #[test]
    fn lists_saved_library_root_settings() {
        let state = AppState::in_memory().expect("database");
        let root_id = state.register_root("D:\\Music").expect("root");
        state
            .update_root_scan_settings(&root_id, false, &["archive".into(), "temp".into()])
            .expect("settings");

        let roots = state.list_library_roots().expect("library roots");
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].id, root_id);
        assert_eq!(roots[0].path, "D:\\Music");
        assert!(!roots[0].recursive);
        assert_eq!(roots[0].exclusions, vec!["archive", "temp"]);
        assert_eq!(roots[0].status, "active");
        assert!(roots[0].last_scanned_at.is_some());
    }

    #[test]
    fn disables_and_reenables_a_library_root_without_deleting_it() {
        let state = AppState::in_memory().expect("database");
        let path = std::env::current_dir()
            .expect("current directory")
            .to_string_lossy()
            .into_owned();
        let root_id = state.register_root(&path).expect("root");

        state
            .set_library_root_enabled(&root_id, "active", false)
            .expect("disable");
        let disabled = state.list_library_roots().expect("disabled root");
        assert_eq!(disabled.len(), 1);
        assert_eq!(disabled[0].status, "disabled");

        let stale = state.set_library_root_enabled(&root_id, "active", true);
        assert!(stale.is_err());
        state
            .set_library_root_enabled(&root_id, "disabled", true)
            .expect("enable");
        assert_eq!(
            state.list_library_roots().expect("enabled root")[0].status,
            "active"
        );
    }

    #[test]
    fn refresh_marks_only_missing_active_roots_unavailable() {
        let state = AppState::in_memory().expect("database");
        let missing_path = std::env::temp_dir()
            .join(format!("sorttrack-missing-root-{}", Uuid::new_v4()))
            .to_string_lossy()
            .into_owned();
        let active_id = state.register_root(&missing_path).expect("active root");
        let disabled_id = state
            .register_root(&format!("{missing_path}-disabled"))
            .expect("disabled root");
        state
            .set_library_root_enabled(&disabled_id, "active", false)
            .expect("disable");

        let roots = state.refresh_library_root_availability().expect("refresh");
        let active = roots
            .iter()
            .find(|root| root.id == active_id)
            .expect("active");
        let disabled = roots
            .iter()
            .find(|root| root.id == disabled_id)
            .expect("disabled");
        assert_eq!(active.status, "unavailable");
        assert_eq!(disabled.status, "disabled");
    }

    #[test]
    fn removes_only_a_disabled_root_and_reattaches_unchanged_files() {
        let state = AppState::in_memory().expect("database");
        let old_root = state.register_root("D:\\Music").expect("root");
        let file_path = "D:\\Music\\Artist - Track.flac";
        state
            .upsert_file(
                &old_root,
                file_path,
                "Artist - Track.flac",
                "flac",
                123,
                456,
                "volume:file",
                &AudioMetadata::default(),
                "available",
            )
            .expect("track");
        assert!(state.preview_library_root_removal(&old_root).is_err());
        state
            .set_library_root_enabled(&old_root, "active", false)
            .expect("disable");
        let preview = state
            .preview_library_root_removal(&old_root)
            .expect("preview");
        assert_eq!(preview.linked_files, 1);
        assert!(
            state
                .remove_library_root_registration(&old_root, 0)
                .is_err()
        );
        let report = state
            .remove_library_root_registration(&old_root, 1)
            .expect("remove registration");
        assert_eq!(report.detached_files, 1);
        assert!(state.list_library_roots().expect("roots").is_empty());
        assert_eq!(state.summary().expect("summary").tracks, 1);

        let new_root = state.register_root("D:\\Music").expect("new root");
        assert_ne!(new_root, old_root);
        assert!(
            !state
                .reconcile_file_registration(&new_root, file_path, 123, 456, "volume:file")
                .expect("reattach")
        );
        let attached_root: String = state
            .connection()
            .expect("connection")
            .query_row(
                "SELECT library_root_id FROM track_files WHERE canonical_path = ?1",
                [file_path],
                |row| row.get(0),
            )
            .expect("attached root");
        assert_eq!(attached_root, new_root);
        assert_eq!(state.summary().expect("summary").tracks, 1);
    }

    #[test]
    fn query_tracks_filters_before_pagination_and_reports_total() {
        let state = AppState::in_memory().expect("database");
        let root = state.register_root("D:\\Music").expect("root");
        for (index, filename) in [
            "Artist - Alpha.flac",
            "Artist - Beta.mp3",
            "Artist - Gamma.flac",
        ]
        .iter()
        .enumerate()
        {
            state
                .upsert_file(
                    &root,
                    &format!("D:\\Music\\{filename}"),
                    filename,
                    filename.rsplit('.').next().expect("extension"),
                    100 + index as u64,
                    index as i64,
                    &format!("identity-{index}"),
                    &AudioMetadata {
                        bpm: Some(120.0 + index as f64 * 10.0),
                        camelot: Some(if index == 1 { "4A" } else { "8A" }.into()),
                        ..AudioMetadata::default()
                    },
                    if index == 2 { "corrupted" } else { "available" },
                )
                .expect("track");
        }
        let beta_id = state.list_tracks(1, 0, "Beta").expect("beta").remove(0).id;
        state
            .connection()
            .expect("connection")
            .execute(
                "UPDATE tracks SET confirmed_genre = 'Techno', rating = 4 WHERE id = ?1",
                [&beta_id],
            )
            .expect("genre");

        let first = state
            .query_tracks(
                1, 0, "Artist", None, false, None, None, None, None, None, "all", "title", false,
            )
            .expect("first page");
        assert_eq!(first.total, 3);
        assert_eq!(first.items.len(), 1);
        assert_eq!(first.items[0].title, "Alpha");
        let second = state
            .query_tracks(
                1, 1, "Artist", None, false, None, None, None, None, None, "all", "title", false,
            )
            .expect("second page");
        assert_eq!(second.items[0].title, "Beta");
        let genre_page = state
            .query_tracks(
                100,
                0,
                "",
                Some("Techno"),
                false,
                None,
                None,
                None,
                None,
                None,
                "all",
                "added",
                true,
            )
            .expect("genre page");
        assert_eq!(genre_page.total, 1);
        assert_eq!(genre_page.items[0].id, beta_id);
        let incoming = state
            .query_tracks(
                100, 0, "", None, true, None, None, None, None, None, "all", "title", false,
            )
            .expect("incoming page");
        assert_eq!(incoming.total, 2);
        assert!(incoming.items.iter().all(|track| track.id != beta_id));
        let advanced = state
            .query_tracks(
                100,
                0,
                "",
                None,
                false,
                Some(125.0),
                Some(135.0),
                Some(4),
                Some("4A"),
                Some("mp3"),
                "available",
                "bpm",
                false,
            )
            .expect("advanced filters");
        assert_eq!(advanced.total, 1);
        assert_eq!(advanced.items[0].id, beta_id);
        let problems = state
            .query_tracks(
                100, 0, "", None, false, None, None, None, None, None, "problems", "title", false,
            )
            .expect("problem filter");
        assert_eq!(problems.total, 1);
        assert_eq!(problems.items[0].title, "Gamma");
        assert!(
            state
                .query_tracks(
                    100,
                    0,
                    "",
                    None,
                    false,
                    Some(150.0),
                    Some(120.0),
                    None,
                    None,
                    None,
                    "all",
                    "added",
                    true,
                )
                .is_err()
        );
    }

    #[test]
    fn locates_the_page_containing_a_track() {
        let state = AppState::in_memory().expect("database");
        let connection = state.connection().expect("connection");
        for index in 0..5 {
            connection
                .execute(
                    "INSERT INTO tracks(id, preferred_title, created_at) VALUES (?1, ?2, ?3)",
                    params![
                        format!("track-{index}"),
                        format!("Track {index}"),
                        format!("2026-01-01T00:00:0{index}Z")
                    ],
                )
                .expect("track");
        }
        drop(connection);
        assert_eq!(state.locate_track_page("track-4", 2).unwrap(), Some(0));
        assert_eq!(state.locate_track_page("track-2", 2).unwrap(), Some(2));
        assert_eq!(state.locate_track_page("track-0", 2).unwrap(), Some(4));
        assert_eq!(state.locate_track_page("missing", 2).unwrap(), None);
    }

    #[test]
    fn migration_and_repository_work_in_memory() {
        let state = AppState::in_memory().expect("database");
        let root = state.register_root("D:\\Music").expect("root");
        assert_eq!(
            state
                .upsert_file(
                    &root,
                    "D:\\Music\\A - B.flac",
                    "A - B.flac",
                    "flac",
                    42,
                    10,
                    "test-file-id",
                    &AudioMetadata::default(),
                    "corrupted"
                )
                .expect("insert"),
            UpsertOutcome::Added
        );
        assert_eq!(state.summary().expect("summary").tracks, 1);
        assert_eq!(state.list_tracks(10, 0, "B").expect("tracks").len(), 1);

        let connection = state.connection().expect("connection");
        let file_id: String = connection
            .query_row("SELECT id FROM track_files LIMIT 1", [], |row| row.get(0))
            .expect("file id");
        connection
            .execute(
                "INSERT INTO track_audio_analysis(track_file_id, bpm, bpm_score, bpm_algorithm_version, size_bytes, modified_at_ms) VALUES (?1, 128, 0.9, 'test', 42, 10)",
                [&file_id],
            )
            .expect("analysis cache");
        connection
            .execute(
                "INSERT INTO audio_feature_cache(track_file_id, extractor_version, features_json, size_bytes, modified_at_ms) VALUES (?1, 'test', '{\"values\":[1]}', 42, 10)",
                [&file_id],
            )
            .expect("feature cache");
        drop(connection);

        assert_eq!(
            state
                .upsert_file(
                    &root,
                    "D:\\Music\\A - B.flac",
                    "A - B.flac",
                    "flac",
                    84,
                    20,
                    "changed-file-id",
                    &AudioMetadata::default(),
                    "corrupted"
                )
                .expect("update"),
            UpsertOutcome::Updated
        );
        let cached: u64 = state
            .connection()
            .expect("connection")
            .query_row("SELECT count(*) FROM track_audio_analysis", [], |row| {
                row.get(0)
            })
            .expect("cache count");
        assert_eq!(cached, 0);
        let cached_features: u64 = state
            .connection()
            .expect("connection")
            .query_row("SELECT count(*) FROM audio_feature_cache", [], |row| {
                row.get(0)
            })
            .expect("feature cache count");
        assert_eq!(cached_features, 0);
    }

    #[test]
    fn upgrades_an_existing_version_one_database() {
        let database_path =
            std::env::temp_dir().join(format!("sorttrack-migration-{}.db", Uuid::new_v4()));
        let connection = Connection::open(&database_path).expect("version one database");
        connection
            .execute_batch(include_str!("../migrations/0001_initial.sql"))
            .expect("version one schema");
        drop(connection);

        let state = AppState::open(&database_path).expect("upgrade database");
        let connection = state.connection().expect("connection");
        let version: u32 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("schema version");
        assert_eq!(version, CURRENT_SCHEMA_VERSION);
        drop(connection);
        drop(state);
        std::fs::remove_file(database_path).expect("cleanup");
    }

    #[test]
    fn upgrades_version_eleven_smart_rules_without_data_loss() {
        let database_path =
            std::env::temp_dir().join(format!("sorttrack-smart-migration-{}.db", Uuid::new_v4()));
        let connection = Connection::open(&database_path).expect("version eleven database");
        for migration in [
            include_str!("../migrations/0001_initial.sql"),
            include_str!("../migrations/0002_metadata_reader.sql"),
            include_str!("../migrations/0003_copy_recovery.sql"),
            include_str!("../migrations/0004_audio_analysis.sql"),
            include_str!("../migrations/0005_genre_assistant.sql"),
            include_str!("../migrations/0006_genre_dataset.sql"),
            include_str!("../migrations/0007_genre_model.sql"),
            include_str!("../migrations/0008_file_relink.sql"),
            include_str!("../migrations/0009_track_search_index.sql"),
            include_str!("../migrations/0010_ui_preferences.sql"),
            include_str!("../migrations/0011_smart_playlists.sql"),
        ] {
            connection.execute_batch(migration).expect("migration");
        }
        connection
            .execute(
                "INSERT INTO playlists(id, name, kind) VALUES ('smart-old', 'Old rule', 'smart')",
                [],
            )
            .expect("old playlist");
        connection
            .execute(
                "INSERT INTO smart_playlist_rules(playlist_id, genre, min_bpm, max_bpm, limit_count, sort_by, descending) VALUES ('smart-old', 'Techno', 130, 150, 25, 'bpm', 0)",
                [],
            )
            .expect("old rule");
        drop(connection);

        let state = AppState::open(&database_path).expect("upgrade database");
        let playlist = state.list_playlists().expect("playlists").remove(0);
        let rule = playlist.smart_rule.expect("preserved rule");
        assert_eq!(rule.genre.as_deref(), Some("Techno"));
        assert_eq!(rule.min_bpm, Some(130.0));
        assert_eq!(rule.max_bpm, Some(150.0));
        assert_eq!(rule.limit_count, 25);
        assert_eq!(rule.sort_by, "bpm");
        assert!(!rule.descending);
        assert!(rule.subgenre.is_none());
        assert!(rule.min_rating.is_none());
        assert!(rule.camelot.is_none());
        drop(state);
        std::fs::remove_file(database_path).expect("cleanup");
    }

    #[test]
    fn playback_path_rejects_missing_files() {
        let state = AppState::in_memory().expect("database");
        let root = state.register_root("D:\\Music").expect("root");
        state
            .upsert_file(
                &root,
                "D:\\Music\\Missing.flac",
                "Missing.flac",
                "flac",
                42,
                10,
                "missing-file-id",
                &AudioMetadata::default(),
                "available",
            )
            .expect("insert");
        let track = state.list_tracks(1, 0, "").expect("track").remove(0);
        assert!(matches!(
            state.playback_path(&track.id),
            Err(AppError::TrackUnavailable(_))
        ));
    }

    #[test]
    fn rating_update_is_guarded_and_recorded_once() {
        let state = AppState::in_memory().expect("database");
        let root = state.register_root("D:\\Music").expect("root");
        state
            .upsert_file(
                &root,
                "D:\\Music\\Rated.flac",
                "Rated.flac",
                "flac",
                42,
                10,
                "rated-file-id",
                &AudioMetadata::default(),
                "available",
            )
            .expect("track");
        let track_id = state
            .list_tracks(1, 0, "Rated")
            .expect("tracks")
            .remove(0)
            .id;
        state.set_track_rating(&track_id, 0, 4).expect("set rating");
        state
            .set_track_rating(&track_id, 4, 4)
            .expect("rating no-op");
        assert!(state.set_track_rating(&track_id, 0, 5).is_err());
        assert!(state.set_track_rating(&track_id, 4, 6).is_err());
        let connection = state.connection().expect("connection");
        let rating: u8 = connection
            .query_row(
                "SELECT rating FROM tracks WHERE id = ?1",
                [&track_id],
                |row| row.get(0),
            )
            .expect("rating");
        let history: (u64, u8, u8) = connection
            .query_row(
                "SELECT count(*), min(before_rating), max(after_rating) FROM track_rating_history WHERE track_id = ?1",
                [&track_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("rating history");
        assert_eq!(rating, 4);
        assert_eq!(history, (1, 0, 4));
        drop(connection);
        let bulk = state
            .bulk_set_track_rating(
                &[track_id.clone(), "missing-track".into(), track_id.clone()],
                5,
            )
            .expect("bulk rating");
        assert_eq!(bulk.total_tracks, 2);
        assert_eq!(bulk.saved_tracks, 1);
        assert_eq!(bulk.unchanged_tracks, 0);
        assert_eq!(bulk.failed_tracks, 1);
        let bulk_history: (u64, u64) = state
            .connection()
            .expect("connection")
            .query_row(
                "SELECT count(*), count(*) FILTER (WHERE source = 'bulk') FROM track_rating_history WHERE track_id = ?1",
                [&track_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("bulk history");
        assert_eq!(bulk_history, (2, 1));
    }

    #[test]
    fn library_table_layout_is_validated_and_persisted() {
        let state = AppState::in_memory().expect("database");
        let mut layout = state.library_table_layout().expect("default layout");
        assert_eq!(layout.columns.len(), 6);
        assert!(layout.columns.iter().all(|column| column.visible));

        layout.columns.swap(0, 2);
        let genre = layout
            .columns
            .iter_mut()
            .find(|column| column.id == "genre")
            .unwrap();
        genre.visible = false;
        genre.width = 210;
        state
            .save_library_table_layout(layout.clone())
            .expect("save layout");
        assert_eq!(state.library_table_layout().expect("stored layout"), layout);

        let track = layout
            .columns
            .iter_mut()
            .find(|column| column.id == "track")
            .unwrap();
        track.visible = false;
        assert!(state.save_library_table_layout(layout).is_err());
    }

    #[test]
    fn creates_reorders_and_moves_playlist_items() {
        let state = AppState::in_memory().expect("database");
        let root = state.register_root("D:\\Music").expect("root");
        for (index, filename) in ["Artist - First.flac", "Artist - Second.mp3"]
            .iter()
            .enumerate()
        {
            state
                .upsert_file(
                    &root,
                    &format!("D:\\Music\\{filename}"),
                    filename,
                    if filename.ends_with("mp3") {
                        "mp3"
                    } else {
                        "flac"
                    },
                    42,
                    index as i64,
                    &format!("playlist-file-{index}"),
                    &AudioMetadata::default(),
                    "available",
                )
                .expect("track");
        }
        let first_track = state.list_tracks(1, 0, "First").expect("first").remove(0);
        let second_track = state.list_tracks(1, 0, "Second").expect("second").remove(0);
        let source = state.create_playlist("Source").expect("source");
        let target = state.create_playlist("Target").expect("target");

        state
            .place_track_in_playlist(&source, 0, &first_track.id, None, None)
            .expect("add first");
        state
            .place_track_in_playlist(&source, 0, &second_track.id, None, None)
            .expect("add second at beginning");
        let playlists = state.list_playlists().expect("playlists");
        let source_detail = playlists
            .iter()
            .find(|item| item.id == source)
            .expect("source");
        assert_eq!(source_detail.items[0].track.id, second_track.id);

        let moved_item = source_detail.items[0].id.clone();
        state
            .place_track_in_playlist(
                &target,
                0,
                &second_track.id,
                Some(&moved_item),
                Some(&source),
            )
            .expect("move between playlists");
        let playlists = state.list_playlists().expect("playlists after move");
        assert_eq!(
            playlists
                .iter()
                .find(|item| item.id == source)
                .expect("source")
                .items
                .len(),
            1
        );
        assert_eq!(
            playlists
                .iter()
                .find(|item| item.id == target)
                .expect("target")
                .items[0]
                .track
                .id,
            second_track.id
        );
    }

    #[test]
    fn removes_only_the_playlist_after_a_current_preview() {
        let state = AppState::in_memory().expect("database");
        let root = state.register_root("D:\\Music").expect("root");
        state
            .upsert_file(
                &root,
                "D:\\Music\\Artist - Track.flac",
                "Artist - Track.flac",
                "flac",
                123,
                456,
                "playlist-removal-file",
                &AudioMetadata::default(),
                "available",
            )
            .expect("track");
        let track = state.list_tracks(1, 0, "").expect("tracks").remove(0);
        let playlist = state.create_playlist("Temporary").expect("playlist");
        state
            .place_track_in_playlist(&playlist, 0, &track.id, None, None)
            .expect("place track");

        let stale_preview = state.preview_playlist_removal(&playlist).expect("preview");
        assert_eq!(stale_preview.item_count, 1);
        state.rename_playlist(&playlist, "Changed").expect("rename");
        assert!(
            state
                .remove_playlist(&playlist, &stale_preview.expected_state)
                .is_err()
        );

        let current_preview = state
            .preview_playlist_removal(&playlist)
            .expect("current preview");
        let report = state
            .remove_playlist(&playlist, &current_preview.expected_state)
            .expect("remove playlist");
        assert_eq!(report.removed_items, 1);
        assert!(state.list_playlists().expect("playlists").is_empty());
        assert_eq!(state.summary().expect("summary").tracks, 1);
    }

    #[test]
    fn smart_playlist_is_dynamic_and_rejects_manual_placement() {
        let state = AppState::in_memory().expect("database");
        let root = state.register_root("D:\\Music").expect("root");
        for (index, (filename, bpm, genre)) in [
            ("Artist - Slow.flac", 130.0, "Techno"),
            ("Artist - Match.flac", 145.0, "Techno"),
            ("Artist - Fast.flac", 160.0, "House"),
        ]
        .into_iter()
        .enumerate()
        {
            let audio = AudioMetadata {
                bpm: Some(bpm),
                camelot: Some(if genre == "Techno" { "4A" } else { "8A" }.into()),
                ..AudioMetadata::default()
            };
            state
                .upsert_file(
                    &root,
                    &format!("D:\\Music\\{filename}"),
                    filename,
                    "flac",
                    100 + index as u64,
                    index as i64,
                    &format!("smart-file-{index}"),
                    &audio,
                    "available",
                )
                .expect("track");
            let track_id = state
                .list_tracks(1, 0, filename.split(" - ").nth(1).expect("title"))
                .expect("track query")
                .remove(0)
                .id;
            state
                .connection()
                .expect("connection")
                .execute(
                    "UPDATE tracks SET confirmed_genre = ?1, confirmed_subgenre = ?2, rating = ?3 WHERE id = ?4",
                    params![genre, (genre == "Techno").then_some("Acid Techno"), if filename.contains("Slow") { 5 } else { 4 }, track_id],
                )
                .expect("genre");
        }

        let playlist_id = state
            .create_smart_playlist(
                "Fast Techno",
                SmartPlaylistRule {
                    genre: Some("Techno".into()),
                    subgenre: Some("Acid Techno".into()),
                    min_bpm: Some(140.0),
                    max_bpm: Some(150.0),
                    min_rating: Some(4),
                    camelot: Some("4A".into()),
                    limit_count: 50,
                    sort_by: "bpm".into(),
                    descending: false,
                },
            )
            .expect("smart playlist");
        let playlists = state.list_playlists().expect("playlists");
        assert_eq!(playlists[0].kind, "smart");
        assert_eq!(playlists[0].items.len(), 1);
        assert_eq!(playlists[0].items[0].track.title, "Match");
        let snapshot = state
            .snapshot_smart_playlist(&playlist_id, "Fast Techno — снимок")
            .expect("snapshot");
        assert_eq!(snapshot.item_count, 1);
        let snapshot_playlists = state.list_playlists().expect("snapshot playlists");
        let manual = snapshot_playlists
            .iter()
            .find(|playlist| playlist.id == snapshot.playlist_id)
            .expect("manual snapshot");
        assert_eq!(manual.kind, "manual");
        assert!(manual.smart_rule.is_none());
        assert_eq!(manual.items.len(), 1);
        assert_eq!(manual.items[0].track.title, "Match");
        state
            .connection()
            .expect("connection")
            .execute(
                "UPDATE track_metadata SET bpm = 145 WHERE track_file_id IN (SELECT f.id FROM track_files f JOIN tracks t ON t.id = f.track_id WHERE t.preferred_title = 'Slow')",
                [],
            )
            .expect("expand smart result");
        let changed_playlists = state.list_playlists().expect("changed playlists");
        let changed_smart = changed_playlists
            .iter()
            .find(|playlist| playlist.id == playlist_id)
            .expect("changed smart playlist");
        let unchanged_snapshot = changed_playlists
            .iter()
            .find(|playlist| playlist.id == snapshot.playlist_id)
            .expect("unchanged snapshot");
        assert_eq!(changed_smart.items.len(), 2);
        assert_eq!(unchanged_snapshot.items.len(), 1);
        let expected_rule = changed_smart.smart_rule.clone().expect("current rule");
        let next_rule = SmartPlaylistRule {
            genre: Some("House".into()),
            subgenre: None,
            min_bpm: Some(150.0),
            max_bpm: Some(170.0),
            min_rating: Some(4),
            camelot: Some("8A".into()),
            limit_count: 10,
            sort_by: "title".into(),
            descending: false,
        };
        state
            .update_smart_playlist_rule(&playlist_id, expected_rule.clone(), next_rule.clone())
            .expect("update smart rule");
        let updated_playlists = state.list_playlists().expect("updated playlists");
        let updated_smart = updated_playlists
            .iter()
            .find(|playlist| playlist.id == playlist_id)
            .expect("updated smart playlist");
        assert_eq!(updated_smart.smart_rule.as_ref(), Some(&next_rule));
        assert_eq!(updated_smart.items.len(), 1);
        assert_eq!(updated_smart.items[0].track.title, "Fast");
        assert!(
            state
                .update_smart_playlist_rule(&playlist_id, expected_rule, next_rule)
                .is_err()
        );
        let rating_only = state
            .create_smart_playlist(
                "Five stars",
                SmartPlaylistRule {
                    genre: None,
                    subgenre: None,
                    min_bpm: None,
                    max_bpm: None,
                    min_rating: Some(5),
                    camelot: None,
                    limit_count: 10,
                    sort_by: "title".into(),
                    descending: false,
                },
            )
            .expect("rating-only smart playlist");
        let rating_playlists = state.list_playlists().expect("rating playlists");
        let rated = rating_playlists
            .iter()
            .find(|playlist| playlist.id == rating_only)
            .expect("rated playlist");
        assert_eq!(rated.items.len(), 1);
        assert_eq!(rated.items[0].track.title, "Slow");
        assert!(
            state
                .create_smart_playlist(
                    "Invalid subgenre",
                    SmartPlaylistRule {
                        genre: Some("House".into()),
                        subgenre: Some("Acid Techno".into()),
                        min_bpm: None,
                        max_bpm: None,
                        min_rating: None,
                        camelot: None,
                        limit_count: 10,
                        sort_by: "added".into(),
                        descending: true,
                    },
                )
                .is_err()
        );
        assert!(
            state
                .snapshot_smart_playlist(&snapshot.playlist_id, "Nested snapshot")
                .is_err()
        );
        assert!(
            state
                .place_track_in_playlist(
                    &playlist_id,
                    0,
                    &playlists[0].items[0].track.id,
                    None,
                    None,
                )
                .is_err()
        );
        let persisted_items: u64 = state
            .connection()
            .expect("connection")
            .query_row(
                "SELECT COUNT(*) FROM playlist_items WHERE playlist_id = ?1",
                [&playlist_id],
                |row| row.get(0),
            )
            .expect("persisted item count");
        assert_eq!(persisted_items, 0);
    }
}
