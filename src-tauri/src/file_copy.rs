use std::{
    collections::{HashMap, HashSet},
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use rusqlite::{OptionalExtension, params};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    error::AppError,
    models::{
        CopyPlan, CopyPlanItem, CopyRecoveryItem, CopyRecoveryOperation, CopyRecoveryReport,
        CopyRecoveryReportItem, CopyReport, CopyReportItem, CopyResolution,
    },
    repository::AppState,
};

#[derive(Debug)]
struct CopySource {
    track_file_id: String,
    track_title: String,
    source_path: String,
    filename: String,
    size_bytes: u64,
}

#[derive(Debug)]
struct WorkItem {
    id: String,
    track_title: String,
    source_path: String,
    destination_path: String,
    expected_size: u64,
    expected_modified_at_ms: i64,
    state: String,
    planned_status: String,
    chosen_action: Option<String>,
    temporary_path: Option<String>,
}

pub fn plan_playlist_copy(
    state: &AppState,
    playlist_id: &str,
    destination_root: &str,
) -> Result<CopyPlan, AppError> {
    let destination = PathBuf::from(destination_root).canonicalize()?;
    if !destination.is_dir() {
        return Err(AppError::InvalidPlaylist(
            "copy destination is not a directory".into(),
        ));
    }
    let sources = load_playlist_sources(state, playlist_id)?;
    if sources.is_empty() {
        return Err(AppError::InvalidPlaylist("playlist has no tracks".into()));
    }

    let operation_id = Uuid::new_v4().to_string();
    let mut seen_destinations = HashSet::new();
    let mut persisted = Vec::with_capacity(sources.len());
    let mut items = Vec::with_capacity(sources.len());
    let mut required_bytes = 0u64;

    for source in sources {
        let item_id = Uuid::new_v4().to_string();
        let source_path = PathBuf::from(&source.source_path);
        let mut destination_path = destination.join(&source.filename);
        let mut status = "ready".to_owned();
        let mut reason = "Готово к копированию".to_owned();

        if !is_safe_windows_filename(&source.filename) {
            status = "invalid".into();
            reason = "Недопустимое имя файла Windows".into();
        } else if !source_path.is_file() {
            status = "missing".into();
            reason = "Исходный файл отсутствует".into();
        } else {
            let key = path_key(&destination_path);
            if !seen_destinations.insert(key) {
                destination_path = unique_copy_path(&destination_path, &seen_destinations);
                seen_destinations.insert(path_key(&destination_path));
                reason = "Имя изменено: в плейлисте есть другой файл с таким именем".into();
            }

            if destination_path.exists() {
                match (sha256_file(&source_path), sha256_file(&destination_path)) {
                    (Ok(source_hash), Ok(destination_hash)) if source_hash == destination_hash => {
                        status = "identical".into();
                        reason = "Идентичный файл уже существует (SHA-256 подтверждён)".into();
                    }
                    (Ok(_), Ok(_)) => {
                        status = "conflict".into();
                        reason = "В папке уже есть другой файл с таким именем".into();
                    }
                    _ => {
                        status = "permission_error".into();
                        reason = "Не удалось прочитать существующий файл для сравнения".into();
                    }
                }
            }
        }

        if matches!(status.as_str(), "ready" | "conflict") {
            required_bytes = required_bytes.saturating_add(source.size_bytes);
        }
        persisted.push((
            item_id.clone(),
            source.track_file_id,
            source.source_path.clone(),
            destination_path.to_string_lossy().into_owned(),
            status.clone(),
            reason.clone(),
            source.size_bytes,
        ));
        items.push(CopyPlanItem {
            id: item_id,
            track_title: source.track_title,
            source_path: source.source_path,
            destination_path: destination_path.to_string_lossy().into_owned(),
            size_bytes: source.size_bytes,
            status,
            reason,
        });
    }

    {
        let mut connection = state.connection()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO file_operations(id, operation_type, destination_root, state, policy_json) VALUES (?1, 'copy', ?2, 'planned', '{}')",
            params![operation_id, destination.to_string_lossy()],
        )?;
        for (id, track_file_id, source_path, destination_path, status, reason, expected_size) in
            &persisted
        {
            transaction.execute(
                "INSERT INTO file_operation_items(id, operation_id, track_file_id, source_path, destination_path, state, reason, planned_status, expected_size_bytes, expected_modified_at_ms) SELECT ?1, ?2, ?3, ?4, ?5, 'planned', ?6, ?7, ?8, modified_at_ms FROM track_files WHERE id = ?3",
                params![id, operation_id, track_file_id, source_path, destination_path, reason, status, expected_size],
            )?;
        }
        transaction.commit()?;
    }

    Ok(CopyPlan {
        operation_id,
        destination_root: destination.to_string_lossy().into_owned(),
        total_files: items.len() as u64,
        total_bytes: items.iter().map(|item| item.size_bytes).sum(),
        required_bytes,
        available_bytes: fs2::available_space(&destination).unwrap_or(0),
        items,
    })
}

pub fn execute_playlist_copy(
    state: &AppState,
    operation_id: &str,
    resolutions: &[CopyResolution],
) -> Result<CopyReport, AppError> {
    let resolution_by_item: HashMap<&str, &str> = resolutions
        .iter()
        .map(|resolution| (resolution.item_id.as_str(), resolution.action.as_str()))
        .collect();
    let (operation_state, mut items) = load_work_items(state, operation_id)?;
    if operation_state != "planned" {
        return Err(AppError::InvalidPlaylist(
            "copy operation is not in planned state".into(),
        ));
    }
    for item in &items {
        if item.planned_status == "conflict"
            && !matches!(
                resolution_by_item.get(item.id.as_str()),
                Some(&"skip" | &"rename")
            )
        {
            return Err(AppError::InvalidPlaylist(format!(
                "conflict {} has no safe resolution",
                item.id
            )));
        }
    }
    persist_copy_resolutions(state, operation_id, &items, &resolution_by_item)?;

    let mut report = CopyReport {
        operation_id: operation_id.to_owned(),
        copied: 0,
        used_existing: 0,
        skipped: 0,
        failed: 0,
        bytes_copied: 0,
        items: Vec::with_capacity(items.len()),
    };

    for item in &mut items {
        let action = resolution_by_item.get(item.id.as_str()).copied();
        let result = match item.planned_status.as_str() {
            "identical" => use_existing(state, item),
            "conflict" if action == Some("skip") => {
                mark_item_completed(state, &item.id, 0, "skipped", None, None)?;
                Ok(("skipped", "Пропущено по решению пользователя", 0))
            }
            "conflict" if action == Some("rename") => {
                let reserved = HashSet::new();
                let renamed = unique_copy_path(Path::new(&item.destination_path), &reserved);
                item.destination_path = renamed.to_string_lossy().into_owned();
                update_item_destination(state, &item.id, &item.destination_path)?;
                copy_one(state, operation_id, item)
            }
            "ready" => copy_one(state, operation_id, item),
            "missing" | "invalid" | "permission_error" => {
                mark_item_failed(state, &item.id, &item.planned_status)?;
                Err(item.planned_status.clone())
            }
            _ => Err("unsupported copy plan state".into()),
        };

        match result {
            Ok(("copied", reason, bytes)) => {
                report.copied += 1;
                report.bytes_copied += bytes;
                report.items.push(report_item(item, "copied", reason));
            }
            Ok(("used_existing", reason, _)) => {
                report.used_existing += 1;
                report
                    .items
                    .push(report_item(item, "used_existing", reason));
            }
            Ok((_, reason, _)) => {
                report.skipped += 1;
                report.items.push(report_item(item, "skipped", reason));
            }
            Err(reason) => {
                report.failed += 1;
                report.items.push(report_item(item, "failed", &reason));
            }
        }
    }

    let unresolved = count_unresolved_items(state, operation_id)?;
    update_operation_state(
        state,
        operation_id,
        if unresolved > 0 {
            "recovery_required"
        } else if report.failed == 0 {
            "completed"
        } else {
            "completed_with_errors"
        },
    )?;
    Ok(report)
}

fn persist_copy_resolutions(
    state: &AppState,
    operation_id: &str,
    items: &[WorkItem],
    resolutions: &HashMap<&str, &str>,
) -> Result<(), AppError> {
    let mut connection = state.connection()?;
    let transaction = connection.transaction()?;
    for item in items {
        let action = match item.planned_status.as_str() {
            "conflict" => resolutions.get(item.id.as_str()).copied(),
            "ready" => Some("copy"),
            "identical" => Some("use_existing"),
            _ => Some("record_error"),
        };
        transaction.execute(
            "UPDATE file_operation_items SET chosen_action = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2 AND operation_id = ?3",
            params![action, item.id, operation_id],
        )?;
    }
    transaction.execute(
        "UPDATE file_operations SET state = 'running', confirmed_at = CURRENT_TIMESTAMP WHERE id = ?1 AND state = 'planned'",
        [operation_id],
    )?;
    transaction.commit()?;
    Ok(())
}

pub fn list_copy_recoveries(state: &AppState) -> Result<Vec<CopyRecoveryOperation>, AppError> {
    let connection = state.connection()?;
    connection.execute(
        "UPDATE file_operations SET state = 'recovery_required' WHERE state = 'running'",
        [],
    )?;
    let mut operation_statement = connection.prepare(
        "SELECT id, destination_root, state, created_at FROM file_operations WHERE state = 'recovery_required' ORDER BY created_at",
    )?;
    let operations = operation_statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(operation_statement);

    let mut result = Vec::with_capacity(operations.len());
    for (operation_id, destination_root, operation_state, created_at) in operations {
        let mut item_statement = connection.prepare(
            "SELECT oi.id, coalesce(m.title, t.preferred_title, f.filename), oi.source_path, oi.destination_path, oi.temporary_path, oi.state, coalesce(oi.reason, '') FROM file_operation_items oi JOIN track_files f ON f.id = oi.track_file_id JOIN tracks t ON t.id = f.track_id LEFT JOIN track_metadata m ON m.track_file_id = f.id WHERE oi.operation_id = ?1 ORDER BY oi.created_at, oi.id",
        )?;
        let all_items = item_statement
            .query_map([&operation_id], |row| {
                let destination_path: String = row.get(3)?;
                let temporary_path: Option<String> = row.get(4)?;
                let item_state: String = row.get(5)?;
                let destination_exists = Path::new(&destination_path).is_file();
                let temporary_exists = temporary_path
                    .as_deref()
                    .is_some_and(|path| Path::new(path).is_file());
                let recommended_action = if destination_exists {
                    "Проверить уже созданный итоговый файл"
                } else if temporary_exists {
                    "Удалить служебную копию и безопасно повторить"
                } else if item_state == "planned" {
                    "Продолжить подтверждённый план"
                } else {
                    "Повторить безопасное копирование"
                };
                Ok(CopyRecoveryItem {
                    id: row.get(0)?,
                    track_title: row.get(1)?,
                    source_path: row.get(2)?,
                    destination_path,
                    temporary_path,
                    state: item_state,
                    reason: row.get(6)?,
                    recommended_action: recommended_action.into(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let total_items = all_items.len() as u64;
        let completed_items = all_items
            .iter()
            .filter(|item| {
                matches!(
                    item.state.as_str(),
                    "completed" | "cancelled_safe" | "failed_final"
                )
            })
            .count() as u64;
        let items = all_items
            .into_iter()
            .filter(|item| {
                !matches!(
                    item.state.as_str(),
                    "completed" | "cancelled_safe" | "failed_final"
                )
            })
            .collect::<Vec<_>>();
        result.push(CopyRecoveryOperation {
            operation_id,
            destination_root,
            state: operation_state,
            created_at,
            total_items,
            completed_items,
            pending_items: items.len() as u64,
            items,
        });
    }
    Ok(result)
}

pub fn recover_copy_operation(
    state: &AppState,
    operation_id: &str,
    action: &str,
) -> Result<CopyRecoveryReport, AppError> {
    if !matches!(action, "resume" | "cancel_safe") {
        return Err(AppError::InvalidPlaylist(
            "unsupported recovery action".into(),
        ));
    }
    let (operation_state, mut items) = load_work_items(state, operation_id)?;
    if operation_state != "recovery_required" {
        return Err(AppError::InvalidPlaylist(
            "copy operation does not require recovery".into(),
        ));
    }
    let mut report = CopyRecoveryReport {
        operation_id: operation_id.to_owned(),
        recovered: 0,
        cancelled: 0,
        failed: 0,
        items: Vec::new(),
    };

    for item in &mut items {
        if matches!(
            item.state.as_str(),
            "completed" | "cancelled_safe" | "failed_final"
        ) {
            continue;
        }
        let result = if action == "resume" {
            resume_recovery_item(state, operation_id, item)
        } else {
            cancel_recovery_item(state, operation_id, item)
        };
        match result {
            Ok((result_kind, reason)) => {
                if result_kind == "recovered" {
                    report.recovered += 1;
                } else {
                    report.cancelled += 1;
                }
                report
                    .items
                    .push(recovery_report_item(item, result_kind, reason));
            }
            Err(reason) => {
                report.failed += 1;
                report
                    .items
                    .push(recovery_report_item(item, "failed", &reason));
            }
        }
    }

    let unresolved = count_unresolved_items(state, operation_id)?;
    update_operation_state(
        state,
        operation_id,
        if unresolved > 0 {
            "recovery_required"
        } else if action == "cancel_safe" {
            "cancelled"
        } else if report.failed > 0 {
            "completed_with_errors"
        } else {
            "completed"
        },
    )?;
    Ok(report)
}

fn resume_recovery_item(
    state: &AppState,
    operation_id: &str,
    item: &mut WorkItem,
) -> Result<(&'static str, &'static str), String> {
    let destination = Path::new(&item.destination_path);
    let temporary_exists = item
        .temporary_path
        .as_deref()
        .is_some_and(|path| Path::new(path).is_file());
    if destination.is_file() {
        if temporary_exists {
            mark_item_recovery(state, &item.id, "destination_and_temporary_both_exist")
                .map_err(|error| error.to_string())?;
            return Err("Найдены и итоговый, и временный файл; требуется ручная проверка".into());
        }
        verify_committed_destination(state, item)?;
        return Ok((
            "recovered",
            "Уже созданный файл проверен по SHA-256 и принят",
        ));
    }
    if temporary_exists {
        remove_owned_temporary(operation_id, item)?;
    }
    reset_item_for_retry(state, &item.id).map_err(|error| error.to_string())?;
    item.state = "planned".into();
    item.temporary_path = None;
    match (item.planned_status.as_str(), item.chosen_action.as_deref()) {
        ("ready", Some("copy")) | ("conflict", Some("rename")) => {
            copy_one(state, operation_id, item)?;
            Ok(("recovered", "Копирование безопасно выполнено повторно"))
        }
        ("identical", Some("use_existing")) => {
            use_existing(state, item)?;
            Ok((
                "recovered",
                "Идентичный существующий файл повторно проверен",
            ))
        }
        ("conflict", Some("skip")) => {
            mark_item_completed(state, &item.id, 0, "skipped", None, None)
                .map_err(|error| error.to_string())?;
            Ok((
                "cancelled",
                "Файл пропущен согласно подтверждённому решению",
            ))
        }
        _ => {
            mark_item_recovery(state, &item.id, "confirmed_action_missing")
                .map_err(|error| error.to_string())?;
            Err("В журнале нет подтверждённого безопасного действия".into())
        }
    }
}

fn cancel_recovery_item(
    state: &AppState,
    operation_id: &str,
    item: &WorkItem,
) -> Result<(&'static str, &'static str), String> {
    let destination_exists = Path::new(&item.destination_path).is_file();
    let temporary_exists = item
        .temporary_path
        .as_deref()
        .is_some_and(|path| Path::new(path).is_file());
    if destination_exists {
        if temporary_exists {
            mark_item_recovery(state, &item.id, "destination_and_temporary_both_exist")
                .map_err(|error| error.to_string())?;
            return Err(
                "Ничего не удалено: итоговый и временный файлы требуют ручной проверки".into(),
            );
        }
        verify_committed_destination(state, item)?;
        return Ok(("recovered", "Готовый итоговый файл проверен и сохранён"));
    }
    if temporary_exists {
        remove_owned_temporary(operation_id, item)?;
    }
    mark_item_cancelled(state, &item.id).map_err(|error| error.to_string())?;
    Ok((
        "cancelled",
        "Служебная копия удалена; исходный файл не изменён",
    ))
}

fn verify_committed_destination(state: &AppState, item: &WorkItem) -> Result<(), String> {
    let source = Path::new(&item.source_path);
    let destination = Path::new(&item.destination_path);
    let source_metadata = source.metadata().map_err(|error| error.to_string())?;
    let modified_at_ms = source_metadata
        .modified()
        .map_err(|error| error.to_string())?
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    if source_metadata.len() != item.expected_size || modified_at_ms != item.expected_modified_at_ms
    {
        mark_item_recovery(state, &item.id, "source_changed_during_recovery")
            .map_err(|error| error.to_string())?;
        return Err("Исходный файл изменился; автоматическое подтверждение запрещено".into());
    }
    let source_hash = sha256_file(source).map_err(|error| error.to_string())?;
    let destination_hash = sha256_file(destination).map_err(|error| error.to_string())?;
    if source_hash != destination_hash {
        mark_item_recovery(state, &item.id, "destination_hash_mismatch")
            .map_err(|error| error.to_string())?;
        return Err("SHA-256 итогового файла не совпадает с источником".into());
    }
    mark_item_completed(
        state,
        &item.id,
        source_metadata.len(),
        "recovered_verified",
        Some(&source_hash),
        Some(&destination_hash),
    )
    .map_err(|error| error.to_string())
}

fn remove_owned_temporary(operation_id: &str, item: &WorkItem) -> Result<(), String> {
    let stored = item
        .temporary_path
        .as_deref()
        .ok_or_else(|| "В журнале отсутствует путь служебного файла".to_owned())?;
    let destination = Path::new(&item.destination_path);
    let expected = destination.with_file_name(format!(".sorttrack-{operation_id}-{}.tmp", item.id));
    if path_key(Path::new(stored)) != path_key(&expected) {
        return Err(
            "Путь служебного файла не принадлежит этой операции; удаление запрещено".into(),
        );
    }
    std::fs::remove_file(&expected).map_err(|error| error.to_string())
}

fn reset_item_for_retry(state: &AppState, item_id: &str) -> Result<(), AppError> {
    let connection = state.connection()?;
    connection.execute(
        "UPDATE file_operation_items SET state = 'planned', temporary_path = NULL, reason = 'recovery_retry', bytes_copied = 0, source_sha256 = NULL, destination_sha256 = NULL, updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
        [item_id],
    )?;
    Ok(())
}

fn mark_item_cancelled(state: &AppState, item_id: &str) -> Result<(), AppError> {
    let connection = state.connection()?;
    connection.execute(
        "UPDATE file_operation_items SET state = 'cancelled_safe', temporary_path = NULL, reason = 'cancelled_during_recovery', updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
        [item_id],
    )?;
    Ok(())
}

fn count_unresolved_items(state: &AppState, operation_id: &str) -> Result<u64, AppError> {
    let connection = state.connection()?;
    Ok(connection.query_row(
        "SELECT count(*) FROM file_operation_items WHERE operation_id = ?1 AND state NOT IN ('completed', 'failed_final', 'cancelled_safe')",
        [operation_id],
        |row| row.get(0),
    )?)
}

fn recovery_report_item(item: &WorkItem, result: &str, reason: &str) -> CopyRecoveryReportItem {
    CopyRecoveryReportItem {
        track_title: item.track_title.clone(),
        destination_path: item.destination_path.clone(),
        result: result.into(),
        reason: reason.into(),
    }
}

fn load_playlist_sources(state: &AppState, playlist_id: &str) -> Result<Vec<CopySource>, AppError> {
    let connection = state.connection()?;
    let exists: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM playlists WHERE id = ?1)",
        [playlist_id],
        |row| row.get(0),
    )?;
    if !exists {
        return Err(AppError::InvalidPlaylist("playlist not found".into()));
    }
    let mut statement = connection.prepare(
        "SELECT f.id, coalesce(m.title, t.preferred_title, f.filename), f.canonical_path, f.filename, f.size_bytes, f.modified_at_ms FROM playlist_items pi JOIN tracks t ON t.id = pi.track_id JOIN track_files f ON f.id = (SELECT preferred.id FROM track_files preferred WHERE preferred.track_id = t.id ORDER BY CASE preferred.availability_status WHEN 'available' THEN 0 ELSE 1 END, CASE lower(preferred.extension) WHEN 'flac' THEN 0 ELSE 1 END LIMIT 1) LEFT JOIN track_metadata m ON m.track_file_id = f.id WHERE pi.playlist_id = ?1 ORDER BY pi.position",
    )?;
    Ok(statement
        .query_map([playlist_id], |row| {
            Ok(CopySource {
                track_file_id: row.get(0)?,
                track_title: row.get(1)?,
                source_path: row.get(2)?,
                filename: row.get(3)?,
                size_bytes: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?)
}

fn load_work_items(
    state: &AppState,
    operation_id: &str,
) -> Result<(String, Vec<WorkItem>), AppError> {
    let connection = state.connection()?;
    let operation_state: String = connection
        .query_row(
            "SELECT state FROM file_operations WHERE id = ?1",
            [operation_id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::InvalidPlaylist("copy operation not found".into()))?;
    let mut statement = connection.prepare(
        "SELECT oi.id, coalesce(m.title, t.preferred_title, f.filename), oi.source_path, oi.destination_path, oi.expected_size_bytes, oi.expected_modified_at_ms, oi.state, oi.planned_status, oi.chosen_action, oi.temporary_path FROM file_operation_items oi JOIN track_files f ON f.id = oi.track_file_id JOIN tracks t ON t.id = f.track_id LEFT JOIN track_metadata m ON m.track_file_id = f.id WHERE oi.operation_id = ?1 ORDER BY oi.created_at, oi.id",
    )?;
    let items = statement
        .query_map([operation_id], |row| {
            Ok(WorkItem {
                id: row.get(0)?,
                track_title: row.get(1)?,
                source_path: row.get(2)?,
                destination_path: row.get(3)?,
                expected_size: row.get(4)?,
                expected_modified_at_ms: row.get(5)?,
                state: row.get(6)?,
                planned_status: row.get(7)?,
                chosen_action: row.get(8)?,
                temporary_path: row.get(9)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok((operation_state, items))
}

fn copy_one(
    state: &AppState,
    operation_id: &str,
    item: &WorkItem,
) -> Result<(&'static str, &'static str, u64), String> {
    let source_path = Path::new(&item.source_path);
    let destination_path = Path::new(&item.destination_path);
    let source_metadata = source_path.metadata().map_err(|error| error.to_string())?;
    let modified_at_ms = source_metadata
        .modified()
        .map_err(|error| error.to_string())?
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    if source_metadata.len() != item.expected_size || modified_at_ms != item.expected_modified_at_ms
    {
        mark_item_failed(state, &item.id, "source_changed").map_err(|error| error.to_string())?;
        return Err("Исходный файл изменился после сканирования".into());
    }
    if destination_path.exists() {
        mark_item_failed(state, &item.id, "destination_appeared")
            .map_err(|error| error.to_string())?;
        return Err("Итоговый файл появился после подтверждения; перезапись отменена".into());
    }

    let temporary_path =
        destination_path.with_file_name(format!(".sorttrack-{operation_id}-{}.tmp", item.id));
    set_item_state(state, &item.id, "copying_to_temp", Some(&temporary_path))
        .map_err(|error| error.to_string())?;

    let mut temporary_created = false;
    let copy_result = (|| -> Result<(u64, String, String), String> {
        let mut source = File::open(source_path).map_err(|error| error.to_string())?;
        let mut temporary = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
            .map_err(|error| error.to_string())?;
        temporary_created = true;
        let mut buffer = vec![0u8; 1024 * 1024];
        let mut bytes_copied = 0u64;
        let mut source_hasher = Sha256::new();
        loop {
            let read = source
                .read(&mut buffer)
                .map_err(|error| error.to_string())?;
            if read == 0 {
                break;
            }
            temporary
                .write_all(&buffer[..read])
                .map_err(|error| error.to_string())?;
            source_hasher.update(&buffer[..read]);
            bytes_copied += read as u64;
        }
        temporary.sync_all().map_err(|error| error.to_string())?;
        drop(temporary);
        set_item_state(state, &item.id, "temp_copied", Some(&temporary_path))
            .map_err(|error| error.to_string())?;
        let destination_hash = sha256_file(&temporary_path).map_err(|error| error.to_string())?;
        let source_hash = digest_hex(source_hasher.finalize().as_slice());
        let temporary_size = temporary_path
            .metadata()
            .map_err(|error| error.to_string())?
            .len();
        if temporary_size != item.expected_size
            || bytes_copied != item.expected_size
            || source_hash != destination_hash
        {
            return Err("Проверка временной копии не пройдена".into());
        }
        mark_item_verified(
            state,
            &item.id,
            bytes_copied,
            &source_hash,
            &destination_hash,
        )
        .map_err(|error| error.to_string())?;
        if destination_path.exists() {
            return Err("Итоговый файл появился во время копирования".into());
        }
        std::fs::rename(&temporary_path, destination_path).map_err(|error| error.to_string())?;
        set_item_state(state, &item.id, "destination_committed", None)
            .map_err(|error| error.to_string())?;
        mark_item_completed(
            state,
            &item.id,
            bytes_copied,
            "copied",
            Some(&source_hash),
            Some(&destination_hash),
        )
        .map_err(|error| error.to_string())?;
        Ok((bytes_copied, source_hash, destination_hash))
    })();

    match copy_result {
        Ok((bytes, _, _)) => Ok(("copied", "Файл скопирован и проверен", bytes)),
        Err(error) => {
            if temporary_created
                && temporary_path.exists()
                && std::fs::remove_file(&temporary_path).is_err()
            {
                let _ = mark_item_recovery(state, &item.id, "temporary_cleanup_failed");
                return Err(format!("{error}; временный файл требует восстановления"));
            }
            if destination_path.exists() && !temporary_path.exists() {
                let _ = mark_item_recovery(state, &item.id, "commit_state_uncertain");
            } else {
                let _ = mark_item_failed(state, &item.id, &error);
            }
            Err(error)
        }
    }
}

fn use_existing(
    state: &AppState,
    item: &WorkItem,
) -> Result<(&'static str, &'static str, u64), String> {
    let source_hash =
        sha256_file(Path::new(&item.source_path)).map_err(|error| error.to_string())?;
    let destination_hash =
        sha256_file(Path::new(&item.destination_path)).map_err(|error| error.to_string())?;
    if source_hash != destination_hash {
        mark_item_failed(state, &item.id, "existing_file_changed")
            .map_err(|error| error.to_string())?;
        return Err("Существующий файл изменился после планирования".into());
    }
    mark_item_completed(
        state,
        &item.id,
        0,
        "use_existing",
        Some(&source_hash),
        Some(&destination_hash),
    )
    .map_err(|error| error.to_string())?;
    Ok((
        "used_existing",
        "Использован идентичный существующий файл",
        0,
    ))
}

fn update_operation_state(
    state: &AppState,
    operation_id: &str,
    operation_state: &str,
) -> Result<(), AppError> {
    let connection = state.connection()?;
    connection.execute(
        "UPDATE file_operations SET state = ?1, confirmed_at = CASE WHEN ?1 = 'running' THEN CURRENT_TIMESTAMP ELSE confirmed_at END, finished_at = CASE WHEN ?1 IN ('completed', 'completed_with_errors', 'cancelled') THEN CURRENT_TIMESTAMP ELSE finished_at END WHERE id = ?2",
        params![operation_state, operation_id],
    )?;
    Ok(())
}

fn set_item_state(
    state: &AppState,
    item_id: &str,
    item_state: &str,
    temporary_path: Option<&Path>,
) -> Result<(), AppError> {
    let connection = state.connection()?;
    connection.execute(
        "UPDATE file_operation_items SET state = ?1, temporary_path = coalesce(?2, temporary_path), updated_at = CURRENT_TIMESTAMP WHERE id = ?3",
        params![item_state, temporary_path.map(|path| path.to_string_lossy()), item_id],
    )?;
    Ok(())
}

fn mark_item_verified(
    state: &AppState,
    item_id: &str,
    bytes: u64,
    source_hash: &str,
    destination_hash: &str,
) -> Result<(), AppError> {
    let connection = state.connection()?;
    connection.execute(
        "UPDATE file_operation_items SET state = 'verified', bytes_copied = ?1, source_sha256 = ?2, destination_sha256 = ?3, updated_at = CURRENT_TIMESTAMP WHERE id = ?4",
        params![bytes, source_hash, destination_hash, item_id],
    )?;
    Ok(())
}

fn mark_item_completed(
    state: &AppState,
    item_id: &str,
    bytes: u64,
    reason: &str,
    source_hash: Option<&str>,
    destination_hash: Option<&str>,
) -> Result<(), AppError> {
    let connection = state.connection()?;
    connection.execute(
        "UPDATE file_operation_items SET state = 'completed', reason = ?1, bytes_copied = ?2, source_sha256 = coalesce(?3, source_sha256), destination_sha256 = coalesce(?4, destination_sha256), updated_at = CURRENT_TIMESTAMP WHERE id = ?5",
        params![reason, bytes, source_hash, destination_hash, item_id],
    )?;
    Ok(())
}

fn mark_item_failed(state: &AppState, item_id: &str, reason: &str) -> Result<(), AppError> {
    let connection = state.connection()?;
    connection.execute(
        "UPDATE file_operation_items SET state = 'failed_final', reason = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
        params![reason, item_id],
    )?;
    Ok(())
}

fn mark_item_recovery(state: &AppState, item_id: &str, reason: &str) -> Result<(), AppError> {
    let connection = state.connection()?;
    connection.execute(
        "UPDATE file_operation_items SET state = 'recovery_required', reason = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
        params![reason, item_id],
    )?;
    Ok(())
}

fn update_item_destination(
    state: &AppState,
    item_id: &str,
    destination: &str,
) -> Result<(), AppError> {
    let connection = state.connection()?;
    connection.execute(
        "UPDATE file_operation_items SET destination_path = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
        params![destination, item_id],
    )?;
    Ok(())
}

fn report_item(item: &WorkItem, result: &str, reason: &str) -> CopyReportItem {
    CopyReportItem {
        track_title: item.track_title.clone(),
        destination_path: item.destination_path.clone(),
        result: result.to_owned(),
        reason: reason.to_owned(),
    }
}

pub(crate) fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(digest_hex(hasher.finalize().as_slice()))
}

fn digest_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn path_key(path: &Path) -> String {
    path.to_string_lossy()
        .replace('/', "\\")
        .to_ascii_lowercase()
}

fn unique_copy_path(path: &Path, reserved: &HashSet<String>) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("track");
    let extension = path.extension().and_then(|value| value.to_str());
    for index in 1..10000 {
        let suffix = if index == 1 {
            " (copy)".to_owned()
        } else {
            format!(" (copy {index})")
        };
        let filename = match extension {
            Some(extension) => format!("{stem}{suffix}.{extension}"),
            None => format!("{stem}{suffix}"),
        };
        let candidate = parent.join(filename);
        if !candidate.exists() && !reserved.contains(&path_key(&candidate)) {
            return candidate;
        }
    }
    parent.join(format!("{stem} (copy {}).tmp", Uuid::new_v4()))
}

fn is_safe_windows_filename(filename: &str) -> bool {
    let lowercase = filename.to_ascii_lowercase();
    let stem = lowercase.split('.').next().unwrap_or_default();
    let reserved = matches!(stem, "con" | "prn" | "aux" | "nul")
        || (stem.len() == 4
            && (stem.starts_with("com") || stem.starts_with("lpt"))
            && matches!(stem.as_bytes()[3], b'1'..=b'9'));
    !filename.is_empty()
        && !filename.ends_with(['.', ' '])
        && !reserved
        && !filename
            .chars()
            .any(|character| character < ' ' || "<>:\"/\\|?*".contains(character))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio_metadata::AudioMetadata;

    fn copy_test_state() -> (AppState, PathBuf, PathBuf, String) {
        let root = std::env::temp_dir().join(format!("sorttrack-copy-{}", Uuid::new_v4()));
        let source_dir = root.join("source");
        let destination_dir = root.join("destination");
        std::fs::create_dir_all(&source_dir).expect("source folder");
        std::fs::create_dir_all(&destination_dir).expect("destination folder");
        let source_path = source_dir.join("Artist - Track.mp3");
        std::fs::write(&source_path, b"sorttrack safe copy fixture").expect("source file");
        let metadata = source_path.metadata().expect("metadata");
        let modified_at_ms = metadata
            .modified()
            .expect("modified")
            .duration_since(UNIX_EPOCH)
            .expect("epoch")
            .as_millis() as i64;
        let state = AppState::in_memory().expect("database");
        let library_root = state
            .register_root(&source_dir.to_string_lossy())
            .expect("library root");
        state
            .upsert_file(
                &library_root,
                &source_path.to_string_lossy(),
                "Artist - Track.mp3",
                "mp3",
                metadata.len(),
                modified_at_ms,
                &crate::file_identity::file_identity(&source_path).expect("file identity"),
                &AudioMetadata::default(),
                "available",
            )
            .expect("track");
        let track_id = state.list_tracks(1, 0, "").expect("tracks")[0].id.clone();
        let playlist_id = state.create_playlist("Copy test").expect("playlist");
        state
            .place_track_in_playlist(&playlist_id, 0, &track_id, None, None)
            .expect("playlist item");
        (state, root, destination_dir, playlist_id)
    }

    #[test]
    fn creates_non_overwriting_copy_names() {
        let root = std::env::temp_dir().join(format!("sorttrack-copy-name-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("folder");
        let original = root.join("Track.flac");
        std::fs::write(&original, b"existing").expect("existing");
        let candidate = unique_copy_path(&original, &HashSet::new());
        assert_eq!(
            candidate.file_name().and_then(|value| value.to_str()),
            Some("Track (copy).flac")
        );
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn rejects_reserved_windows_names() {
        assert!(!is_safe_windows_filename("CON.mp3"));
        assert!(!is_safe_windows_filename("bad:name.flac"));
        assert!(is_safe_windows_filename("Artist - Track.flac"));
    }

    #[test]
    fn copies_through_temp_and_verifies_hash() {
        let (state, root, destination, playlist_id) = copy_test_state();
        let plan =
            plan_playlist_copy(&state, &playlist_id, &destination.to_string_lossy()).expect("plan");
        assert_eq!(plan.items[0].status, "ready");
        let report = execute_playlist_copy(&state, &plan.operation_id, &[]).expect("execute");
        assert_eq!(report.copied, 1);
        assert_eq!(
            std::fs::read(destination.join("Artist - Track.mp3")).expect("copied file"),
            b"sorttrack safe copy fixture"
        );
        assert!(
            std::fs::read_dir(&destination)
                .expect("destination entries")
                .all(|entry| !entry
                    .expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".sorttrack-"))
        );
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn never_overwrites_a_conflicting_destination() {
        let (state, root, destination, playlist_id) = copy_test_state();
        let existing = destination.join("Artist - Track.mp3");
        std::fs::write(&existing, b"do not overwrite").expect("conflicting file");
        let plan =
            plan_playlist_copy(&state, &playlist_id, &destination.to_string_lossy()).expect("plan");
        assert_eq!(plan.items[0].status, "conflict");
        assert!(execute_playlist_copy(&state, &plan.operation_id, &[]).is_err());
        assert_eq!(
            std::fs::read(&existing).expect("original"),
            b"do not overwrite"
        );

        let report = execute_playlist_copy(
            &state,
            &plan.operation_id,
            &[CopyResolution {
                item_id: plan.items[0].id.clone(),
                action: "rename".into(),
            }],
        )
        .expect("rename copy");
        assert_eq!(report.copied, 1);
        assert_eq!(
            std::fs::read(&existing).expect("original"),
            b"do not overwrite"
        );
        assert_eq!(
            std::fs::read(destination.join("Artist - Track (copy).mp3")).expect("renamed copy"),
            b"sorttrack safe copy fixture"
        );
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn refuses_to_copy_a_source_changed_after_planning() {
        let (state, root, destination, playlist_id) = copy_test_state();
        let plan =
            plan_playlist_copy(&state, &playlist_id, &destination.to_string_lossy()).expect("plan");
        let source = root.join("source").join("Artist - Track.mp3");
        std::fs::write(&source, b"changed after the preview and confirmation").expect("change");
        let report = execute_playlist_copy(&state, &plan.operation_id, &[]).expect("report");
        assert_eq!(report.failed, 1);
        assert!(!destination.join("Artist - Track.mp3").exists());
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn resumes_after_a_crash_with_a_partial_owned_temporary_file() {
        let (state, root, destination, playlist_id) = copy_test_state();
        let plan =
            plan_playlist_copy(&state, &playlist_id, &destination.to_string_lossy()).expect("plan");
        let item = &plan.items[0];
        let temporary = PathBuf::from(&item.destination_path)
            .with_file_name(format!(".sorttrack-{}-{}.tmp", plan.operation_id, item.id));
        std::fs::write(&temporary, b"partial").expect("partial temporary file");
        {
            let connection = state.connection().expect("connection");
            connection
                .execute(
                    "UPDATE file_operations SET state = 'running' WHERE id = ?1",
                    [&plan.operation_id],
                )
                .expect("running operation");
            connection
                .execute(
                    "UPDATE file_operation_items SET state = 'copying_to_temp', chosen_action = 'copy', temporary_path = ?1 WHERE id = ?2",
                    params![temporary.to_string_lossy(), item.id],
                )
                .expect("interrupted item");
        }

        let recoveries = list_copy_recoveries(&state).expect("recovery list");
        assert_eq!(recoveries.len(), 1);
        assert_eq!(recoveries[0].pending_items, 1);
        let report =
            recover_copy_operation(&state, &plan.operation_id, "resume").expect("resume recovery");
        assert_eq!(report.recovered, 1, "{report:?}");
        assert_eq!(report.failed, 0);
        assert!(!temporary.exists());
        assert_eq!(
            std::fs::read(destination.join("Artist - Track.mp3")).expect("destination"),
            b"sorttrack safe copy fixture"
        );
        assert!(list_copy_recoveries(&state).expect("resolved").is_empty());
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn accepts_a_committed_file_after_crash_only_after_hash_verification() {
        let (state, root, destination, playlist_id) = copy_test_state();
        let plan =
            plan_playlist_copy(&state, &playlist_id, &destination.to_string_lossy()).expect("plan");
        let item = &plan.items[0];
        std::fs::copy(&item.source_path, &item.destination_path).expect("committed destination");
        {
            let connection = state.connection().expect("connection");
            connection
                .execute(
                    "UPDATE file_operations SET state = 'recovery_required' WHERE id = ?1",
                    [&plan.operation_id],
                )
                .expect("recovery operation");
            connection
                .execute(
                    "UPDATE file_operation_items SET state = 'destination_committed', chosen_action = 'copy' WHERE id = ?1",
                    [&item.id],
                )
                .expect("committed item");
        }

        let report = recover_copy_operation(&state, &plan.operation_id, "resume")
            .expect("verify committed destination");
        assert_eq!(report.recovered, 1);
        assert_eq!(report.failed, 0);
        assert_eq!(
            std::fs::read(&item.destination_path).expect("destination"),
            b"sorttrack safe copy fixture"
        );
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn safe_cancel_never_deletes_an_unowned_temporary_path() {
        let (state, root, destination, playlist_id) = copy_test_state();
        let plan =
            plan_playlist_copy(&state, &playlist_id, &destination.to_string_lossy()).expect("plan");
        let item = &plan.items[0];
        let unrelated = destination.join("unrelated.tmp");
        std::fs::write(&unrelated, b"must survive").expect("unrelated file");
        {
            let connection = state.connection().expect("connection");
            connection
                .execute(
                    "UPDATE file_operations SET state = 'recovery_required' WHERE id = ?1",
                    [&plan.operation_id],
                )
                .expect("recovery operation");
            connection
                .execute(
                    "UPDATE file_operation_items SET state = 'copying_to_temp', chosen_action = 'copy', temporary_path = ?1 WHERE id = ?2",
                    params![unrelated.to_string_lossy(), item.id],
                )
                .expect("foreign temporary path");
        }

        let report = recover_copy_operation(&state, &plan.operation_id, "cancel_safe")
            .expect("safe cancellation report");
        assert_eq!(report.failed, 1);
        assert_eq!(
            std::fs::read(&unrelated).expect("unrelated survives"),
            b"must survive"
        );
        assert_eq!(
            list_copy_recoveries(&state)
                .expect("still unresolved")
                .len(),
            1
        );
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
