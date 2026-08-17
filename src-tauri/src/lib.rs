mod audio_analysis;
mod audio_features;
mod audio_metadata;
mod background_jobs;
mod backup;
mod cover_art;
mod diagnostics;
mod duplicates;
mod error;
mod file_copy;
mod file_identity;
mod file_relink;
mod genre_assistant;
mod genre_dataset;
mod genre_model;
mod metadata_editor;
mod models;
mod playlist_export;
mod repository;
mod restore;
mod runtime_profile;
mod scan_runtime;
mod scanner;
mod waveform;

use std::path::PathBuf;

use diagnostics::{DiagnosticsExportReport, DiagnosticsPreview, DiagnosticsState};
use error::AppError;
use models::{
    AnalysisJob, BackupReport, BulkMetadataCommitItem, BulkMetadataPreview, BulkMetadataReport,
    BulkMetadataRequest, BulkRatingReport, CopyPlan, CopyRecoveryOperation, CopyRecoveryReport,
    CopyReport, CopyResolution, CoverData, DuplicateGroup, DuplicateScanResult, GenreDatasetAudit,
    GenreDatasetExportReport, GenreModelEvaluation, GenreSuggestion, LibraryRootRemovalPreview,
    LibraryRootRemovalReport, LibraryRootSettings, LibrarySummary, LibraryTableLayout,
    MetadataPreview, MetadataValues, PlaylistDetail, PlaylistExportPreview, PlaylistExportReport,
    PlaylistExportRequest, PlaylistRemovalPreview, PlaylistRemovalReport, PlaylistSnapshotReport,
    RelinkPreview, RelinkReport, RestorePrepared, RestorePreview, ScanOptions, ScanPreview,
    ScanResult, SmartPlaylistRule, TrackMetadataDetail, TrackPage, TrackRow, WaveformData,
};
use repository::AppState;
use runtime_profile::RuntimeProfile;
use scan_runtime::{ScanProgress, ScanRuntimeState};
use tauri::{AppHandle, Manager, State};

#[tauri::command]
fn get_library_summary(state: State<'_, AppState>) -> Result<LibrarySummary, AppError> {
    state.summary()
}

#[tauri::command]
fn list_library_roots(state: State<'_, AppState>) -> Result<Vec<LibraryRootSettings>, AppError> {
    state.list_library_roots()
}

#[tauri::command]
fn set_library_root_enabled(
    state: State<'_, AppState>,
    root_id: String,
    expected_status: String,
    enabled: bool,
) -> Result<(), AppError> {
    state.set_library_root_enabled(&root_id, &expected_status, enabled)
}

#[tauri::command]
async fn refresh_library_root_availability(
    state: State<'_, AppState>,
) -> Result<Vec<LibraryRootSettings>, AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.refresh_library_root_availability())
        .await
        .map_err(|error| AppError::Task(error.to_string()))?
}

#[tauri::command]
fn preview_library_root_removal(
    state: State<'_, AppState>,
    root_id: String,
) -> Result<LibraryRootRemovalPreview, AppError> {
    state.preview_library_root_removal(&root_id)
}

#[tauri::command]
fn remove_library_root_registration(
    state: State<'_, AppState>,
    root_id: String,
    expected_linked_files: u64,
) -> Result<LibraryRootRemovalReport, AppError> {
    state.remove_library_root_registration(&root_id, expected_linked_files)
}

#[tauri::command]
fn get_library_table_layout(state: State<'_, AppState>) -> Result<LibraryTableLayout, AppError> {
    state.library_table_layout()
}

#[tauri::command]
fn save_library_table_layout(
    state: State<'_, AppState>,
    layout: LibraryTableLayout,
) -> Result<LibraryTableLayout, AppError> {
    state.save_library_table_layout(layout)
}

#[tauri::command]
fn get_runtime_profile(profile: State<'_, RuntimeProfile>) -> RuntimeProfile {
    profile.inner().clone()
}

#[tauri::command]
fn preview_diagnostics(
    diagnostics: State<'_, DiagnosticsState>,
    redact_paths: Option<bool>,
) -> Result<DiagnosticsPreview, AppError> {
    diagnostics.preview(redact_paths.unwrap_or(true))
}

#[tauri::command]
async fn export_diagnostics(
    diagnostics: State<'_, DiagnosticsState>,
    destination_path: String,
    expected_state: String,
    redact_paths: Option<bool>,
) -> Result<DiagnosticsExportReport, AppError> {
    let diagnostics = diagnostics.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        diagnostics.export(
            PathBuf::from(destination_path).as_path(),
            &expected_state,
            redact_paths.unwrap_or(true),
        )
    })
    .await
    .map_err(|error| AppError::Task(error.to_string()))?
}

#[tauri::command]
fn open_log_directory(diagnostics: State<'_, DiagnosticsState>) -> Result<(), AppError> {
    diagnostics.open_log_directory()
}

#[tauri::command]
fn clear_diagnostics(diagnostics: State<'_, DiagnosticsState>) -> Result<u64, AppError> {
    diagnostics.clear()
}

#[tauri::command]
async fn create_backup(
    state: State<'_, AppState>,
    destination_root: String,
) -> Result<BackupReport, AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        backup::create(&state, PathBuf::from(destination_root).as_path())
    })
    .await
    .map_err(|error| AppError::Task(error.to_string()))?
}

#[tauri::command]
async fn preview_restore(directory_path: String) -> Result<RestorePreview, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        restore::preview(PathBuf::from(directory_path).as_path())
    })
    .await
    .map_err(|error| AppError::Task(error.to_string()))?
}

#[tauri::command]
async fn prepare_restore_and_restart(
    app: AppHandle,
    state: State<'_, AppState>,
    directory_path: String,
    expected_manifest_sha256: String,
) -> Result<RestorePrepared, AppError> {
    let state = state.inner().clone();
    let prepared = tauri::async_runtime::spawn_blocking(move || {
        restore::prepare(
            &state,
            PathBuf::from(directory_path).as_path(),
            &expected_manifest_sha256,
        )
    })
    .await
    .map_err(|error| AppError::Task(error.to_string()))??;
    app.request_restart();
    Ok(prepared)
}

#[tauri::command]
fn list_tracks(
    state: State<'_, AppState>,
    limit: Option<u32>,
    offset: Option<u32>,
    search: Option<String>,
) -> Result<Vec<TrackRow>, AppError> {
    state.list_tracks(
        limit.unwrap_or(200).min(1000),
        offset.unwrap_or(0),
        search.as_deref().unwrap_or(""),
    )
}

#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri maps these named IPC fields directly from the frontend.
fn search_tracks(
    state: State<'_, AppState>,
    limit: Option<u32>,
    offset: Option<u32>,
    search: Option<String>,
    genre: Option<String>,
    only_incoming: Option<bool>,
    min_bpm: Option<f64>,
    max_bpm: Option<f64>,
    min_rating: Option<u8>,
    camelot: Option<String>,
    format: Option<String>,
    file_status: Option<String>,
    sort_by: Option<String>,
    descending: Option<bool>,
) -> Result<TrackPage, AppError> {
    state.query_tracks(
        limit.unwrap_or(100),
        offset.unwrap_or(0),
        search.as_deref().unwrap_or(""),
        genre.as_deref(),
        only_incoming.unwrap_or(false),
        min_bpm,
        max_bpm,
        min_rating,
        camelot.as_deref(),
        format.as_deref(),
        file_status.as_deref().unwrap_or("all"),
        sort_by.as_deref().unwrap_or("added"),
        descending.unwrap_or(true),
    )
}

#[tauri::command]
fn locate_track_page(
    state: State<'_, AppState>,
    track_id: String,
    page_size: Option<u32>,
) -> Result<Option<u32>, AppError> {
    state.locate_track_page(&track_id, page_size.unwrap_or(100))
}

#[tauri::command]
async fn preview_missing_file_relink(
    state: State<'_, AppState>,
    track_id: String,
    selected_path: String,
) -> Result<RelinkPreview, AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        file_relink::preview(&state, &track_id, PathBuf::from(selected_path).as_path())
    })
    .await
    .map_err(|error| AppError::Task(error.to_string()))?
}

#[tauri::command]
async fn commit_missing_file_relink(
    state: State<'_, AppState>,
    track_id: String,
    selected_path: String,
    expected_state: String,
) -> Result<RelinkReport, AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        file_relink::commit(
            &state,
            &track_id,
            PathBuf::from(selected_path).as_path(),
            &expected_state,
        )
    })
    .await
    .map_err(|error| AppError::Task(error.to_string()))?
}

#[tauri::command]
async fn preview_music_folder(path: String, options: ScanOptions) -> Result<ScanPreview, AppError> {
    let path = PathBuf::from(path);
    tauri::async_runtime::spawn_blocking(move || scanner::preview(&path, options))
        .await
        .map_err(|error| AppError::Task(error.to_string()))?
}

#[tauri::command]
async fn scan_music_folder(
    state: State<'_, AppState>,
    runtime: State<'_, ScanRuntimeState>,
    path: String,
    options: ScanOptions,
    expected_total: u64,
) -> Result<ScanResult, AppError> {
    let path = PathBuf::from(path);
    let state = state.inner().clone();
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let job_id = runtime.start(
            path.to_string_lossy().into_owned(),
            expected_total.min(10_000_000),
        )?;
        let result = scanner::scan_controlled(&path, options, &state, |processed| {
            runtime.checkpoint(&job_id, processed)
        });
        match result {
            Ok(report) => {
                runtime.finish(&job_id, report.discovered, report.cancelled)?;
                Ok(report)
            }
            Err(error) => {
                runtime.fail(&job_id);
                Err(error)
            }
        }
    })
    .await
    .map_err(|error| AppError::Task(error.to_string()))?
}

#[tauri::command]
fn get_scan_progress(
    runtime: State<'_, ScanRuntimeState>,
) -> Result<Option<ScanProgress>, AppError> {
    runtime.progress()
}

#[tauri::command]
fn pause_scan(runtime: State<'_, ScanRuntimeState>, job_id: String) -> Result<(), AppError> {
    runtime.pause(&job_id)
}

#[tauri::command]
fn resume_scan(runtime: State<'_, ScanRuntimeState>, job_id: String) -> Result<(), AppError> {
    runtime.resume(&job_id)
}

#[tauri::command]
fn cancel_scan(runtime: State<'_, ScanRuntimeState>, job_id: String) -> Result<(), AppError> {
    runtime.cancel(&job_id)
}

#[tauri::command]
fn prepare_track_playback(
    app: AppHandle,
    state: State<'_, AppState>,
    track_id: String,
) -> Result<String, AppError> {
    let path = state.playback_path(&track_id)?;
    app.asset_protocol_scope()
        .allow_file(&path)
        .map_err(|error| AppError::Task(error.to_string()))?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
async fn get_track_waveform(
    state: State<'_, AppState>,
    track_id: String,
) -> Result<WaveformData, AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || waveform::load(&state, &track_id))
        .await
        .map_err(|error| AppError::Task(error.to_string()))?
}

#[tauri::command]
async fn prepare_track_cover(
    app: AppHandle,
    state: State<'_, AppState>,
    track_id: String,
) -> Result<CoverData, AppError> {
    let state = state.inner().clone();
    let cover = tauri::async_runtime::spawn_blocking(move || cover_art::load(&state, &track_id))
        .await
        .map_err(|error| AppError::Task(error.to_string()))??;
    if let Some(path) = cover.cover_path.as_deref() {
        app.asset_protocol_scope()
            .allow_file(path)
            .map_err(|error| AppError::Task(error.to_string()))?;
    }
    Ok(cover)
}

#[tauri::command]
fn list_playlists(state: State<'_, AppState>) -> Result<Vec<PlaylistDetail>, AppError> {
    state.list_playlists()
}

#[tauri::command]
fn set_track_rating(
    state: State<'_, AppState>,
    track_id: String,
    expected_rating: u8,
    next_rating: u8,
) -> Result<(), AppError> {
    state.set_track_rating(&track_id, expected_rating, next_rating)
}

#[tauri::command]
fn bulk_set_track_rating(
    state: State<'_, AppState>,
    track_ids: Vec<String>,
    target_rating: u8,
) -> Result<BulkRatingReport, AppError> {
    state.bulk_set_track_rating(&track_ids, target_rating)
}

#[tauri::command]
fn create_playlist(state: State<'_, AppState>, name: String) -> Result<String, AppError> {
    state.create_playlist(&name)
}

#[tauri::command]
fn create_smart_playlist(
    state: State<'_, AppState>,
    name: String,
    rule: SmartPlaylistRule,
) -> Result<String, AppError> {
    state.create_smart_playlist(&name, rule)
}

#[tauri::command]
fn snapshot_smart_playlist(
    state: State<'_, AppState>,
    smart_playlist_id: String,
    name: String,
) -> Result<PlaylistSnapshotReport, AppError> {
    state.snapshot_smart_playlist(&smart_playlist_id, &name)
}

#[tauri::command]
fn update_smart_playlist_rule(
    state: State<'_, AppState>,
    smart_playlist_id: String,
    expected_rule: SmartPlaylistRule,
    next_rule: SmartPlaylistRule,
) -> Result<(), AppError> {
    state.update_smart_playlist_rule(&smart_playlist_id, expected_rule, next_rule)
}

#[tauri::command]
fn rename_playlist(
    state: State<'_, AppState>,
    playlist_id: String,
    name: String,
) -> Result<(), AppError> {
    state.rename_playlist(&playlist_id, &name)
}

#[tauri::command]
fn preview_playlist_removal(
    state: State<'_, AppState>,
    playlist_id: String,
) -> Result<PlaylistRemovalPreview, AppError> {
    state.preview_playlist_removal(&playlist_id)
}

#[tauri::command]
fn remove_playlist(
    state: State<'_, AppState>,
    playlist_id: String,
    expected_state: String,
) -> Result<PlaylistRemovalReport, AppError> {
    state.remove_playlist(&playlist_id, &expected_state)
}

#[tauri::command]
fn place_track_in_playlist(
    state: State<'_, AppState>,
    target_playlist_id: String,
    target_position: u32,
    track_id: String,
    item_id: Option<String>,
    source_playlist_id: Option<String>,
) -> Result<(), AppError> {
    state.place_track_in_playlist(
        &target_playlist_id,
        target_position,
        &track_id,
        item_id.as_deref(),
        source_playlist_id.as_deref(),
    )
}

#[tauri::command]
fn remove_playlist_item(
    state: State<'_, AppState>,
    playlist_id: String,
    item_id: String,
) -> Result<(), AppError> {
    state.remove_playlist_item(&playlist_id, &item_id)
}

#[tauri::command]
async fn plan_playlist_copy(
    state: State<'_, AppState>,
    playlist_id: String,
    destination_root: String,
) -> Result<CopyPlan, AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        file_copy::plan_playlist_copy(&state, &playlist_id, &destination_root)
    })
    .await
    .map_err(|error| AppError::Task(error.to_string()))?
}

#[tauri::command]
async fn execute_playlist_copy(
    state: State<'_, AppState>,
    operation_id: String,
    resolutions: Vec<CopyResolution>,
) -> Result<CopyReport, AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        file_copy::execute_playlist_copy(&state, &operation_id, &resolutions)
    })
    .await
    .map_err(|error| AppError::Task(error.to_string()))?
}

#[tauri::command]
fn list_copy_recoveries(
    state: State<'_, AppState>,
) -> Result<Vec<CopyRecoveryOperation>, AppError> {
    file_copy::list_copy_recoveries(state.inner())
}

#[tauri::command]
async fn recover_copy_operation(
    state: State<'_, AppState>,
    operation_id: String,
    action: String,
) -> Result<CopyRecoveryReport, AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        file_copy::recover_copy_operation(&state, &operation_id, &action)
    })
    .await
    .map_err(|error| AppError::Task(error.to_string()))?
}

#[tauri::command]
async fn preview_playlist_export(
    state: State<'_, AppState>,
    request: PlaylistExportRequest,
) -> Result<PlaylistExportPreview, AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || playlist_export::preview(&state, request))
        .await
        .map_err(|error| AppError::Task(error.to_string()))?
}

#[tauri::command]
async fn commit_playlist_export(
    state: State<'_, AppState>,
    request: PlaylistExportRequest,
    expected_state: String,
) -> Result<PlaylistExportReport, AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        playlist_export::commit(&state, request, &expected_state)
    })
    .await
    .map_err(|error| AppError::Task(error.to_string()))?
}

#[tauri::command]
fn list_analysis_jobs(state: State<'_, AppState>) -> Result<Vec<AnalysisJob>, AppError> {
    background_jobs::list(state.inner())
}

#[tauri::command]
fn enqueue_audio_analysis(
    state: State<'_, AppState>,
    track_ids: Vec<String>,
) -> Result<AnalysisJob, AppError> {
    background_jobs::enqueue(state.inner(), &track_ids)
}

#[tauri::command]
async fn run_analysis_job(
    state: State<'_, AppState>,
    job_id: String,
) -> Result<AnalysisJob, AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || background_jobs::run(&state, &job_id))
        .await
        .map_err(|error| AppError::Task(error.to_string()))?
}

#[tauri::command]
fn pause_analysis_job(state: State<'_, AppState>, job_id: String) -> Result<(), AppError> {
    background_jobs::pause(state.inner(), &job_id)
}

#[tauri::command]
fn cancel_analysis_job(state: State<'_, AppState>, job_id: String) -> Result<(), AppError> {
    background_jobs::cancel(state.inner(), &job_id)
}

#[tauri::command]
fn suggest_track_genres(
    state: State<'_, AppState>,
    track_ids: Vec<String>,
) -> Result<Vec<GenreSuggestion>, AppError> {
    genre_assistant::suggest(state.inner(), &track_ids)
}

#[tauri::command]
fn confirm_track_genre(
    state: State<'_, AppState>,
    track_id: String,
    genre: String,
    subgenre: Option<String>,
    locked: bool,
) -> Result<(), AppError> {
    genre_assistant::confirm(
        state.inner(),
        &track_id,
        &genre,
        subgenre.as_deref(),
        locked,
    )
}

#[tauri::command]
fn audit_genre_dataset(state: State<'_, AppState>) -> Result<GenreDatasetAudit, AppError> {
    genre_dataset::audit(state.inner())
}

#[tauri::command]
async fn export_genre_dataset(
    state: State<'_, AppState>,
    destination_path: String,
) -> Result<GenreDatasetExportReport, AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        genre_dataset::export(&state, PathBuf::from(destination_path).as_path())
    })
    .await
    .map_err(|error| AppError::Task(error.to_string()))?
}

#[tauri::command]
async fn evaluate_latest_genre_model(
    state: State<'_, AppState>,
) -> Result<GenreModelEvaluation, AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || genre_model::evaluate_latest(&state))
        .await
        .map_err(|error| AppError::Task(error.to_string()))?
}

#[tauri::command]
fn get_latest_genre_model_evaluation(
    state: State<'_, AppState>,
) -> Result<Option<GenreModelEvaluation>, AppError> {
    genre_model::latest_evaluation(state.inner())
}

#[tauri::command]
async fn export_latest_genre_model_report(
    state: State<'_, AppState>,
    destination_path: String,
) -> Result<String, AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        genre_model::export_latest_report(&state, PathBuf::from(destination_path).as_path())
    })
    .await
    .map_err(|error| AppError::Task(error.to_string()))?
}

#[tauri::command]
fn list_exact_duplicates(state: State<'_, AppState>) -> Result<Vec<DuplicateGroup>, AppError> {
    duplicates::list_exact_duplicates(state.inner())
}

#[tauri::command]
async fn scan_exact_duplicates(
    state: State<'_, AppState>,
) -> Result<DuplicateScanResult, AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || duplicates::scan_exact_duplicates(&state))
        .await
        .map_err(|error| AppError::Task(error.to_string()))?
}

#[tauri::command]
async fn get_track_metadata(
    state: State<'_, AppState>,
    track_id: String,
) -> Result<TrackMetadataDetail, AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || metadata_editor::detail(&state, &track_id))
        .await
        .map_err(|error| AppError::Task(error.to_string()))?
}

#[tauri::command]
async fn preview_metadata_update(
    state: State<'_, AppState>,
    track_id: String,
    values: MetadataValues,
) -> Result<MetadataPreview, AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        metadata_editor::preview(&state, &track_id, values)
    })
    .await
    .map_err(|error| AppError::Task(error.to_string()))?
}

#[tauri::command]
async fn commit_metadata_update(
    state: State<'_, AppState>,
    track_id: String,
    expected_file_state: String,
    values: MetadataValues,
) -> Result<TrackMetadataDetail, AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        metadata_editor::commit(&state, &track_id, &expected_file_state, values)
    })
    .await
    .map_err(|error| AppError::Task(error.to_string()))?
}

#[tauri::command]
async fn preview_bulk_metadata_update(
    state: State<'_, AppState>,
    request: BulkMetadataRequest,
) -> Result<BulkMetadataPreview, AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || metadata_editor::bulk_preview(&state, request))
        .await
        .map_err(|error| AppError::Task(error.to_string()))?
}

#[tauri::command]
async fn commit_bulk_metadata_update(
    state: State<'_, AppState>,
    items: Vec<BulkMetadataCommitItem>,
) -> Result<BulkMetadataReport, AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || metadata_editor::bulk_commit(&state, items))
        .await
        .map_err(|error| AppError::Task(error.to_string()))?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let preferred_installed_data_dir = app.path().local_data_dir()?.join("SortTrack");
            let legacy_installed_data_dir = app.path().app_local_data_dir()?;
            let installed_data_dir = runtime_profile::select_installed_data_root(
                &preferred_installed_data_dir,
                &legacy_installed_data_dir,
            )
            .map_err(|error| std::io::Error::other(error.to_string()))?;
            let executable = std::env::current_exe()?;
            let profile = runtime_profile::resolve(&installed_data_dir, &executable)
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            let diagnostics = DiagnosticsState::new(&profile)
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            diagnostics.install_panic_hook();
            diagnostics.info(
                "application_start",
                "SortTrack started; automatic diagnostic upload is disabled",
            );
            if profile.data_root == legacy_installed_data_dir {
                diagnostics.warn(
                    "legacy_storage_selected",
                    "Existing legacy application storage was selected to preserve user data",
                );
            }
            let database_dir = profile.data_root.join("database");
            std::fs::create_dir_all(&database_dir)?;
            restore::apply_pending(&database_dir)
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            let state = AppState::open(&database_dir.join("sorttrack.db"))
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            background_jobs::recover_supported_jobs(&state)
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            diagnostics.info("database_ready", "Local database is ready");
            app.manage(profile);
            app.manage(diagnostics);
            app.manage(ScanRuntimeState::default());
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_library_summary,
            list_library_roots,
            set_library_root_enabled,
            refresh_library_root_availability,
            preview_library_root_removal,
            remove_library_root_registration,
            get_library_table_layout,
            save_library_table_layout,
            get_runtime_profile,
            preview_diagnostics,
            export_diagnostics,
            open_log_directory,
            clear_diagnostics,
            create_backup,
            preview_restore,
            prepare_restore_and_restart,
            list_tracks,
            search_tracks,
            locate_track_page,
            preview_missing_file_relink,
            commit_missing_file_relink,
            preview_music_folder,
            scan_music_folder,
            get_scan_progress,
            pause_scan,
            resume_scan,
            cancel_scan,
            prepare_track_playback,
            get_track_waveform,
            prepare_track_cover,
            set_track_rating,
            bulk_set_track_rating,
            list_playlists,
            create_playlist,
            create_smart_playlist,
            snapshot_smart_playlist,
            update_smart_playlist_rule,
            rename_playlist,
            preview_playlist_removal,
            remove_playlist,
            place_track_in_playlist,
            remove_playlist_item,
            plan_playlist_copy,
            execute_playlist_copy,
            list_copy_recoveries,
            recover_copy_operation,
            preview_playlist_export,
            commit_playlist_export,
            list_analysis_jobs,
            enqueue_audio_analysis,
            run_analysis_job,
            pause_analysis_job,
            cancel_analysis_job,
            suggest_track_genres,
            confirm_track_genre,
            audit_genre_dataset,
            export_genre_dataset,
            evaluate_latest_genre_model,
            get_latest_genre_model_evaluation,
            export_latest_genre_model_report,
            list_exact_duplicates,
            scan_exact_duplicates,
            get_track_metadata,
            preview_metadata_update,
            commit_metadata_update,
            preview_bulk_metadata_update,
            commit_bulk_metadata_update
        ])
        .run(tauri::generate_context!())
        .expect("error while running SortTrack");
}
