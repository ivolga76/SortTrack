use std::{
    collections::HashSet,
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use lofty::{
    config::WriteOptions,
    file::TaggedFileExt,
    probe::Probe,
    tag::{Accessor, ItemKey, Tag, TagExt, TagType},
};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    audio_metadata::{AudioMetadata, camelot_for_key},
    error::AppError,
    file_identity::file_identity,
    models::{
        BulkMetadataCommitItem, BulkMetadataPreview, BulkMetadataPreviewItem, BulkMetadataReport,
        BulkMetadataReportItem, BulkMetadataRequest, MetadataChange, MetadataHistoryEntry,
        MetadataPreview, MetadataValues, TrackMetadataDetail, TrackRatingHistoryEntry,
    },
    repository::AppState,
};

#[derive(Debug)]
struct FileRecord {
    file_id: String,
    track_id: String,
    path: PathBuf,
    filename: String,
    extension: String,
    root_path: PathBuf,
    size_bytes: u64,
    modified_at_ms: i64,
    identity: Option<String>,
    metadata_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct FileState {
    size_bytes: u64,
    modified_at_ms: i64,
    identity: String,
    metadata_fingerprint: String,
}

pub fn detail(state: &AppState, track_id: &str) -> Result<TrackMetadataDetail, AppError> {
    let record = load_record(state, track_id)?;
    let (_, metadata) = validate_live_file(&record)?;
    let connection = state.connection()?;
    let mut statement = connection.prepare(
        "SELECT id, created_at, before_json, after_json FROM metadata_history WHERE track_file_id = ?1 ORDER BY created_at DESC, id DESC LIMIT 100",
    )?;
    let history = statement
        .query_map([&record.file_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|(id, created_at, before, after)| {
            Ok(MetadataHistoryEntry {
                id,
                created_at,
                before: serde_json::from_str(&before)
                    .map_err(|error| AppError::Metadata(error.to_string()))?,
                after: serde_json::from_str(&after)
                    .map_err(|error| AppError::Metadata(error.to_string()))?,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    drop(statement);
    let mut rating_statement = connection.prepare(
        "SELECT id, created_at, before_rating, after_rating FROM track_rating_history WHERE track_id = ?1 ORDER BY created_at DESC, id DESC LIMIT 100",
    )?;
    let rating_history = rating_statement
        .query_map([track_id], |row| {
            Ok(TrackRatingHistoryEntry {
                id: row.get(0)?,
                created_at: row.get(1)?,
                before_rating: row.get(2)?,
                after_rating: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(TrackMetadataDetail {
        track_id: record.track_id,
        file_id: record.file_id,
        path: record.path.to_string_lossy().into_owned(),
        format: record.extension.to_uppercase(),
        values: MetadataValues::from(&metadata),
        history,
        rating_history,
    })
}

pub fn preview(
    state: &AppState,
    track_id: &str,
    values: MetadataValues,
) -> Result<MetadataPreview, AppError> {
    let record = load_record(state, track_id)?;
    let (file_state, metadata) = validate_live_file(&record)?;
    let before = MetadataValues::from(&metadata);
    let after = normalize(values)?;
    let changes = changes(&before, &after);
    if changes.is_empty() {
        return Err(AppError::Metadata("нет изменений для сохранения".into()));
    }
    Ok(MetadataPreview {
        track_id: record.track_id,
        file_id: record.file_id,
        expected_file_state: serde_json::to_string(&file_state)
            .map_err(|error| AppError::Metadata(error.to_string()))?,
        before,
        after,
        changes,
    })
}

pub fn bulk_preview(
    state: &AppState,
    request: BulkMetadataRequest,
) -> Result<BulkMetadataPreview, AppError> {
    if request.track_ids.is_empty() {
        return Err(AppError::Metadata("не выбраны треки".into()));
    }
    if request.track_ids.len() > 1000 {
        return Err(AppError::Metadata(
            "за одну операцию можно изменить не более 1000 треков".into(),
        ));
    }
    let fields = validate_bulk_fields(&request.fields)?;
    let mut seen = HashSet::new();
    let mut items = Vec::new();

    for track_id in request
        .track_ids
        .into_iter()
        .filter(|track_id| seen.insert(track_id.clone()))
    {
        let detail = match detail(state, &track_id) {
            Ok(value) => value,
            Err(error) => {
                items.push(BulkMetadataPreviewItem {
                    track_id: track_id.clone(),
                    title: track_id,
                    path: String::new(),
                    status: "failed".into(),
                    expected_file_state: None,
                    after: None,
                    changes: Vec::new(),
                    reason: error.to_string(),
                });
                continue;
            }
        };
        let title = detail
            .values
            .title
            .clone()
            .unwrap_or_else(|| "Без названия".into());
        let after = apply_bulk_fields(&detail.values, &request.values, &fields);
        match preview(state, &track_id, after) {
            Ok(item) => items.push(BulkMetadataPreviewItem {
                track_id,
                title,
                path: detail.path,
                status: "ready".into(),
                expected_file_state: Some(item.expected_file_state),
                after: Some(item.after),
                changes: item.changes,
                reason: "Готово к безопасной записи".into(),
            }),
            Err(AppError::Metadata(reason)) if reason == "нет изменений для сохранения" =>
            {
                items.push(BulkMetadataPreviewItem {
                    track_id,
                    title,
                    path: detail.path,
                    status: "skipped".into(),
                    expected_file_state: None,
                    after: None,
                    changes: Vec::new(),
                    reason,
                });
            }
            Err(error) => items.push(BulkMetadataPreviewItem {
                track_id,
                title,
                path: detail.path,
                status: "failed".into(),
                expected_file_state: None,
                after: None,
                changes: Vec::new(),
                reason: error.to_string(),
            }),
        }
    }

    Ok(BulkMetadataPreview {
        total_files: items.len() as u64,
        ready_files: items.iter().filter(|item| item.status == "ready").count() as u64,
        skipped_files: items.iter().filter(|item| item.status == "skipped").count() as u64,
        failed_files: items.iter().filter(|item| item.status == "failed").count() as u64,
        items,
    })
}

pub fn bulk_commit(
    state: &AppState,
    items: Vec<BulkMetadataCommitItem>,
) -> Result<BulkMetadataReport, AppError> {
    if items.is_empty() {
        return Err(AppError::Metadata("нет файлов, готовых к записи".into()));
    }
    if items.len() > 1000 {
        return Err(AppError::Metadata(
            "за одну операцию можно изменить не более 1000 треков".into(),
        ));
    }
    let mut report_items = Vec::with_capacity(items.len());
    for item in items {
        match commit(
            state,
            &item.track_id,
            &item.expected_file_state,
            item.values,
        ) {
            Ok(saved) => report_items.push(BulkMetadataReportItem {
                track_id: saved.track_id,
                title: saved.values.title.unwrap_or_else(|| "Без названия".into()),
                path: saved.path,
                status: "saved".into(),
                reason: "Теги записаны и повторно проверены".into(),
            }),
            Err(error) => {
                let fallback = detail(state, &item.track_id).ok();
                report_items.push(BulkMetadataReportItem {
                    track_id: item.track_id.clone(),
                    title: fallback
                        .as_ref()
                        .and_then(|value| value.values.title.clone())
                        .unwrap_or(item.track_id),
                    path: fallback.map(|value| value.path).unwrap_or_default(),
                    status: "failed".into(),
                    reason: error.to_string(),
                });
            }
        }
    }
    Ok(BulkMetadataReport {
        saved_files: report_items
            .iter()
            .filter(|item| item.status == "saved")
            .count() as u64,
        failed_files: report_items
            .iter()
            .filter(|item| item.status == "failed")
            .count() as u64,
        items: report_items,
    })
}

fn validate_bulk_fields(fields: &[String]) -> Result<HashSet<String>, AppError> {
    let allowed: HashSet<&str> = [
        "title",
        "artist",
        "album",
        "albumArtist",
        "trackNumber",
        "discNumber",
        "genre",
        "year",
        "composer",
        "comment",
        "bpm",
        "musicalKey",
    ]
    .into_iter()
    .collect();
    let selected: HashSet<String> = fields.iter().cloned().collect();
    if selected.is_empty() {
        return Err(AppError::Metadata(
            "отметьте хотя бы одно поле для изменения".into(),
        ));
    }
    if let Some(field) = selected
        .iter()
        .find(|field| !allowed.contains(field.as_str()))
    {
        return Err(AppError::Metadata(format!(
            "неподдерживаемое поле массового редактирования: {field}"
        )));
    }
    Ok(selected)
}

fn apply_bulk_fields(
    current: &MetadataValues,
    values: &MetadataValues,
    fields: &HashSet<String>,
) -> MetadataValues {
    let mut result = current.clone();
    macro_rules! apply {
        ($name:literal, $field:ident) => {
            if fields.contains($name) {
                result.$field = values.$field.clone();
            }
        };
    }
    apply!("title", title);
    apply!("artist", artist);
    apply!("album", album);
    apply!("albumArtist", album_artist);
    apply!("trackNumber", track_number);
    apply!("discNumber", disc_number);
    apply!("genre", genre);
    apply!("year", year);
    apply!("composer", composer);
    apply!("comment", comment);
    apply!("bpm", bpm);
    apply!("musicalKey", musical_key);
    result
}

pub fn commit(
    state: &AppState,
    track_id: &str,
    expected_file_state: &str,
    values: MetadataValues,
) -> Result<TrackMetadataDetail, AppError> {
    let expected: FileState = serde_json::from_str(expected_file_state)
        .map_err(|_| AppError::Metadata("предварительный просмотр устарел".into()))?;
    let record = load_record(state, track_id)?;
    let (current_state, current_metadata) = validate_live_file(&record)?;
    if current_state != expected {
        return Err(AppError::Metadata(
            "файл изменился после предварительного просмотра; перечитайте метаданные".into(),
        ));
    }
    let before = MetadataValues::from(&current_metadata);
    let after = normalize(values)?;
    if changes(&before, &after).is_empty() {
        return Err(AppError::Metadata("нет изменений для сохранения".into()));
    }

    let extension = record.extension.to_ascii_lowercase();
    let parent = record
        .path
        .parent()
        .ok_or_else(|| AppError::Metadata("у файла нет родительской папки".into()))?;
    let token = Uuid::new_v4();
    let temporary = parent.join(format!(".sorttrack-{token}.{extension}"));
    let backup = parent.join(format!(".sorttrack-backup-{token}.{extension}"));
    let mut source = open_source_exclusively(&record.path)?;

    let write_result = (|| -> Result<AudioMetadata, AppError> {
        let mut temporary_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        std::io::copy(&mut source, &mut temporary_file)?;
        temporary_file.sync_all()?;
        drop(temporary_file);
        write_values(&temporary, &extension, &after)?;
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(&temporary)?
            .sync_all()?;
        let verified = AudioMetadata::read(&temporary)
            .map_err(|error| AppError::Metadata(format!("проверка записанных тегов: {error}")))?;
        if MetadataValues::from(&verified) != after {
            return Err(AppError::Metadata(
                "повторное чтение не подтвердило записанные значения".into(),
            ));
        }
        Ok(verified)
    })();
    drop(source);
    let verified = match write_result {
        Ok(value) => value,
        Err(error) => {
            let _ = std::fs::remove_file(&temporary);
            return Err(error);
        }
    };

    let (still_current, _) = validate_live_file(&record)?;
    if still_current != expected {
        let _ = std::fs::remove_file(&temporary);
        return Err(AppError::Metadata(
            "исходный файл изменился во время подготовки записи".into(),
        ));
    }

    replace_file(&record.path, &temporary, Some(&backup))
        .map_err(|error| AppError::Metadata(format!("атомарная замена файла: {error}")))?;
    let new_signature = signature(&record.path, &verified.fingerprint())?;
    let operation_id = Uuid::new_v4().to_string();
    let database_result = record_commit(
        state,
        &record,
        &operation_id,
        &before,
        &after,
        &expected,
        &new_signature,
        &verified,
    );
    if let Err(error) = database_result {
        let _ = replace_file(&record.path, &backup, None);
        return Err(error);
    }
    let _ = std::fs::remove_file(&backup);
    detail(state, track_id)
}

fn load_record(state: &AppState, track_id: &str) -> Result<FileRecord, AppError> {
    let connection = state.connection()?;
    connection
        .query_row(
            "SELECT f.id, f.track_id, f.canonical_path, f.filename, f.extension, r.canonical_path, f.size_bytes, f.modified_at_ms, f.file_identity, m.metadata_fingerprint FROM track_files f JOIN library_roots r ON r.id = f.library_root_id JOIN track_metadata m ON m.track_file_id = f.id WHERE f.track_id = ?1 AND f.availability_status = 'available' AND lower(f.extension) IN ('mp3', 'flac') ORDER BY CASE lower(f.extension) WHEN 'flac' THEN 0 ELSE 1 END, f.created_at LIMIT 1",
            [track_id],
            |row| {
                Ok(FileRecord {
                    file_id: row.get(0)?,
                    track_id: row.get(1)?,
                    path: PathBuf::from(row.get::<_, String>(2)?),
                    filename: row.get(3)?,
                    extension: row.get(4)?,
                    root_path: PathBuf::from(row.get::<_, String>(5)?),
                    size_bytes: row.get(6)?,
                    modified_at_ms: row.get(7)?,
                    identity: row.get(8)?,
                    metadata_fingerprint: row.get(9)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| AppError::Metadata("доступный MP3/FLAC для трека не найден".into()))
}

fn validate_live_file(record: &FileRecord) -> Result<(FileState, AudioMetadata), AppError> {
    let path = record.path.canonicalize()?;
    let root = record.root_path.canonicalize()?;
    if !path.is_file() || !path.starts_with(root) {
        return Err(AppError::Metadata(
            "файл находится вне зарегистрированной библиотеки".into(),
        ));
    }
    let metadata = AudioMetadata::read(&path)
        .map_err(|error| AppError::Metadata(format!("не удалось прочитать теги: {error}")))?;
    let state = signature(&path, &metadata.fingerprint())?;
    if state.size_bytes != record.size_bytes
        || state.modified_at_ms != record.modified_at_ms
        || record.identity.as_deref() != Some(&state.identity)
        || state.metadata_fingerprint != record.metadata_fingerprint
    {
        return Err(AppError::Metadata(
            "файл изменён вне SortTrack; сначала повторно просканируйте библиотеку".into(),
        ));
    }
    Ok((state, metadata))
}

fn signature(path: &Path, metadata_fingerprint: &str) -> Result<FileState, AppError> {
    let metadata = path.metadata()?;
    Ok(FileState {
        size_bytes: metadata.len(),
        modified_at_ms: metadata
            .modified()?
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64,
        identity: file_identity(path)?,
        metadata_fingerprint: metadata_fingerprint.to_owned(),
    })
}

fn normalize(mut values: MetadataValues) -> Result<MetadataValues, AppError> {
    for value in [
        &mut values.title,
        &mut values.artist,
        &mut values.album,
        &mut values.album_artist,
        &mut values.genre,
        &mut values.year,
        &mut values.composer,
        &mut values.comment,
        &mut values.musical_key,
    ] {
        *value = value
            .take()
            .map(|text| text.trim().to_owned())
            .filter(|text| !text.is_empty());
    }
    values.track_number = normalize_number(values.track_number, "номер трека")?;
    values.disc_number = normalize_number(values.disc_number, "номер диска")?;
    if values
        .bpm
        .is_some_and(|value| !value.is_finite() || !(20.0..=400.0).contains(&value))
    {
        return Err(AppError::Metadata("BPM должен быть от 20 до 400".into()));
    }
    values.camelot = values.musical_key.as_deref().and_then(camelot_for_key);
    Ok(values)
}

fn normalize_number(value: Option<String>, label: &str) -> Result<Option<String>, AppError> {
    let Some(value) = value
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
    else {
        return Ok(None);
    };
    let number: u32 = value
        .parse()
        .map_err(|_| AppError::Metadata(format!("{label} должен быть целым числом")))?;
    if number == 0 {
        return Err(AppError::Metadata(format!(
            "{label} должен быть больше нуля"
        )));
    }
    Ok(Some(number.to_string()))
}

fn write_values(path: &Path, extension: &str, values: &MetadataValues) -> Result<(), AppError> {
    let tagged = Probe::open(path)
        .map_err(|error| AppError::Metadata(error.to_string()))?
        .read()
        .map_err(|error| AppError::Metadata(error.to_string()))?;
    let tag_type = match extension {
        "mp3" => TagType::Id3v2,
        "flac" => TagType::VorbisComments,
        _ => {
            return Err(AppError::Metadata(
                "поддерживаются только MP3 и FLAC".into(),
            ));
        }
    };
    let mut tag = tagged
        .tag(tag_type)
        .cloned()
        .unwrap_or_else(|| Tag::new(tag_type));

    set_accessor(&mut tag, values.title.as_deref(), AccessorField::Title);
    set_accessor(&mut tag, values.artist.as_deref(), AccessorField::Artist);
    set_accessor(&mut tag, values.album.as_deref(), AccessorField::Album);
    set_accessor(&mut tag, values.genre.as_deref(), AccessorField::Genre);
    set_accessor(&mut tag, values.comment.as_deref(), AccessorField::Comment);
    set_number(&mut tag, values.track_number.as_deref(), true);
    set_number(&mut tag, values.disc_number.as_deref(), false);
    set_text(
        &mut tag,
        ItemKey::AlbumArtist,
        values.album_artist.as_deref(),
    );
    tag.remove_key(ItemKey::Year);
    set_text(&mut tag, ItemKey::RecordingDate, values.year.as_deref());
    set_text(&mut tag, ItemKey::Composer, values.composer.as_deref());
    tag.remove_key(ItemKey::Bpm);
    set_text(
        &mut tag,
        ItemKey::IntegerBpm,
        values.bpm.map(|value| value.to_string()).as_deref(),
    );
    set_text(&mut tag, ItemKey::InitialKey, values.musical_key.as_deref());
    tag.save_to_path(path, WriteOptions::new())
        .map_err(|error| AppError::Metadata(format!("запись тегов: {error}")))?;
    Ok(())
}

enum AccessorField {
    Title,
    Artist,
    Album,
    Genre,
    Comment,
}

fn set_accessor(tag: &mut Tag, value: Option<&str>, field: AccessorField) {
    match (field, value) {
        (AccessorField::Title, Some(value)) => tag.set_title(value.to_owned()),
        (AccessorField::Title, None) => tag.remove_title(),
        (AccessorField::Artist, Some(value)) => tag.set_artist(value.to_owned()),
        (AccessorField::Artist, None) => tag.remove_artist(),
        (AccessorField::Album, Some(value)) => tag.set_album(value.to_owned()),
        (AccessorField::Album, None) => tag.remove_album(),
        (AccessorField::Genre, Some(value)) => tag.set_genre(value.to_owned()),
        (AccessorField::Genre, None) => tag.remove_genre(),
        (AccessorField::Comment, Some(value)) => tag.set_comment(value.to_owned()),
        (AccessorField::Comment, None) => tag.remove_comment(),
    }
}

fn set_number(tag: &mut Tag, value: Option<&str>, track: bool) {
    match (track, value.and_then(|value| value.parse::<u32>().ok())) {
        (true, Some(value)) => tag.set_track(value),
        (true, None) => tag.remove_track(),
        (false, Some(value)) => tag.set_disk(value),
        (false, None) => tag.remove_disk(),
    }
}

fn set_text(tag: &mut Tag, key: ItemKey, value: Option<&str>) {
    tag.remove_key(key);
    if let Some(value) = value {
        tag.insert_text(key, value.to_owned());
    }
}

#[cfg(windows)]
fn open_source_exclusively(path: &Path) -> Result<File, AppError> {
    use std::os::windows::fs::OpenOptionsExt;
    OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(path)
        .map_err(|error| AppError::Metadata(format!("файл занят другим приложением: {error}")))
}

#[cfg(not(windows))]
fn open_source_exclusively(path: &Path) -> Result<File, AppError> {
    let file = OpenOptions::new().read(true).open(path)?;
    fs2::FileExt::lock_shared(&file)
        .map_err(|error| AppError::Metadata(format!("файл занят другим приложением: {error}")))?;
    Ok(file)
}

#[allow(clippy::too_many_arguments)]
fn record_commit(
    state: &AppState,
    record: &FileRecord,
    operation_id: &str,
    before: &MetadataValues,
    after: &MetadataValues,
    old_state: &FileState,
    new_state: &FileState,
    verified: &AudioMetadata,
) -> Result<(), AppError> {
    let mut connection = state.connection()?;
    let transaction = connection.transaction()?;
    transaction.execute(
        "INSERT INTO metadata_history(id, track_file_id, operation_id, before_json, after_json, file_state_before, file_state_after) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            Uuid::new_v4().to_string(),
            record.file_id,
            operation_id,
            serde_json::to_string(before).map_err(|error| AppError::Metadata(error.to_string()))?,
            serde_json::to_string(after).map_err(|error| AppError::Metadata(error.to_string()))?,
            serde_json::to_string(old_state).map_err(|error| AppError::Metadata(error.to_string()))?,
            serde_json::to_string(new_state).map_err(|error| AppError::Metadata(error.to_string()))?,
        ],
    )?;
    transaction.execute(
        "UPDATE track_files SET size_bytes = ?1, modified_at_ms = ?2, file_identity = ?3, sha256 = NULL, sha256_state = 'stale', last_scanned_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE id = ?4",
        params![new_state.size_bytes, new_state.modified_at_ms, new_state.identity, record.file_id],
    )?;
    transaction.execute(
        "DELETE FROM track_audio_analysis WHERE track_file_id = ?1",
        [&record.file_id],
    )?;
    transaction.execute(
        "DELETE FROM audio_feature_cache WHERE track_file_id = ?1",
        [&record.file_id],
    )?;
    transaction.execute(
        "UPDATE tracks SET preferred_title = ?1, preferred_artist = ?2, updated_at = CURRENT_TIMESTAMP WHERE id = ?3",
        params![after.title.as_deref().unwrap_or(&record.filename), after.artist.as_deref().unwrap_or(""), record.track_id],
    )?;
    transaction.execute(
        "INSERT INTO track_metadata(track_file_id, title, artist, album, album_artist, track_number, disc_number, original_genre_tag, year, composer, comment, bpm, musical_key, camelot, metadata_fingerprint) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15) ON CONFLICT(track_file_id) DO UPDATE SET title=excluded.title, artist=excluded.artist, album=excluded.album, album_artist=excluded.album_artist, track_number=excluded.track_number, disc_number=excluded.disc_number, original_genre_tag=excluded.original_genre_tag, year=excluded.year, composer=excluded.composer, comment=excluded.comment, bpm=excluded.bpm, musical_key=excluded.musical_key, camelot=excluded.camelot, metadata_fingerprint=excluded.metadata_fingerprint, read_at=CURRENT_TIMESTAMP, updated_at=CURRENT_TIMESTAMP",
        params![record.file_id, verified.title, verified.artist, verified.album, verified.album_artist, verified.track_number, verified.disc_number, verified.original_genre_tag, verified.year, verified.composer, verified.comment, verified.bpm, verified.musical_key, verified.camelot, verified.fingerprint()],
    )?;
    transaction.commit()?;
    Ok(())
}

fn changes(before: &MetadataValues, after: &MetadataValues) -> Vec<MetadataChange> {
    let fields = [
        (
            "title",
            "Название",
            before.title.clone(),
            after.title.clone(),
        ),
        (
            "artist",
            "Исполнитель",
            before.artist.clone(),
            after.artist.clone(),
        ),
        ("album", "Альбом", before.album.clone(), after.album.clone()),
        (
            "albumArtist",
            "Исполнитель альбома",
            before.album_artist.clone(),
            after.album_artist.clone(),
        ),
        (
            "trackNumber",
            "Номер трека",
            before.track_number.clone(),
            after.track_number.clone(),
        ),
        (
            "discNumber",
            "Номер диска",
            before.disc_number.clone(),
            after.disc_number.clone(),
        ),
        (
            "genre",
            "Жанр в файле",
            before.genre.clone(),
            after.genre.clone(),
        ),
        ("year", "Год", before.year.clone(), after.year.clone()),
        (
            "composer",
            "Композитор",
            before.composer.clone(),
            after.composer.clone(),
        ),
        (
            "comment",
            "Комментарий",
            before.comment.clone(),
            after.comment.clone(),
        ),
        (
            "bpm",
            "BPM",
            before.bpm.map(|v| v.to_string()),
            after.bpm.map(|v| v.to_string()),
        ),
        (
            "musicalKey",
            "Тональность",
            before.musical_key.clone(),
            after.musical_key.clone(),
        ),
    ];
    fields
        .into_iter()
        .filter(|(_, _, old, new)| old != new)
        .map(|(field, label, before, after)| MetadataChange {
            field: field.into(),
            label: label.into(),
            before,
            after,
        })
        .collect()
}

#[cfg(windows)]
fn replace_file(
    original: &Path,
    replacement: &Path,
    backup: Option<&Path>,
) -> Result<(), AppError> {
    use std::{os::windows::ffi::OsStrExt, ptr};
    use windows_sys::Win32::Storage::FileSystem::{REPLACEFILE_IGNORE_MERGE_ERRORS, ReplaceFileW};
    let wide = |path: &Path| {
        path.as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>()
    };
    let original = wide(original);
    let replacement = wide(replacement);
    let backup = backup.map(wide);
    for attempt in 0..5 {
        let result = unsafe {
            ReplaceFileW(
                original.as_ptr(),
                replacement.as_ptr(),
                backup.as_ref().map_or(ptr::null(), |path| path.as_ptr()),
                REPLACEFILE_IGNORE_MERGE_ERRORS,
                ptr::null(),
                ptr::null(),
            )
        };
        if result != 0 {
            return Ok(());
        }
        let error = std::io::Error::last_os_error();
        if !matches!(error.raw_os_error(), Some(32 | 33)) || attempt == 4 {
            return Err(AppError::Io(error));
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    unreachable!()
}

#[cfg(not(windows))]
fn replace_file(
    original: &Path,
    replacement: &Path,
    backup: Option<&Path>,
) -> Result<(), AppError> {
    if let Some(backup) = backup {
        std::fs::rename(original, backup)?;
        if let Err(error) = std::fs::rename(replacement, original) {
            let _ = std::fs::rename(backup, original);
            return Err(AppError::Io(error));
        }
    } else {
        std::fs::remove_file(original)?;
        std::fs::rename(replacement, original)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_values_and_computes_camelot() {
        let values = normalize(MetadataValues {
            title: Some("  Night Drive  ".into()),
            track_number: Some("03".into()),
            bpm: Some(142.5),
            musical_key: Some("A minor".into()),
            ..MetadataValues::default()
        })
        .expect("valid values");
        assert_eq!(values.title.as_deref(), Some("Night Drive"));
        assert_eq!(values.track_number.as_deref(), Some("3"));
        assert_eq!(values.camelot.as_deref(), Some("8A"));
    }

    #[test]
    fn rejects_invalid_bpm_and_track_number() {
        assert!(
            normalize(MetadataValues {
                bpm: Some(500.0),
                ..MetadataValues::default()
            })
            .is_err()
        );
        assert!(
            normalize(MetadataValues {
                track_number: Some("A1".into()),
                ..MetadataValues::default()
            })
            .is_err()
        );
    }

    #[test]
    fn preview_changes_only_contains_modified_fields() {
        let before = MetadataValues {
            title: Some("Before".into()),
            artist: Some("Artist".into()),
            ..MetadataValues::default()
        };
        let after = MetadataValues {
            title: Some("After".into()),
            artist: Some("Artist".into()),
            ..MetadataValues::default()
        };
        let result = changes(&before, &after);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].field, "title");
    }

    #[test]
    fn writes_and_rechecks_metadata_on_an_mp3_copy() {
        let path = std::env::temp_dir().join(format!("sorttrack-tags-{}.mp3", Uuid::new_v4()));
        let frame_size = 417;
        let mut audio = vec![0u8; frame_size * 4];
        for frame in audio.chunks_exact_mut(frame_size) {
            frame[..4].copy_from_slice(&[0xff, 0xfb, 0x90, 0x64]);
        }
        std::fs::write(&path, audio).expect("minimal mp3");
        let values = normalize(MetadataValues {
            title: Some("Safe title".into()),
            artist: Some("SortTrack".into()),
            genre: Some("Techno".into()),
            bpm: Some(142.0),
            musical_key: Some("A minor".into()),
            ..MetadataValues::default()
        })
        .expect("values");
        write_values(&path, "mp3", &values).expect("write tags");
        let read_back = AudioMetadata::read(&path).expect("read tags");
        assert_eq!(MetadataValues::from(&read_back), values);
        std::fs::remove_file(path).expect("cleanup");
    }

    #[test]
    fn writes_and_rechecks_metadata_on_a_flac_copy() {
        let path = std::env::temp_dir().join(format!("sorttrack-tags-{}.flac", Uuid::new_v4()));
        let mut audio = b"fLaC".to_vec();
        audio.extend_from_slice(&[0x80, 0x00, 0x00, 34]);
        let mut stream_info = vec![0u8; 34];
        stream_info[0..2].copy_from_slice(&4096u16.to_be_bytes());
        stream_info[2..4].copy_from_slice(&4096u16.to_be_bytes());
        let packed = (44_100u64 << 44) | (1u64 << 41) | (15u64 << 36);
        stream_info[10..18].copy_from_slice(&packed.to_be_bytes());
        audio.extend_from_slice(&stream_info);
        std::fs::write(&path, audio).expect("minimal flac");
        let values = normalize(MetadataValues {
            title: Some("Safe FLAC title".into()),
            artist: Some("SortTrack".into()),
            genre: Some("Techno".into()),
            year: Some("2026".into()),
            ..MetadataValues::default()
        })
        .expect("values");
        write_values(&path, "flac", &values).expect("write tags");
        let read_back = AudioMetadata::read(&path).expect("read tags");
        assert_eq!(MetadataValues::from(&read_back), values);
        std::fs::remove_file(path).expect("cleanup");
    }

    #[test]
    fn commits_verified_metadata_and_records_history() {
        let root = std::env::temp_dir().join(format!("sorttrack-tag-commit-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("root");
        let path = root.join("Artist - Original.mp3");
        let frame_size = 417;
        let mut audio = vec![0u8; frame_size * 4];
        for frame in audio.chunks_exact_mut(frame_size) {
            frame[..4].copy_from_slice(&[0xff, 0xfb, 0x90, 0x64]);
        }
        std::fs::write(&path, audio).expect("minimal mp3");
        let state = AppState::in_memory().expect("database");
        crate::scanner::scan(&root, crate::models::ScanOptions::default(), &state)
            .expect("library scan");
        let track_id = state.list_tracks(1, 0, "").expect("tracks")[0].id.clone();
        let current = detail(&state, &track_id).expect("detail");
        let mut edited = current.values;
        edited.title = Some("Verified edit".into());
        edited.artist = Some("SortTrack".into());
        let first_preview = preview(&state, &track_id, edited.clone()).expect("preview");
        let saved = commit(
            &state,
            &track_id,
            &first_preview.expected_file_state,
            edited,
        )
        .expect("commit");
        assert_eq!(saved.values.title.as_deref(), Some("Verified edit"));
        assert_eq!(saved.history.len(), 1);
        assert_eq!(saved.history[0].before.title, None);
        assert_eq!(
            saved.history[0].after.title.as_deref(),
            Some("Verified edit")
        );
        assert!(root.read_dir().expect("files").all(|entry| {
            !entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .starts_with(".sorttrack-")
        }));

        let mut second_edit = saved.values;
        second_edit.title = Some("Must not be written".into());
        let stale_preview =
            preview(&state, &track_id, second_edit.clone()).expect("second preview");
        let size_before_external_change = path.metadata().expect("metadata").len();
        let mut external = OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("external open");
        std::io::Write::write_all(&mut external, b"x").expect("external change");
        drop(external);
        let rejected = commit(
            &state,
            &track_id,
            &stale_preview.expected_file_state,
            second_edit,
        )
        .expect_err("stale preview must be rejected");
        assert!(rejected.to_string().contains("изменён вне SortTrack"));
        assert_eq!(
            path.metadata().expect("metadata").len(),
            size_before_external_change + 1
        );
        let history_count: u64 = state
            .connection()
            .expect("connection")
            .query_row("SELECT count(*) FROM metadata_history", [], |row| {
                row.get(0)
            })
            .expect("history count");
        assert_eq!(history_count, 1);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn bulk_commit_isolates_a_file_changed_after_preview() {
        let root = std::env::temp_dir().join(format!("sorttrack-tag-bulk-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("root");
        let frame_size = 417;
        let mut audio = vec![0u8; frame_size * 4];
        for frame in audio.chunks_exact_mut(frame_size) {
            frame[..4].copy_from_slice(&[0xff, 0xfb, 0x90, 0x64]);
        }
        for filename in ["Artist - First.mp3", "Artist - Second.mp3"] {
            std::fs::write(root.join(filename), &audio).expect("minimal mp3");
        }
        let state = AppState::in_memory().expect("database");
        crate::scanner::scan(&root, crate::models::ScanOptions::default(), &state)
            .expect("library scan");
        let track_ids = state
            .list_tracks(10, 0, "")
            .expect("tracks")
            .into_iter()
            .map(|track| track.id)
            .collect::<Vec<_>>();
        let bulk = bulk_preview(
            &state,
            BulkMetadataRequest {
                track_ids,
                fields: vec!["genre".into(), "bpm".into()],
                values: MetadataValues {
                    genre: Some("Techno".into()),
                    bpm: Some(140.0),
                    ..MetadataValues::default()
                },
            },
        )
        .expect("bulk preview");
        assert_eq!(bulk.ready_files, 2);

        let changed_path = PathBuf::from(&bulk.items[1].path);
        let mut external = OpenOptions::new()
            .append(true)
            .open(changed_path)
            .expect("external open");
        std::io::Write::write_all(&mut external, b"x").expect("external change");
        drop(external);

        let report = bulk_commit(
            &state,
            bulk.items
                .into_iter()
                .filter_map(|item| {
                    Some(BulkMetadataCommitItem {
                        track_id: item.track_id,
                        expected_file_state: item.expected_file_state?,
                        values: item.after?,
                    })
                })
                .collect(),
        )
        .expect("bulk commit");
        assert_eq!(report.saved_files, 1);
        assert_eq!(report.failed_files, 1);
        let history_count: u64 = state
            .connection()
            .expect("connection")
            .query_row("SELECT count(*) FROM metadata_history", [], |row| {
                row.get(0)
            })
            .expect("history count");
        assert_eq!(history_count, 1);
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
