use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use lofty::{file::TaggedFileExt, picture::PictureType, probe::Probe};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::AppError, file_identity::file_identity, models::CoverData, repository::AppState,
};

const CACHE_VERSION: u32 = 1;
const MAX_COVER_BYTES: usize = 10 * 1024 * 1024;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CoverManifest {
    cache_version: u32,
    file_id: String,
    size_bytes: u64,
    modified_at_ms: i64,
    file_identity: String,
    image_filename: Option<String>,
    mime_type: Option<String>,
}

pub fn load(state: &AppState, track_id: &str) -> Result<CoverData, AppError> {
    load_in(state, track_id, &state.cover_cache_dir()?)
}

fn load_in(state: &AppState, track_id: &str, cache_dir: &Path) -> Result<CoverData, AppError> {
    let (file_id, stored_path): (String, String) = state
        .connection()?
        .query_row(
            "SELECT id, canonical_path FROM track_files WHERE track_id = ?1 AND availability_status = 'available' ORDER BY CASE lower(extension) WHEN 'flac' THEN 0 ELSE 1 END, updated_at DESC, id LIMIT 1",
            [track_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::TrackUnavailable(track_id.to_owned()))?;
    validate_file_id(&file_id)?;
    let audio_path = PathBuf::from(stored_path)
        .canonicalize()
        .map_err(|_| AppError::TrackUnavailable(track_id.to_owned()))?;
    let metadata = audio_path.metadata()?;
    let size_bytes = metadata.len();
    let modified_at_ms = metadata
        .modified()?
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    let identity = file_identity(&audio_path)?;
    fs::create_dir_all(cache_dir)?;
    let manifest_path = cache_dir.join(format!("{file_id}.json"));

    if let Ok(bytes) = fs::read(&manifest_path)
        && let Ok(manifest) = serde_json::from_slice::<CoverManifest>(&bytes)
        && manifest.cache_version == CACHE_VERSION
        && manifest.file_id == file_id
        && manifest.size_bytes == size_bytes
        && manifest.modified_at_ms == modified_at_ms
        && manifest.file_identity == identity
    {
        let cover_path = manifest
            .image_filename
            .as_deref()
            .map(|filename| cache_dir.join(filename))
            .filter(|path| path.is_file())
            .map(|path| path.to_string_lossy().into_owned());
        if manifest.image_filename.is_none() || cover_path.is_some() {
            return Ok(CoverData {
                cover_path,
                mime_type: manifest.mime_type,
                cache_hit: true,
            });
        }
    }

    let tagged = Probe::open(&audio_path)
        .map_err(|error| AppError::Task(error.to_string()))?
        .read()
        .map_err(|error| AppError::Task(error.to_string()))?;
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag());
    let picture = tag.and_then(|tag| {
        tag.get_picture_type(PictureType::CoverFront)
            .or_else(|| tag.pictures().first())
    });
    let extracted = picture.and_then(|picture| {
        let mime = picture.mime_type()?;
        let extension = mime.ext()?;
        let supported = matches!(extension, "jpg" | "png" | "gif" | "bmp");
        (supported && !picture.data().is_empty() && picture.data().len() <= MAX_COVER_BYTES)
            .then(|| (picture.data(), extension, mime.as_str().to_owned()))
    });

    remove_previous_images(cache_dir, &file_id)?;
    let (image_filename, mime_type, cover_path) = if let Some((bytes, extension, mime)) = extracted
    {
        let filename = format!("{file_id}.{extension}");
        let path = cache_dir.join(&filename);
        atomic_write(&path, bytes)?;
        (
            Some(filename),
            Some(mime),
            Some(path.to_string_lossy().into_owned()),
        )
    } else {
        (None, None, None)
    };
    let manifest = CoverManifest {
        cache_version: CACHE_VERSION,
        file_id,
        size_bytes,
        modified_at_ms,
        file_identity: identity,
        image_filename,
        mime_type: mime_type.clone(),
    };
    atomic_write(
        &manifest_path,
        &serde_json::to_vec(&manifest).map_err(|error| AppError::Task(error.to_string()))?,
    )?;
    Ok(CoverData {
        cover_path,
        mime_type,
        cache_hit: false,
    })
}

fn validate_file_id(file_id: &str) -> Result<(), AppError> {
    if file_id.len() <= 64
        && file_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
    {
        Ok(())
    } else {
        Err(AppError::Task(
            "Некорректный идентификатор cover-кэша".into(),
        ))
    }
}

fn remove_previous_images(cache_dir: &Path, file_id: &str) -> Result<(), AppError> {
    for extension in ["jpg", "png", "gif", "bmp"] {
        let candidate = cache_dir.join(format!("{file_id}.{extension}"));
        if candidate.is_file() {
            fs::remove_file(candidate)?;
        }
    }
    Ok(())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let temporary = path.with_extension(format!("{}.tmp", Uuid::new_v4()));
    let mut file = File::create(&temporary)?;
    file.write_all(bytes)?;
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
    use lofty::{
        config::WriteOptions,
        picture::{MimeType, Picture},
        tag::{Tag, TagExt, TagType},
    };

    #[test]
    fn absence_of_embedded_cover_is_cached() {
        let root = std::env::temp_dir().join(format!("sorttrack-cover-{}", Uuid::new_v4()));
        let cache = root.join("cache");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("No cover.mp3");
        let frame_size = 417;
        let mut audio = vec![0u8; frame_size * 4];
        for frame in audio.chunks_exact_mut(frame_size) {
            frame[..4].copy_from_slice(&[0xff, 0xfb, 0x90, 0x64]);
        }
        fs::write(&path, audio).unwrap();
        let state = AppState::in_memory().unwrap();
        let root_id = state
            .register_root(&root.canonicalize().unwrap().to_string_lossy())
            .unwrap();
        let canonical = path.canonicalize().unwrap();
        let metadata = canonical.metadata().unwrap();
        assert_eq!(
            state
                .upsert_file(
                    &root_id,
                    &canonical.to_string_lossy(),
                    "No cover.mp3",
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
                    "available"
                )
                .unwrap(),
            UpsertOutcome::Added
        );
        let track_id = state.list_tracks(1, 0, "").unwrap()[0].id.clone();
        let first = load_in(&state, &track_id, &cache).unwrap();
        assert!(!first.cache_hit);
        assert!(first.cover_path.is_none());
        let second = load_in(&state, &track_id, &cache).unwrap();
        assert!(second.cache_hit);
        assert!(second.cover_path.is_none());
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn embedded_front_cover_is_extracted_to_the_cache() {
        let root = std::env::temp_dir().join(format!("sorttrack-cover-image-{}", Uuid::new_v4()));
        let cache = root.join("cache");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("With cover.mp3");
        let frame_size = 417;
        let mut audio = vec![0u8; frame_size * 4];
        for frame in audio.chunks_exact_mut(frame_size) {
            frame[..4].copy_from_slice(&[0xff, 0xfb, 0x90, 0x64]);
        }
        fs::write(&path, audio).unwrap();
        let png = vec![
            0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1f, 0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x08,
            0xd7, 0x63, 0xf8, 0xcf, 0xc0, 0xf0, 0x1f, 0x00, 0x05, 0x00, 0x01, 0xff, 0x89, 0x99,
            0x3d, 0x1d, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
        ];
        let mut tag = Tag::new(TagType::Id3v2);
        tag.push_picture(
            Picture::unchecked(png.clone())
                .pic_type(PictureType::CoverFront)
                .mime_type(MimeType::Png)
                .build(),
        );
        tag.save_to_path(&path, WriteOptions::default()).unwrap();

        let state = AppState::in_memory().unwrap();
        let root_id = state
            .register_root(&root.canonicalize().unwrap().to_string_lossy())
            .unwrap();
        let canonical = path.canonicalize().unwrap();
        let metadata = canonical.metadata().unwrap();
        state
            .upsert_file(
                &root_id,
                &canonical.to_string_lossy(),
                "With cover.mp3",
                "mp3",
                metadata.len(),
                metadata
                    .modified()
                    .unwrap()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64,
                &file_identity(&canonical).unwrap(),
                &AudioMetadata::read(&canonical).unwrap(),
                "available",
            )
            .unwrap();
        let track_id = state.list_tracks(1, 0, "").unwrap()[0].id.clone();
        let cover = load_in(&state, &track_id, &cache).unwrap();
        assert_eq!(cover.mime_type.as_deref(), Some("image/png"));
        assert_eq!(fs::read(cover.cover_path.unwrap()).unwrap(), png);
        fs::remove_dir_all(root).ok();
    }
}
