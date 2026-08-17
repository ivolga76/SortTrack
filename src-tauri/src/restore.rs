use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};

use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    backup,
    error::AppError,
    file_copy::sha256_file,
    models::{RestorePrepared, RestorePreview},
    repository::{AppState, CURRENT_SCHEMA_VERSION},
};

const ACTIVE_DATABASE_FILENAME: &str = "sorttrack.db";
const PENDING_FILENAME: &str = "pending-restore.json";

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct PendingRestore {
    format_version: u32,
    staged_filename: String,
    rollback_filename: String,
    expected_database_sha256: String,
    backup_path: String,
    safety_backup_path: String,
}

pub fn preview(directory: &Path) -> Result<RestorePreview, AppError> {
    let verified = backup::verify(directory)?;
    validate_schema(verified.database_schema_version)?;
    Ok(RestorePreview {
        backup_path: verified.directory.to_string_lossy().into_owned(),
        created_at: verified.created_at,
        application_version: verified.application_version,
        database_sha256: verified.database_sha256,
        manifest_sha256: verified.manifest_sha256,
        database_size_bytes: verified.database_size_bytes,
        database_schema_version: verified.database_schema_version,
        current_schema_version: CURRENT_SCHEMA_VERSION,
        requires_migration: verified.database_schema_version < CURRENT_SCHEMA_VERSION,
        counts: verified.counts,
    })
}

pub fn prepare(
    state: &AppState,
    directory: &Path,
    expected_manifest_sha256: &str,
) -> Result<RestorePrepared, AppError> {
    let verified = backup::verify(directory)?;
    validate_schema(verified.database_schema_version)?;
    if verified.manifest_sha256 != expected_manifest_sha256 {
        return Err(AppError::Task(
            "Backup изменился после preview; восстановление остановлено".into(),
        ));
    }
    let active_database = state.database_path()?;
    let database_directory = active_database
        .parent()
        .ok_or_else(|| AppError::Task("Не удалось определить папку активной базы".into()))?;
    let pending_path = database_directory.join(PENDING_FILENAME);
    if pending_path.exists() {
        return Err(AppError::Task(
            "Уже подготовлено другое восстановление; перезапустите SortTrack".into(),
        ));
    }

    state
        .connection()?
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
    let safety = backup::create(state, database_directory)?;
    let token = Uuid::new_v4();
    let staged_filename = format!(".sorttrack-restore-{token}.sqlite3");
    let rollback_filename = format!(".sorttrack-restore-rollback-{token}.sqlite3");
    let staged_path = database_directory.join(&staged_filename);
    copy_new(&verified.database_path, &staged_path)?;
    let prepare_result = (|| -> Result<RestorePrepared, AppError> {
        verify_database(&staged_path, &verified.database_sha256)?;
        let pending = PendingRestore {
            format_version: 1,
            staged_filename,
            rollback_filename,
            expected_database_sha256: verified.database_sha256.clone(),
            backup_path: verified.directory.to_string_lossy().into_owned(),
            safety_backup_path: safety.backup_path.clone(),
        };
        write_pending_atomic(database_directory, &pending)?;
        Ok(RestorePrepared {
            backup_path: pending.backup_path,
            safety_backup_path: pending.safety_backup_path,
            expected_database_sha256: pending.expected_database_sha256,
            restart_required: true,
        })
    })();
    if prepare_result.is_err() {
        let _ = std::fs::remove_file(&staged_path);
    }
    prepare_result
}

pub fn apply_pending(database_directory: &Path) -> Result<(), AppError> {
    let pending_path = database_directory.join(PENDING_FILENAME);
    if !pending_path.exists() {
        return Ok(());
    }
    let pending: PendingRestore = serde_json::from_slice(&std::fs::read(&pending_path)?)
        .map_err(|error| AppError::Task(format!("Некорректный pending restore: {error}")))?;
    if pending.format_version != 1
        || !safe_internal_filename(&pending.staged_filename, ".sorttrack-restore-")
        || !safe_internal_filename(&pending.rollback_filename, ".sorttrack-restore-rollback-")
    {
        return Err(AppError::Task("Некорректный pending restore".into()));
    }
    let active = database_directory.join(ACTIVE_DATABASE_FILENAME);
    let staged = database_directory.join(&pending.staged_filename);
    let rollback = database_directory.join(&pending.rollback_filename);

    if active.exists()
        && sha256_file(&active).ok().as_deref() == Some(&pending.expected_database_sha256)
    {
        remove_sidecars(&active);
        verify_database(&active, &pending.expected_database_sha256)?;
        finish_restore(&pending_path, &staged, &rollback);
        return Ok(());
    }

    if verify_database(&staged, &pending.expected_database_sha256).is_err() {
        if active.exists() {
            let _ = std::fs::remove_file(&staged);
            std::fs::remove_file(&pending_path)?;
            return Ok(());
        }
        if rollback.exists() {
            std::fs::rename(&rollback, &active)?;
            move_sidecars(&rollback, &active)?;
            let _ = std::fs::remove_file(&staged);
            std::fs::remove_file(&pending_path)?;
            return Ok(());
        }
        return Err(AppError::Task(
            "Ни восстановленная, ни предыдущая база недоступны".into(),
        ));
    }

    if !active.exists() && rollback.exists() {
        move_sidecars(&active, &rollback)?;
    }

    if active.exists() {
        if rollback.exists() {
            return Err(AppError::Task(
                "Обнаружено неоднозначное состояние восстановления".into(),
            ));
        }
        std::fs::rename(&active, &rollback)?;
        if let Err(error) = move_sidecars(&active, &rollback) {
            let _ = std::fs::rename(&rollback, &active);
            return Err(error);
        }
    }
    if let Err(error) = std::fs::rename(&staged, &active) {
        if rollback.exists() && !active.exists() {
            let _ = std::fs::rename(&rollback, &active);
            let _ = move_sidecars(&rollback, &active);
        }
        return Err(error.into());
    }
    if let Err(error) = verify_database(&active, &pending.expected_database_sha256) {
        let failed = database_directory.join(format!(
            ".sorttrack-failed-restore-{}.sqlite3",
            Uuid::new_v4()
        ));
        let _ = std::fs::rename(&active, failed);
        if rollback.exists() {
            let _ = std::fs::rename(&rollback, &active);
            let _ = move_sidecars(&rollback, &active);
        }
        return Err(error);
    }
    finish_restore(&pending_path, &staged, &rollback);
    Ok(())
}

fn validate_schema(version: u32) -> Result<(), AppError> {
    if version == 0 || version > CURRENT_SCHEMA_VERSION {
        return Err(AppError::Task(format!(
            "Версия схемы backup {version} несовместима с SortTrack (поддерживается до {CURRENT_SCHEMA_VERSION})"
        )));
    }
    Ok(())
}

fn verify_database(path: &Path, expected_sha256: &str) -> Result<(), AppError> {
    if !path.is_file() || sha256_file(path)? != expected_sha256 {
        return Err(AppError::Task(
            "SHA-256 подготовленной базы не совпадает".into(),
        ));
    }
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let integrity: String = connection.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    let schema: u32 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err(AppError::Task(
            "Integrity check подготовленной базы не пройден".into(),
        ));
    }
    validate_schema(schema)
}

fn copy_new(source: &Path, destination: &Path) -> Result<(), AppError> {
    let mut input = OpenOptions::new().read(true).open(source)?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?;
    std::io::copy(&mut input, &mut output)?;
    output.sync_all()?;
    Ok(())
}

fn write_pending_atomic(directory: &Path, pending: &PendingRestore) -> Result<(), AppError> {
    let final_path = directory.join(PENDING_FILENAME);
    let temporary = directory.join(format!(".sorttrack-pending-{}.tmp", Uuid::new_v4()));
    let bytes =
        serde_json::to_vec_pretty(pending).map_err(|error| AppError::Task(error.to_string()))?;
    let result = (|| -> Result<(), AppError> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        std::fs::hard_link(&temporary, &final_path)?;
        std::fs::remove_file(&temporary)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temporary);
    }
    result
}

fn safe_internal_filename(value: &str, prefix: &str) -> bool {
    let path = Path::new(value);
    value.starts_with(prefix)
        && value.ends_with(".sqlite3")
        && path.file_name().and_then(|name| name.to_str()) == Some(value)
}

fn finish_restore(pending: &Path, staged: &Path, rollback: &Path) {
    let _ = std::fs::remove_file(staged);
    let _ = std::fs::remove_file(rollback);
    remove_sidecars(rollback);
    let _ = std::fs::remove_file(pending);
}

fn sidecar(database: &Path, suffix: &str) -> PathBuf {
    let mut value = database.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

fn move_sidecars(source: &Path, destination: &Path) -> Result<(), AppError> {
    let mut moved = Vec::new();
    for suffix in ["-wal", "-shm"] {
        let from = sidecar(source, suffix);
        if from.exists() {
            let to = sidecar(destination, suffix);
            if let Err(error) = std::fs::rename(&from, &to) {
                for (moved_from, moved_to) in moved.into_iter().rev() {
                    let _ = std::fs::rename(moved_to, moved_from);
                }
                return Err(error.into());
            }
            moved.push((from, to));
        }
    }
    Ok(())
}

fn remove_sidecars(database: &Path) {
    for suffix in ["-wal", "-shm"] {
        let _ = std::fs::remove_file(sidecar(database, suffix));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio_metadata::AudioMetadata;

    fn temporary_directory(label: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("sorttrack-restore-test-{label}-{}", Uuid::new_v4()));
        std::fs::create_dir(&path).expect("temporary directory");
        path
    }

    fn add_track(state: &AppState, filename: &str, identity: &str) {
        let root = state.register_root("D:\\Music").expect("root");
        state
            .upsert_file(
                &root,
                &format!("D:\\Music\\{filename}"),
                filename,
                "flac",
                42,
                10,
                identity,
                &AudioMetadata::default(),
                "available",
            )
            .expect("track");
    }

    #[test]
    fn resumes_after_crash_between_old_and_restored_database_renames() {
        let source_root = temporary_directory("source");
        let source_database_dir = source_root.join("database");
        std::fs::create_dir(&source_database_dir).expect("source db directory");
        let source = AppState::open(&source_database_dir.join(ACTIVE_DATABASE_FILENAME))
            .expect("source database");
        add_track(&source, "Source.flac", "source-identity");
        let backup_root = source_root.join("backups");
        std::fs::create_dir(&backup_root).expect("backup directory");
        let backup_report = backup::create(&source, &backup_root).expect("backup");
        drop(source);

        let target_root = temporary_directory("target");
        let target_database_dir = target_root.join("database");
        std::fs::create_dir(&target_database_dir).expect("target db directory");
        let target = AppState::open(&target_database_dir.join(ACTIVE_DATABASE_FILENAME))
            .expect("target database");
        add_track(&target, "Target.flac", "target-identity");
        let preview = preview(Path::new(&backup_report.backup_path)).expect("preview");
        let prepared = prepare(
            &target,
            Path::new(&backup_report.backup_path),
            &preview.manifest_sha256,
        )
        .expect("prepare");
        assert!(Path::new(&prepared.safety_backup_path).is_dir());
        drop(target);

        let pending_path = target_database_dir.join(PENDING_FILENAME);
        let pending: PendingRestore =
            serde_json::from_slice(&std::fs::read(&pending_path).expect("pending"))
                .expect("pending manifest");
        std::fs::rename(
            target_database_dir.join(ACTIVE_DATABASE_FILENAME),
            target_database_dir.join(&pending.rollback_filename),
        )
        .expect("simulate interrupted swap");
        std::fs::write(
            sidecar(&target_database_dir.join(ACTIVE_DATABASE_FILENAME), "-wal"),
            b"stale wal",
        )
        .expect("stale wal");
        std::fs::write(
            sidecar(&target_database_dir.join(ACTIVE_DATABASE_FILENAME), "-shm"),
            b"stale shm",
        )
        .expect("stale shm");

        apply_pending(&target_database_dir).expect("resume restore");
        assert!(!sidecar(&target_database_dir.join(ACTIVE_DATABASE_FILENAME), "-wal").exists());
        let restored =
            AppState::open(&target_database_dir.join(ACTIVE_DATABASE_FILENAME)).expect("restored");
        let tracks = restored.list_tracks(10, 0, "").expect("tracks");
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].title, "Source");
        assert!(!pending_path.exists());
        drop(restored);
        std::fs::remove_dir_all(source_root).expect("source cleanup");
        std::fs::remove_dir_all(target_root).expect("target cleanup");
    }

    #[test]
    fn rejects_backup_changed_after_preview() {
        let root = temporary_directory("changed");
        let database_dir = root.join("database");
        std::fs::create_dir(&database_dir).expect("db directory");
        let state = AppState::open(&database_dir.join(ACTIVE_DATABASE_FILENAME)).expect("database");
        let backup_root = root.join("backups");
        std::fs::create_dir(&backup_root).expect("backup directory");
        let report = backup::create(&state, &backup_root).expect("backup");
        assert!(prepare(&state, Path::new(&report.backup_path), "wrong-manifest-sha").is_err());
        drop(state);
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
