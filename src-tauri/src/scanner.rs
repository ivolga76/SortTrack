use std::{path::Path, time::UNIX_EPOCH};

use walkdir::WalkDir;

use crate::{
    audio_metadata::AudioMetadata,
    error::AppError,
    file_identity::file_identity,
    models::{ScanError, ScanOptions, ScanPreview, ScanResult},
    repository::{AppState, UpsertOutcome},
};

pub fn preview(root: &Path, options: ScanOptions) -> Result<ScanPreview, AppError> {
    let canonical_root = validate_root(root)?;
    let options = normalize_options(options)?;
    let mut discovered = 0;
    let mut warnings = Vec::new();
    let mut walker = WalkDir::new(&canonical_root).follow_links(false);
    if !options.recursive {
        walker = walker.max_depth(1);
    }
    for entry in walker.into_iter().filter_entry(|entry| {
        include_entry(
            entry.path(),
            entry.file_type().is_symlink(),
            &canonical_root,
            &options.exclusions,
        )
    }) {
        let entry = match entry {
            Ok(value) => value,
            Err(error) => {
                warnings.push(ScanError {
                    path: error
                        .path()
                        .map(|path| path.display().to_string())
                        .unwrap_or_default(),
                    reason: error.to_string(),
                });
                continue;
            }
        };
        if entry.file_type().is_file() && supported_extension(entry.path()) {
            discovered += 1;
        }
    }
    Ok(ScanPreview {
        root_path: canonical_root.to_string_lossy().into_owned(),
        discovered,
        recursive: options.recursive,
        exclusions: options.exclusions,
        warnings,
    })
}

#[cfg(test)]
pub fn scan(root: &Path, options: ScanOptions, state: &AppState) -> Result<ScanResult, AppError> {
    scan_controlled(root, options, state, |_| Ok(true))
}

pub fn scan_controlled<F>(
    root: &Path,
    options: ScanOptions,
    state: &AppState,
    mut checkpoint: F,
) -> Result<ScanResult, AppError>
where
    F: FnMut(u64) -> Result<bool, AppError>,
{
    let options = normalize_options(options)?;
    let canonical_root = validate_root(root)?;

    let root_string = canonical_root.to_string_lossy().to_string();
    let root_id = state.register_root(&root_string)?;
    state.update_root_scan_settings(&root_id, options.recursive, &options.exclusions)?;
    let mut result = ScanResult {
        root_path: root_string,
        discovered: 0,
        added: 0,
        updated: 0,
        moved: 0,
        restored: 0,
        missing: 0,
        skipped: 0,
        errors: Vec::new(),
        cancelled: false,
    };

    let mut walker = WalkDir::new(&canonical_root).follow_links(false);
    if !options.recursive {
        walker = walker.max_depth(1);
    }
    for entry in walker.into_iter().filter_entry(|entry| {
        include_entry(
            entry.path(),
            entry.file_type().is_symlink(),
            &canonical_root,
            &options.exclusions,
        )
    }) {
        if !checkpoint(result.discovered)? {
            result.cancelled = true;
            break;
        }
        let entry = match entry {
            Ok(value) => value,
            Err(error) => {
                result.errors.push(ScanError {
                    path: error
                        .path()
                        .map(|path| path.display().to_string())
                        .unwrap_or_default(),
                    reason: error.to_string(),
                });
                continue;
            }
        };

        if entry.file_type().is_symlink() || !entry.file_type().is_file() {
            continue;
        }
        if !supported_extension(entry.path()) {
            continue;
        }
        let extension = entry
            .path()
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_lowercase();

        result.discovered += 1;
        let process = || -> Result<(UpsertOutcome, Option<String>), AppError> {
            let path = entry.path().canonicalize()?;
            if !path.starts_with(&canonical_root) {
                return Err(AppError::InvalidLibraryRoot(path.display().to_string()));
            }
            let metadata = entry
                .metadata()
                .map_err(|error| AppError::Io(error.into()))?;
            let modified_at_ms = metadata
                .modified()?
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64;
            let filename = entry.file_name().to_string_lossy();
            let canonical_path = path.to_string_lossy();
            let identity = file_identity(&path)?;
            if !state.reconcile_file_registration(
                &root_id,
                &canonical_path,
                metadata.len(),
                modified_at_ms,
                &identity,
            )? {
                return Ok((UpsertOutcome::Unchanged, None));
            }
            let (audio, availability_status, parse_error) = match AudioMetadata::read(&path) {
                Ok(audio) => (audio, "available", None),
                Err(error) => (AudioMetadata::default(), "corrupted", Some(error)),
            };
            let outcome = state.upsert_file(
                &root_id,
                &canonical_path,
                &filename,
                &extension,
                metadata.len(),
                modified_at_ms,
                &identity,
                &audio,
                availability_status,
            )?;
            Ok((outcome, parse_error))
        };

        match process() {
            Ok((outcome, parse_error)) => {
                match outcome {
                    UpsertOutcome::Added => result.added += 1,
                    UpsertOutcome::Updated => result.updated += 1,
                    UpsertOutcome::Moved => result.moved += 1,
                    UpsertOutcome::Restored => result.restored += 1,
                    UpsertOutcome::Unchanged => result.skipped += 1,
                }
                if let Some(reason) = parse_error {
                    result.errors.push(ScanError {
                        path: entry.path().display().to_string(),
                        reason: format!("Не удалось прочитать аудиофайл: {reason}"),
                    });
                }
            }
            Err(error) => result.errors.push(ScanError {
                path: entry.path().display().to_string(),
                reason: error.to_string(),
            }),
        }
    }

    if !result.cancelled && !checkpoint(result.discovered)? {
        result.cancelled = true;
    }

    if !result.cancelled {
        let (missing, reconcile_errors) = state.mark_missing_files(&root_id)?;
        result.missing = missing;
        result.errors.extend(
            reconcile_errors
                .into_iter()
                .map(|(path, reason)| ScanError { path, reason }),
        );
    }

    Ok(result)
}

fn validate_root(root: &Path) -> Result<std::path::PathBuf, AppError> {
    let canonical_root = root.canonicalize()?;
    if !canonical_root.is_dir() {
        return Err(AppError::InvalidLibraryRoot(
            canonical_root.display().to_string(),
        ));
    }
    Ok(canonical_root)
}

fn normalize_options(options: ScanOptions) -> Result<ScanOptions, AppError> {
    if options.exclusions.len() > 50 {
        return Err(AppError::Task(
            "A scan can contain at most 50 exclusions".into(),
        ));
    }
    let mut exclusions = Vec::new();
    for raw in options.exclusions {
        let value = raw.trim().replace('\\', "/").trim_matches('/').to_string();
        if value.is_empty() {
            continue;
        }
        if value.len() > 200
            || value.contains(':')
            || value
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
        {
            return Err(AppError::Task(format!(
                "Unsafe relative scan exclusion: {raw}"
            )));
        }
        let normalized = value.to_lowercase();
        if !exclusions.contains(&normalized) {
            exclusions.push(normalized);
        }
    }
    Ok(ScanOptions {
        recursive: options.recursive,
        exclusions,
    })
}

fn include_entry(path: &Path, symlink: bool, root: &Path, exclusions: &[String]) -> bool {
    if path == root {
        return true;
    }
    if symlink {
        return false;
    }
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };
    let relative = relative.to_string_lossy().replace('\\', "/").to_lowercase();
    !exclusions
        .iter()
        .any(|excluded| relative == *excluded || relative.starts_with(&format!("{excluded}/")))
}

fn supported_extension(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|value| value.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("mp3" | "flac")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn recursive_options() -> ScanOptions {
        ScanOptions {
            recursive: true,
            exclusions: Vec::new(),
        }
    }

    #[test]
    fn scans_supported_files_without_duplicate_rows() {
        let test_root = std::env::temp_dir().join(format!("sorttrack-scan-{}", Uuid::new_v4()));
        std::fs::create_dir_all(test_root.join("nested")).expect("test folder");
        std::fs::write(test_root.join("Artist - Track.flac"), b"fake flac").expect("flac");
        std::fs::write(test_root.join("nested").join("Second.MP3"), b"fake mp3").expect("mp3");
        std::fs::write(test_root.join("notes.txt"), b"ignored").expect("text");

        let state = AppState::in_memory().expect("database");
        let first = scan(&test_root, recursive_options(), &state).expect("first scan");
        assert_eq!(first.discovered, 2);
        assert_eq!(first.added, 2);
        assert_eq!(first.skipped, 0);
        assert_eq!(first.errors.len(), 2);

        let second = scan(&test_root, recursive_options(), &state).expect("second scan");
        assert_eq!(second.discovered, 2);
        assert_eq!(second.added, 0);
        assert_eq!(second.skipped, 2);
        assert!(second.errors.is_empty());
        assert_eq!(state.summary().expect("summary").tracks, 2);

        let original = test_root.join("Artist - Track.flac");
        let moved = test_root.join("Artist - Moved.flac");
        std::fs::rename(&original, &moved).expect("move inside root");
        let moved_scan = scan(&test_root, recursive_options(), &state).expect("moved scan");
        assert_eq!(moved_scan.moved, 1);
        assert_eq!(moved_scan.missing, 0);
        assert_eq!(state.summary().expect("summary after move").tracks, 2);

        let parked = test_root.with_extension("parked-flac");
        std::fs::rename(&moved, &parked).expect("move outside root");
        let missing_scan = scan(&test_root, recursive_options(), &state).expect("missing scan");
        assert_eq!(missing_scan.missing, 1);
        assert!(
            state.list_tracks(10, 0, "Moved").expect("missing track")[0]
                .analysis_state
                .eq("missing")
        );

        std::fs::rename(&parked, &moved).expect("return to root");
        let restored_scan = scan(&test_root, recursive_options(), &state).expect("restored scan");
        assert_eq!(restored_scan.restored, 1);
        assert_eq!(restored_scan.missing, 0);
        assert_eq!(state.summary().expect("summary after restore").tracks, 2);

        std::fs::remove_dir_all(test_root).expect("cleanup");
    }

    #[test]
    fn preview_respects_depth_and_safe_relative_exclusions() {
        let root = std::env::temp_dir().join(format!("sorttrack-scan-preview-{}", Uuid::new_v4()));
        std::fs::create_dir_all(root.join("nested").join("ignore")).unwrap();
        std::fs::write(root.join("top.flac"), b"audio").unwrap();
        std::fs::write(root.join("nested").join("keep.mp3"), b"audio").unwrap();
        std::fs::write(
            root.join("nested").join("ignore").join("hidden.flac"),
            b"audio",
        )
        .unwrap();

        let shallow = preview(
            &root,
            ScanOptions {
                recursive: false,
                exclusions: vec![],
            },
        )
        .unwrap();
        assert_eq!(shallow.discovered, 1);
        let filtered = preview(
            &root,
            ScanOptions {
                recursive: true,
                exclusions: vec!["nested/ignore".into()],
            },
        )
        .unwrap();
        assert_eq!(filtered.discovered, 2);
        assert_eq!(filtered.exclusions, vec!["nested/ignore"]);
        assert!(
            preview(
                &root,
                ScanOptions {
                    recursive: true,
                    exclusions: vec!["../outside".into()]
                }
            )
            .is_err()
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn controlled_scan_stops_between_files_and_keeps_committed_index_rows() {
        let root = std::env::temp_dir().join(format!("sorttrack-scan-cancel-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("first.flac"), b"audio").unwrap();
        std::fs::write(root.join("second.mp3"), b"audio").unwrap();
        let state = AppState::in_memory().unwrap();
        let report = scan_controlled(&root, recursive_options(), &state, |processed| {
            Ok(processed < 1)
        })
        .unwrap();
        assert!(report.cancelled);
        assert_eq!(report.discovered, 1);
        assert_eq!(state.summary().unwrap().tracks, 1);
        std::fs::remove_dir_all(root).unwrap();
    }
}
