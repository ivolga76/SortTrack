use std::{collections::HashSet, path::Path};

use rusqlite::{OptionalExtension, params};
use uuid::Uuid;

use crate::{
    audio_analysis, error::AppError, file_identity::file_identity, models::AnalysisJob,
    repository::AppState,
};

#[derive(Debug)]
struct AnalysisItem {
    id: String,
    track_file_id: String,
    path: String,
    size_bytes: u64,
    modified_at_ms: i64,
    expected_identity: Option<String>,
}

pub fn recover_supported_jobs(state: &AppState) -> Result<(), AppError> {
    let mut connection = state.connection()?;
    let transaction = connection.transaction()?;
    transaction.execute(
        "UPDATE background_job_items SET state = 'queued', updated_at = CURRENT_TIMESTAMP WHERE state = 'running' AND job_id IN (SELECT id FROM background_jobs WHERE job_type = 'bpm_key' AND state = 'running')",
        [],
    )?;
    transaction.execute(
        "UPDATE background_jobs SET state = 'recovery_required', updated_at = CURRENT_TIMESTAMP WHERE job_type = 'bpm_key' AND state = 'running'",
        [],
    )?;
    transaction.commit()?;
    Ok(())
}

pub fn enqueue(state: &AppState, track_ids: &[String]) -> Result<AnalysisJob, AppError> {
    let unique = track_ids.iter().cloned().collect::<HashSet<_>>();
    if unique.is_empty() || unique.len() > 1000 {
        return Err(AppError::Task(
            "Для одного задания выберите от 1 до 1000 треков".into(),
        ));
    }
    let connection = state.connection()?;
    let mut files = Vec::with_capacity(unique.len());
    for track_id in unique {
        let file = connection
            .query_row(
                "SELECT f.id, f.size_bytes FROM track_files f WHERE f.track_id = ?1 AND f.availability_status = 'available' ORDER BY CASE lower(f.extension) WHEN 'flac' THEN 0 ELSE 1 END, f.updated_at DESC LIMIT 1",
                [&track_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?)),
            )
            .optional()?;
        if let Some(file) = file {
            files.push(file);
        }
    }
    if files.is_empty() {
        return Err(AppError::Task("Нет доступных MP3/FLAC для анализа".into()));
    }
    let job_id = Uuid::new_v4().to_string();
    let total_bytes = files.iter().map(|(_, size)| *size).sum::<u64>();
    drop(connection);
    let mut connection = state.connection()?;
    let transaction = connection.transaction()?;
    transaction.execute(
        "INSERT INTO background_jobs(id, job_type, state, priority, payload_json, total_items, total_bytes) VALUES (?1, 'bpm_key', 'queued', 10, ?2, ?3, ?4)",
        params![job_id, format!("{{\"algorithmVersion\":\"{}\"}}", audio_analysis::ALGORITHM_VERSION), files.len() as u64, total_bytes],
    )?;
    for (track_file_id, _) in files {
        transaction.execute(
            "INSERT INTO background_job_items(id, job_id, track_file_id, state) VALUES (?1, ?2, ?3, 'queued')",
            params![Uuid::new_v4().to_string(), job_id, track_file_id],
        )?;
    }
    transaction.commit()?;
    drop(connection);
    get_job(state, &job_id)?.ok_or_else(|| AppError::Task("Не удалось создать задание".into()))
}

pub fn list(state: &AppState) -> Result<Vec<AnalysisJob>, AppError> {
    let connection = state.connection()?;
    let mut statement = connection.prepare(
        "SELECT j.id, j.state, coalesce(j.total_items, 0), j.processed_items, (SELECT count(*) FROM background_job_items i WHERE i.job_id = j.id AND i.state = 'failed'), j.processed_bytes, coalesce(j.total_bytes, 0), j.created_at, j.error_json FROM background_jobs j WHERE j.job_type = 'bpm_key' ORDER BY j.created_at DESC LIMIT 20",
    )?;
    Ok(statement
        .query_map([], map_job)?
        .collect::<Result<Vec<_>, _>>()?)
}

pub fn run(state: &AppState, job_id: &str) -> Result<AnalysisJob, AppError> {
    {
        let connection = state.connection()?;
        let changed = connection.execute(
            "UPDATE background_jobs SET state = 'running', started_at = coalesce(started_at, CURRENT_TIMESTAMP), updated_at = CURRENT_TIMESTAMP WHERE id = ?1 AND job_type = 'bpm_key' AND state IN ('queued', 'paused', 'recovery_required')",
            [job_id],
        )?;
        if changed == 0 {
            return get_job(state, job_id)?
                .ok_or_else(|| AppError::Task("Задание не найдено".into()));
        }
    }

    loop {
        let job_state = current_state(state, job_id)?;
        if job_state == "paused" {
            break;
        }
        if job_state == "cancelled" {
            cancel_queued_items(state, job_id)?;
            finish_cancelled_job(state, job_id)?;
            break;
        }
        if job_state != "running" {
            break;
        }
        let Some(item) = next_item(state, job_id)? else {
            finish_job(state, job_id)?;
            break;
        };
        mark_item_running(state, &item.id)?;
        let result = analyze_item(&item);
        match result {
            Ok(analysis) => save_success(state, job_id, &item, &analysis)?,
            Err(error) => save_failure(state, job_id, &item, &error)?,
        }
    }
    get_job(state, job_id)?.ok_or_else(|| AppError::Task("Задание не найдено".into()))
}

pub fn pause(state: &AppState, job_id: &str) -> Result<(), AppError> {
    let connection = state.connection()?;
    connection.execute(
        "UPDATE background_jobs SET state = 'paused', updated_at = CURRENT_TIMESTAMP WHERE id = ?1 AND job_type = 'bpm_key' AND state IN ('queued', 'running', 'recovery_required')",
        [job_id],
    )?;
    Ok(())
}

pub fn cancel(state: &AppState, job_id: &str) -> Result<(), AppError> {
    let connection = state.connection()?;
    connection.execute(
        "UPDATE background_jobs SET state = 'cancelled', updated_at = CURRENT_TIMESTAMP WHERE id = ?1 AND job_type = 'bpm_key' AND state IN ('queued', 'running', 'paused', 'recovery_required')",
        [job_id],
    )?;
    Ok(())
}

fn analyze_item(item: &AnalysisItem) -> Result<crate::models::AudioAnalysisResult, String> {
    let path = Path::new(&item.path);
    verify_file_snapshot(item, path)?;
    let result = audio_analysis::analyze_file(path)?;
    verify_file_snapshot(item, path)?;
    Ok(result)
}

fn verify_file_snapshot(item: &AnalysisItem, path: &Path) -> Result<(), String> {
    let metadata = path.metadata().map_err(|error| error.to_string())?;
    let modified_at_ms = metadata
        .modified()
        .map_err(|error| error.to_string())?
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    if metadata.len() != item.size_bytes || modified_at_ms != item.modified_at_ms {
        return Err("Файл изменился во время анализа".into());
    }
    if let Some(expected) = &item.expected_identity {
        let actual = file_identity(path).map_err(|error| error.to_string())?;
        if &actual != expected {
            return Err("Физическая идентичность файла изменилась".into());
        }
    }
    Ok(())
}

fn next_item(state: &AppState, job_id: &str) -> Result<Option<AnalysisItem>, AppError> {
    let connection = state.connection()?;
    Ok(connection
        .query_row(
            "SELECT i.id, f.id, f.canonical_path, f.size_bytes, f.modified_at_ms, f.file_identity FROM background_job_items i JOIN track_files f ON f.id = i.track_file_id WHERE i.job_id = ?1 AND i.state = 'queued' ORDER BY i.created_at, i.id LIMIT 1",
            [job_id],
            |row| Ok(AnalysisItem { id: row.get(0)?, track_file_id: row.get(1)?, path: row.get(2)?, size_bytes: row.get(3)?, modified_at_ms: row.get(4)?, expected_identity: row.get(5)? }),
        )
        .optional()?)
}

fn mark_item_running(state: &AppState, item_id: &str) -> Result<(), AppError> {
    let connection = state.connection()?;
    connection.execute(
        "UPDATE background_job_items SET state = 'running', started_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE id = ?1 AND state = 'queued'",
        [item_id],
    )?;
    Ok(())
}

fn save_success(
    state: &AppState,
    job_id: &str,
    item: &AnalysisItem,
    result: &crate::models::AudioAnalysisResult,
) -> Result<(), AppError> {
    let mut connection = state.connection()?;
    let transaction = connection.transaction()?;
    transaction.execute(
        "INSERT INTO track_audio_analysis(track_file_id, bpm, bpm_score, bpm_algorithm_version, musical_key, key_score, key_algorithm_version, camelot, file_identity, size_bytes, modified_at_ms, error) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?4, ?7, ?8, ?9, ?10, NULL) ON CONFLICT(track_file_id) DO UPDATE SET bpm=excluded.bpm, bpm_score=excluded.bpm_score, bpm_algorithm_version=excluded.bpm_algorithm_version, musical_key=excluded.musical_key, key_score=excluded.key_score, key_algorithm_version=excluded.key_algorithm_version, camelot=excluded.camelot, file_identity=excluded.file_identity, size_bytes=excluded.size_bytes, modified_at_ms=excluded.modified_at_ms, analyzed_at=CURRENT_TIMESTAMP, error=NULL",
        params![item.track_file_id, result.bpm, result.bpm_score, result.algorithm_version, result.musical_key, result.key_score, result.camelot, item.expected_identity, item.size_bytes, item.modified_at_ms],
    )?;
    transaction.execute(
        "UPDATE background_job_items SET state = 'completed', processed_bytes = ?1, result_json = ?2, finished_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE id = ?3",
        params![item.size_bytes, serde_json::to_string(result).map_err(|error| AppError::Task(error.to_string()))?, item.id],
    )?;
    transaction.execute(
        "UPDATE background_jobs SET processed_items = processed_items + 1, processed_bytes = processed_bytes + ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
        params![item.size_bytes, job_id],
    )?;
    transaction.commit()?;
    Ok(())
}

fn save_failure(
    state: &AppState,
    job_id: &str,
    item: &AnalysisItem,
    error: &str,
) -> Result<(), AppError> {
    let mut connection = state.connection()?;
    let transaction = connection.transaction()?;
    transaction.execute(
        "UPDATE background_job_items SET state = 'failed', processed_bytes = ?1, error = ?2, finished_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE id = ?3",
        params![item.size_bytes, error, item.id],
    )?;
    transaction.execute(
        "UPDATE background_jobs SET processed_items = processed_items + 1, processed_bytes = processed_bytes + ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
        params![item.size_bytes, job_id],
    )?;
    transaction.commit()?;
    Ok(())
}

fn finish_job(state: &AppState, job_id: &str) -> Result<(), AppError> {
    let connection = state.connection()?;
    let failed: u64 = connection.query_row(
        "SELECT count(*) FROM background_job_items WHERE job_id = ?1 AND state = 'failed'",
        [job_id],
        |row| row.get(0),
    )?;
    connection.execute(
        "UPDATE background_jobs SET state = ?1, error_json = CASE WHEN ?2 > 0 THEN json_object('failedItems', ?2) ELSE NULL END, finished_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE id = ?3",
        params![if failed == 0 { "completed" } else { "failed" }, failed, job_id],
    )?;
    Ok(())
}

fn cancel_queued_items(state: &AppState, job_id: &str) -> Result<(), AppError> {
    let connection = state.connection()?;
    connection.execute(
        "UPDATE background_job_items SET state = 'cancelled', finished_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE job_id = ?1 AND state = 'queued'",
        [job_id],
    )?;
    Ok(())
}

fn finish_cancelled_job(state: &AppState, job_id: &str) -> Result<(), AppError> {
    let connection = state.connection()?;
    connection.execute(
        "UPDATE background_jobs SET finished_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
        [job_id],
    )?;
    Ok(())
}

fn current_state(state: &AppState, job_id: &str) -> Result<String, AppError> {
    let connection = state.connection()?;
    connection
        .query_row(
            "SELECT state FROM background_jobs WHERE id = ?1",
            [job_id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::Task("Задание не найдено".into()))
}

fn get_job(state: &AppState, job_id: &str) -> Result<Option<AnalysisJob>, AppError> {
    let connection = state.connection()?;
    Ok(connection
        .query_row(
            "SELECT j.id, j.state, coalesce(j.total_items, 0), j.processed_items, (SELECT count(*) FROM background_job_items i WHERE i.job_id = j.id AND i.state = 'failed'), j.processed_bytes, coalesce(j.total_bytes, 0), j.created_at, j.error_json FROM background_jobs j WHERE j.id = ?1 AND j.job_type = 'bpm_key'",
            [job_id],
            map_job,
        )
        .optional()?)
}

fn map_job(row: &rusqlite::Row<'_>) -> rusqlite::Result<AnalysisJob> {
    Ok(AnalysisJob {
        id: row.get(0)?,
        state: row.get(1)?,
        total_items: row.get(2)?,
        processed_items: row.get(3)?,
        failed_items: row.get(4)?,
        processed_bytes: row.get(5)?,
        total_bytes: row.get(6)?,
        created_at: row.get(7)?,
        error: row.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio_metadata::AudioMetadata;

    #[test]
    fn interrupted_analysis_job_becomes_recoverable() {
        let state = AppState::in_memory().expect("database");
        let connection = state.connection().expect("connection");
        connection.execute("INSERT INTO background_jobs(id, job_type, state, total_items) VALUES ('job', 'bpm_key', 'running', 1)", []).expect("job");
        drop(connection);
        recover_supported_jobs(&state).expect("recover");
        assert_eq!(list(&state).expect("jobs")[0].state, "recovery_required");
    }

    #[test]
    fn failed_audio_file_does_not_leave_the_job_running() {
        let root = std::env::temp_dir().join(format!("sorttrack-analysis-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("root");
        let path = root.join("Broken.mp3");
        std::fs::write(&path, b"not an mp3").expect("fixture");
        let metadata = path.metadata().expect("metadata");
        let modified_at_ms = metadata
            .modified()
            .expect("modified")
            .duration_since(std::time::UNIX_EPOCH)
            .expect("epoch")
            .as_millis() as i64;
        let state = AppState::in_memory().expect("database");
        let library_root = state
            .register_root(&root.to_string_lossy())
            .expect("library root");
        state
            .upsert_file(
                &library_root,
                &path.to_string_lossy(),
                "Broken.mp3",
                "mp3",
                metadata.len(),
                modified_at_ms,
                &file_identity(&path).expect("identity"),
                &AudioMetadata::default(),
                "available",
            )
            .expect("track");
        let track_id = state.list_tracks(1, 0, "Broken").expect("track")[0]
            .id
            .clone();
        let job = enqueue(&state, &[track_id]).expect("enqueue");
        let finished = run(&state, &job.id).expect("run");
        assert_eq!(finished.state, "failed");
        assert_eq!(finished.failed_items, 1);
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
