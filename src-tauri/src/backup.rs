use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};

use chrono::Utc;
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::AppError,
    file_copy::sha256_file,
    models::{BackupCounts, BackupReport},
    repository::AppState,
};

pub(crate) const BACKUP_FORMAT_VERSION: u32 = 1;
pub(crate) const DATABASE_FILENAME: &str = "sorttrack.sqlite3";
pub(crate) const MANIFEST_FILENAME: &str = "manifest.json";

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct BackupManifest {
    backup_format_version: u32,
    created_at: String,
    application: String,
    application_version: String,
    database_file: String,
    database_sha256: String,
    database_size_bytes: u64,
    database_schema_version: u32,
    integrity_check: String,
    counts: BackupCounts,
    excluded: Vec<String>,
}

pub(crate) struct VerifiedBackup {
    pub directory: PathBuf,
    pub database_path: PathBuf,
    pub created_at: String,
    pub application_version: String,
    pub database_sha256: String,
    pub manifest_sha256: String,
    pub database_size_bytes: u64,
    pub database_schema_version: u32,
    pub counts: BackupCounts,
}

pub(crate) fn verify(directory: &Path) -> Result<VerifiedBackup, AppError> {
    let directory = directory.canonicalize()?;
    if !directory.is_dir() {
        return Err(AppError::Task("Папка backup недоступна".into()));
    }
    let manifest_path = directory.join(MANIFEST_FILENAME);
    let bytes = std::fs::read(&manifest_path)?;
    let manifest: BackupManifest = serde_json::from_slice(&bytes)
        .map_err(|error| AppError::Task(format!("Некорректный manifest backup: {error}")))?;
    if manifest.backup_format_version != BACKUP_FORMAT_VERSION
        || manifest.application != "SortTrack"
        || manifest.database_file != DATABASE_FILENAME
        || manifest.integrity_check != "ok"
    {
        return Err(AppError::Task("Формат backup не поддерживается".into()));
    }
    let database_path = directory.join(DATABASE_FILENAME).canonicalize()?;
    if !database_path.starts_with(&directory) || !database_path.is_file() {
        return Err(AppError::Task("Файл базы backup недоступен".into()));
    }
    if database_path.metadata()?.len() != manifest.database_size_bytes {
        return Err(AppError::Task(
            "Размер базы backup не совпадает с manifest".into(),
        ));
    }
    let database_sha256 = sha256_file(&database_path)?;
    if database_sha256 != manifest.database_sha256 {
        return Err(AppError::Task(
            "SHA-256 базы backup не совпадает с manifest".into(),
        ));
    }
    let snapshot = Connection::open_with_flags(&database_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let integrity: String = snapshot.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    let actual_schema: u32 = snapshot.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if integrity != "ok" || actual_schema != manifest.database_schema_version {
        return Err(AppError::Task(
            "Проверка целостности backup не пройдена".into(),
        ));
    }
    let actual_counts = BackupCounts {
        tracks: count(&snapshot, "tracks")?,
        track_files: count(&snapshot, "track_files")?,
        playlists: count(&snapshot, "playlists")?,
        metadata_history: count(&snapshot, "metadata_history")?,
        genre_history: count(&snapshot, "genre_confirmation_history")?,
        dataset_exports: count(&snapshot, "genre_dataset_exports")?,
        model_runs: count(&snapshot, "genre_model_runs")?,
    };
    if actual_counts != manifest.counts {
        return Err(AppError::Task(
            "Содержимое backup не совпадает с manifest".into(),
        ));
    }
    Ok(VerifiedBackup {
        directory,
        database_path,
        created_at: manifest.created_at,
        application_version: manifest.application_version.to_owned(),
        database_sha256,
        manifest_sha256: sha256_file(&manifest_path)?,
        database_size_bytes: manifest.database_size_bytes,
        database_schema_version: actual_schema,
        counts: actual_counts,
    })
}

pub fn create(state: &AppState, destination_root: &Path) -> Result<BackupReport, AppError> {
    let destination_root = destination_root.canonicalize()?;
    if !destination_root.is_dir() {
        return Err(AppError::Task(
            "Папка назначения резервной копии недоступна".into(),
        ));
    }

    let token = Uuid::new_v4();
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
    let final_directory = destination_root.join(format!(
        "SortTrack-backup-{timestamp}-{}",
        &token.to_string()[..8]
    ));
    let temporary_directory = destination_root.join(format!(".sorttrack-backup-{token}.tmp"));
    std::fs::create_dir(&temporary_directory)?;
    let database_path = temporary_directory.join(DATABASE_FILENAME);
    let manifest_path = temporary_directory.join(MANIFEST_FILENAME);

    let result = create_in_directory(state, &database_path, &manifest_path).and_then(|report| {
        if final_directory.exists() {
            return Err(AppError::Task(
                "Папка резервной копии уже существует".into(),
            ));
        }
        std::fs::rename(&temporary_directory, &final_directory)?;
        Ok(BackupReport {
            backup_path: final_directory.to_string_lossy().into_owned(),
            database_path: final_directory
                .join(DATABASE_FILENAME)
                .to_string_lossy()
                .into_owned(),
            manifest_path: final_directory
                .join(MANIFEST_FILENAME)
                .to_string_lossy()
                .into_owned(),
            ..report
        })
    });

    if result.is_err() {
        cleanup_temporary(&temporary_directory, &database_path, &manifest_path);
    }
    result
}

fn create_in_directory(
    state: &AppState,
    database_path: &Path,
    manifest_path: &Path,
) -> Result<BackupReport, AppError> {
    {
        let connection = state.connection()?;
        connection.execute("VACUUM INTO ?1", [database_path.to_string_lossy().as_ref()])?;
    }
    OpenOptions::new()
        .write(true)
        .open(database_path)?
        .sync_all()?;

    let snapshot = Connection::open_with_flags(database_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let integrity: String = snapshot.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err(AppError::Task(format!(
            "Проверка целостности резервной копии не пройдена: {integrity}"
        )));
    }
    let database_schema_version =
        snapshot.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    let counts = BackupCounts {
        tracks: count(&snapshot, "tracks")?,
        track_files: count(&snapshot, "track_files")?,
        playlists: count(&snapshot, "playlists")?,
        metadata_history: count(&snapshot, "metadata_history")?,
        genre_history: count(&snapshot, "genre_confirmation_history")?,
        dataset_exports: count(&snapshot, "genre_dataset_exports")?,
        model_runs: count(&snapshot, "genre_model_runs")?,
    };
    drop(snapshot);

    let database_size_bytes = database_path.metadata()?.len();
    let database_sha256 = sha256_file(database_path)?;
    let manifest = BackupManifest {
        backup_format_version: BACKUP_FORMAT_VERSION,
        created_at: Utc::now().to_rfc3339(),
        application: "SortTrack".into(),
        application_version: env!("CARGO_PKG_VERSION").into(),
        database_file: DATABASE_FILENAME.into(),
        database_sha256: database_sha256.clone(),
        database_size_bytes,
        database_schema_version,
        integrity_check: "ok".into(),
        counts: counts.clone(),
        excluded: vec!["audio files".into(), "rebuildable waveform cache".into()],
    };
    let bytes =
        serde_json::to_vec_pretty(&manifest).map_err(|error| AppError::Task(error.to_string()))?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(manifest_path)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    let manifest_sha256 = sha256_file(manifest_path)?;

    Ok(BackupReport {
        backup_path: String::new(),
        database_path: String::new(),
        manifest_path: String::new(),
        database_sha256,
        manifest_sha256,
        database_size_bytes,
        database_schema_version,
        counts,
    })
}

fn count(connection: &Connection, table: &str) -> Result<u64, AppError> {
    let allowed = [
        "tracks",
        "track_files",
        "playlists",
        "metadata_history",
        "genre_confirmation_history",
        "genre_dataset_exports",
        "genre_model_runs",
    ];
    if !allowed.contains(&table) {
        return Err(AppError::Task("Недопустимая таблица отчёта backup".into()));
    }
    Ok(
        connection.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
            row.get(0)
        })?,
    )
}

fn cleanup_temporary(directory: &Path, database: &Path, manifest: &Path) {
    let _ = std::fs::remove_file(manifest);
    let _ = std::fs::remove_file(database);
    let _ = std::fs::remove_file(database.with_extension("sqlite3-journal"));
    let _ = std::fs::remove_dir(directory);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio_metadata::AudioMetadata;

    fn temporary_directory() -> PathBuf {
        let path = std::env::temp_dir().join(format!("sorttrack-backup-test-{}", Uuid::new_v4()));
        std::fs::create_dir(&path).expect("temporary directory");
        path
    }

    #[test]
    fn creates_verified_snapshot_with_manifest() {
        let state = AppState::in_memory().expect("database");
        let root = state.register_root("D:\\Music").expect("root");
        state
            .upsert_file(
                &root,
                "D:\\Music\\Track.flac",
                "Track.flac",
                "flac",
                42,
                10,
                "backup-track",
                &AudioMetadata::default(),
                "available",
            )
            .expect("track");
        let destination = temporary_directory();
        let report = create(&state, &destination).expect("backup");
        assert_eq!(
            report.database_schema_version,
            crate::repository::CURRENT_SCHEMA_VERSION
        );
        assert_eq!(report.counts.tracks, 1);
        assert!(Path::new(&report.database_path).is_file());
        assert!(Path::new(&report.manifest_path).is_file());
        let snapshot = Connection::open(&report.database_path).expect("snapshot");
        assert_eq!(count(&snapshot, "tracks").expect("count"), 1);
        drop(snapshot);
        let second = create(&state, &destination).expect("second backup");
        assert_ne!(second.backup_path, report.backup_path);
        assert!(Path::new(&report.database_path).is_file());
        assert!(Path::new(&second.database_path).is_file());
        std::fs::remove_dir_all(destination).expect("cleanup");
    }

    #[test]
    fn refuses_a_non_directory_destination() {
        let state = AppState::in_memory().expect("database");
        let destination = temporary_directory().join("missing");
        assert!(create(&state, &destination).is_err());
        std::fs::remove_dir(destination.parent().expect("parent")).expect("cleanup");
    }
}
