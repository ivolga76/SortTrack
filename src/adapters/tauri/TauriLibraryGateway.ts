import type { AnalysisJob, BackupReport, BulkMetadataCommitItem, BulkMetadataPreview, BulkMetadataReport, BulkMetadataRequest, BulkRatingReport, CopyPlan, CopyRecoveryOperation, CopyRecoveryReport, CopyReport, CopyResolution, CoverData, DiagnosticsExportReport, DiagnosticsPreview, DuplicateGroup, DuplicateScanResult, GenreDatasetAudit, GenreDatasetExportReport, GenreModelEvaluation, GenreSuggestion, LibraryRootSettings, LibrarySummary, LibraryTableLayout, MetadataPreview, MetadataValues, Playlist, PlaylistExportFormat, PlaylistExportPathMode, PlaylistExportPreview, PlaylistExportReport, PlaylistExportRequest, PlaylistPlacement, RelinkPreview, RelinkReport, RestorePrepared, RestorePreview, RuntimeProfile, ScanOptions, ScanPreview, ScanProgress, ScanResult, Track, TrackMetadataDetail, TrackPage, TrackQuery, TrackRatingHistoryEntry, TraktorTargetProfile, WaveformData } from "../../domain/models";
import type { LibraryRootRemovalPreview, LibraryRootRemovalReport } from "../../domain/models";
import type { PlaylistRemovalPreview, PlaylistRemovalReport } from "../../domain/models";
import type { PlaylistSnapshotReport, SmartPlaylistRule } from "../../domain/models";
import { convertFileSrc, invoke as tauriInvoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { AppGateway } from "../../shared/api/ports";
import { normalizeGatewayError } from "../../shared/api/errors";

async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await tauriInvoke<T>(command, args);
  } catch (error) {
    throw normalizeGatewayError(error, command);
  }
}

export class TauriLibraryGateway implements AppGateway {
  readonly isDesktop = true;

  getSummary(): Promise<LibrarySummary> {
    return invoke<LibrarySummary>("get_library_summary");
  }

  listLibraryRoots(): Promise<LibraryRootSettings[]> {
    return invoke<LibraryRootSettings[]>("list_library_roots");
  }
  refreshLibraryRootAvailability(): Promise<LibraryRootSettings[]> {
    return invoke<LibraryRootSettings[]>("refresh_library_root_availability");
  }
  previewLibraryRootRemoval(rootId: string): Promise<LibraryRootRemovalPreview> {
    return invoke<LibraryRootRemovalPreview>("preview_library_root_removal", { rootId });
  }
  removeLibraryRootRegistration(preview: LibraryRootRemovalPreview): Promise<LibraryRootRemovalReport> {
    return invoke<LibraryRootRemovalReport>("remove_library_root_registration", { rootId: preview.rootId, expectedLinkedFiles: preview.linkedFiles });
  }

  setLibraryRootEnabled(rootId: string, expectedStatus: LibraryRootSettings["status"], enabled: boolean): Promise<void> {
    return invoke<void>("set_library_root_enabled", { rootId, expectedStatus, enabled });
  }

  getLibraryTableLayout(): Promise<LibraryTableLayout> {
    return invoke<LibraryTableLayout>("get_library_table_layout");
  }

  saveLibraryTableLayout(layout: LibraryTableLayout): Promise<LibraryTableLayout> {
    return invoke<LibraryTableLayout>("save_library_table_layout", { layout });
  }

  getRuntimeProfile(): Promise<RuntimeProfile> {
    return invoke<RuntimeProfile>("get_runtime_profile");
  }

  previewDiagnostics(redactPaths: boolean): Promise<DiagnosticsPreview> {
    return invoke<DiagnosticsPreview>("preview_diagnostics", { redactPaths });
  }

  async pickAndExportDiagnostics(preview: DiagnosticsPreview, redactPaths: boolean): Promise<DiagnosticsExportReport | null> {
    const destinationPath = await save({
      title: "Сохранить локальный диагностический пакет",
      defaultPath: `SortTrack-diagnostics-${new Date().toISOString().slice(0, 10)}.json`,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!destinationPath) return null;
    return invoke<DiagnosticsExportReport>("export_diagnostics", { destinationPath, expectedState: preview.expectedState, redactPaths });
  }

  openLogDirectory(): Promise<void> {
    return invoke<void>("open_log_directory");
  }

  clearDiagnostics(): Promise<number> {
    return invoke<number>("clear_diagnostics");
  }

  async pickAndCreateBackup(): Promise<BackupReport | null> {
    const destinationRoot = await open({ directory: true, multiple: false, title: "Выберите папку для резервной копии SortTrack" });
    if (!destinationRoot || Array.isArray(destinationRoot)) return null;
    return invoke<BackupReport>("create_backup", { destinationRoot });
  }

  async pickAndPreviewRestore(): Promise<RestorePreview | null> {
    const directoryPath = await open({ directory: true, multiple: false, title: "Выберите папку резервной копии SortTrack" });
    if (!directoryPath || Array.isArray(directoryPath)) return null;
    return invoke<RestorePreview>("preview_restore", { directoryPath });
  }

  prepareRestoreAndRestart(preview: RestorePreview): Promise<RestorePrepared> {
    return invoke<RestorePrepared>("prepare_restore_and_restart", { directoryPath: preview.backupPath, expectedManifestSha256: preview.manifestSha256 });
  }

  searchTracks(query: TrackQuery): Promise<TrackPage> {
    return invoke<TrackPage>("search_tracks", {
      limit: query.limit,
      offset: query.offset,
      search: query.text,
      genre: query.genre,
      onlyIncoming: query.onlyIncoming,
      minBpm: query.minBpm,
      maxBpm: query.maxBpm,
      minRating: query.minRating,
      camelot: query.camelot,
      format: query.format,
      fileStatus: query.fileStatus,
      sortBy: query.sortBy,
      descending: query.descending,
    });
  }

  locateTrackPage(trackId: string, pageSize: number): Promise<number | null> {
    return invoke<number | null>("locate_track_page", { trackId, pageSize });
  }

  async pickAndPreviewScanFolder(): Promise<ScanPreview | null> {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Выберите папку с MP3 и FLAC",
    });
    if (!selected || Array.isArray(selected)) return null;
    return this.previewScanFolder(selected, { recursive: true, exclusions: [] });
  }

  previewScanFolder(rootPath: string, options: ScanOptions): Promise<ScanPreview> {
    return invoke<ScanPreview>("preview_music_folder", { path: rootPath, options });
  }

  scanFolder(preview: ScanPreview): Promise<ScanResult> {
    return invoke<ScanResult>("scan_music_folder", { path: preview.rootPath, options: { recursive: preview.recursive, exclusions: preview.exclusions }, expectedTotal: preview.discovered });
  }

  getScanProgress(): Promise<ScanProgress | null> { return invoke<ScanProgress | null>("get_scan_progress"); }
  pauseScan(jobId: string): Promise<void> { return invoke<void>("pause_scan", { jobId }); }
  resumeScan(jobId: string): Promise<void> { return invoke<void>("resume_scan", { jobId }); }
  cancelScan(jobId: string): Promise<void> { return invoke<void>("cancel_scan", { jobId }); }

  async pickAndPreviewRelink(trackId: string): Promise<RelinkPreview | null> {
    const selectedPath = await open({
      directory: false,
      multiple: false,
      title: "Найдите MP3 или FLAC для сохранённой записи",
      filters: [{ name: "Музыка", extensions: ["mp3", "flac"] }],
    });
    if (!selectedPath || Array.isArray(selectedPath)) return null;
    return invoke<RelinkPreview>("preview_missing_file_relink", { trackId, selectedPath });
  }

  commitRelink(preview: RelinkPreview): Promise<RelinkReport> {
    return invoke<RelinkReport>("commit_missing_file_relink", {
      trackId: preview.trackId,
      selectedPath: preview.newPath,
      expectedState: preview.expectedState,
    });
  }

  async preparePlayback(trackId: string): Promise<string> {
    const path = await invoke<string>("prepare_track_playback", { trackId });
    return convertFileSrc(path);
  }

  getTrackWaveform(trackId: string): Promise<WaveformData> {
    return invoke<WaveformData>("get_track_waveform", { trackId });
  }

  async getTrackCover(trackId: string): Promise<string | null> {
    const result = await invoke<CoverData>("prepare_track_cover", { trackId });
    return result.coverPath ? convertFileSrc(result.coverPath) : null;
  }

  setTrackRating(trackId: string, expectedRating: number, nextRating: number): Promise<void> {
    return invoke<void>("set_track_rating", { trackId, expectedRating, nextRating });
  }

  bulkSetTrackRating(trackIds: string[], targetRating: number): Promise<BulkRatingReport> {
    return invoke<BulkRatingReport>("bulk_set_track_rating", { trackIds, targetRating });
  }

  listPlaylists(): Promise<Playlist[]> {
    return invoke<Playlist[]>("list_playlists");
  }

  createPlaylist(name: string): Promise<string> {
    return invoke<string>("create_playlist", { name });
  }

  createSmartPlaylist(name: string, rule: SmartPlaylistRule): Promise<string> {
    return invoke<string>("create_smart_playlist", { name, rule });
  }

  snapshotSmartPlaylist(smartPlaylistId: string, name: string): Promise<PlaylistSnapshotReport> {
    return invoke<PlaylistSnapshotReport>("snapshot_smart_playlist", { smartPlaylistId, name });
  }

  updateSmartPlaylistRule(smartPlaylistId: string, expectedRule: SmartPlaylistRule, nextRule: SmartPlaylistRule): Promise<void> {
    return invoke<void>("update_smart_playlist_rule", { smartPlaylistId, expectedRule, nextRule });
  }

  renamePlaylist(playlistId: string, name: string): Promise<void> {
    return invoke<void>("rename_playlist", { playlistId, name });
  }

  previewPlaylistRemoval(playlistId: string): Promise<PlaylistRemovalPreview> {
    return invoke<PlaylistRemovalPreview>("preview_playlist_removal", { playlistId });
  }
  removePlaylist(preview: PlaylistRemovalPreview): Promise<PlaylistRemovalReport> {
    return invoke<PlaylistRemovalReport>("remove_playlist", { playlistId: preview.playlistId, expectedState: preview.expectedState });
  }

  placeTrack(placement: PlaylistPlacement): Promise<void> {
    return invoke<void>("place_track_in_playlist", {
      targetPlaylistId: placement.targetPlaylistId,
      targetPosition: placement.targetPosition,
      trackId: placement.trackId,
      itemId: placement.itemId,
      sourcePlaylistId: placement.sourcePlaylistId,
    });
  }

  removePlaylistItem(playlistId: string, itemId: string): Promise<void> {
    return invoke<void>("remove_playlist_item", { playlistId, itemId });
  }

  async pickAndPlanPlaylistCopy(playlistId: string): Promise<CopyPlan | null> {
    const destinationRoot = await open({
      directory: true,
      multiple: false,
      title: "Выберите папку для безопасной копии",
    });
    if (!destinationRoot || Array.isArray(destinationRoot)) return null;
    return invoke<CopyPlan>("plan_playlist_copy", { playlistId, destinationRoot });
  }

  executePlaylistCopy(operationId: string, resolutions: CopyResolution[]): Promise<CopyReport> {
    return invoke<CopyReport>("execute_playlist_copy", { operationId, resolutions });
  }

  listCopyRecoveries(): Promise<CopyRecoveryOperation[]> {
    return invoke<CopyRecoveryOperation[]>("list_copy_recoveries");
  }

  recoverCopyOperation(operationId: string, action: "resume" | "cancel_safe"): Promise<CopyRecoveryReport> {
    return invoke<CopyRecoveryReport>("recover_copy_operation", { operationId, action });
  }

  async pickAndPreviewPlaylistExport(playlistId: string, format: PlaylistExportFormat, pathMode: PlaylistExportPathMode, targetProfile: TraktorTargetProfile | null): Promise<PlaylistExportPreview | null> {
    const extension = format === "m3u8" ? "m3u8" : "nml";
    const destinationPath = await save({ title: "Сохранить отдельный файл плейлиста", filters: [{ name: format === "m3u8" ? "M3U8 playlist" : "Traktor NML", extensions: [extension] }] });
    if (!destinationPath) return null;
    return invoke<PlaylistExportPreview>("preview_playlist_export", { request: { playlistId, destinationPath, format, pathMode: format === "traktor_nml" ? "absolute" : pathMode, targetProfile: format === "traktor_nml" ? targetProfile : null } });
  }

  commitPlaylistExport(request: PlaylistExportRequest, expectedState: string): Promise<PlaylistExportReport> {
    return invoke<PlaylistExportReport>("commit_playlist_export", { request, expectedState });
  }

  listAnalysisJobs(): Promise<AnalysisJob[]> { return invoke<AnalysisJob[]>("list_analysis_jobs"); }
  enqueueAudioAnalysis(trackIds: string[]): Promise<AnalysisJob> { return invoke<AnalysisJob>("enqueue_audio_analysis", { trackIds }); }
  runAnalysisJob(jobId: string): Promise<AnalysisJob> { return invoke<AnalysisJob>("run_analysis_job", { jobId }); }
  pauseAnalysisJob(jobId: string): Promise<void> { return invoke<void>("pause_analysis_job", { jobId }); }
  cancelAnalysisJob(jobId: string): Promise<void> { return invoke<void>("cancel_analysis_job", { jobId }); }
  suggestTrackGenres(trackIds: string[]): Promise<GenreSuggestion[]> { return invoke<GenreSuggestion[]>("suggest_track_genres", { trackIds }); }
  confirmTrackGenre(trackId: string, genre: string, subgenre: string | null, locked: boolean): Promise<void> { return invoke<void>("confirm_track_genre", { trackId, genre, subgenre, locked }); }
  auditGenreDataset(): Promise<GenreDatasetAudit> { return invoke<GenreDatasetAudit>("audit_genre_dataset"); }
  async pickAndExportGenreDataset(): Promise<GenreDatasetExportReport | null> {
    const destinationPath = await save({ title: "Сохранить manifest validation dataset", filters: [{ name: "SortTrack dataset manifest", extensions: ["json"] }] });
    if (!destinationPath) return null;
    return invoke<GenreDatasetExportReport>("export_genre_dataset", { destinationPath });
  }
  evaluateLatestGenreModel(): Promise<GenreModelEvaluation> { return invoke<GenreModelEvaluation>("evaluate_latest_genre_model"); }
  getLatestGenreModelEvaluation(): Promise<GenreModelEvaluation | null> { return invoke<GenreModelEvaluation | null>("get_latest_genre_model_evaluation"); }
  async pickAndExportLatestModelReport(): Promise<string | null> {
    const destinationPath = await save({ title: "Сохранить отчёт проверки модели", filters: [{ name: "SortTrack model report", extensions: ["json"] }] });
    if (!destinationPath) return null;
    return invoke<string>("export_latest_genre_model_report", { destinationPath });
  }

  listDuplicateGroups(): Promise<DuplicateGroup[]> {
    return invoke<DuplicateGroup[]>("list_exact_duplicates");
  }

  scanExactDuplicates(): Promise<DuplicateScanResult> {
    return invoke<DuplicateScanResult>("scan_exact_duplicates");
  }

  getTrackMetadata(trackId: string): Promise<TrackMetadataDetail> {
    return invoke<TrackMetadataDetail>("get_track_metadata", { trackId });
  }

  previewMetadataUpdate(trackId: string, values: MetadataValues): Promise<MetadataPreview> {
    return invoke<MetadataPreview>("preview_metadata_update", { trackId, values });
  }

  commitMetadataUpdate(trackId: string, expectedFileState: string, values: MetadataValues): Promise<TrackMetadataDetail> {
    return invoke<TrackMetadataDetail>("commit_metadata_update", { trackId, expectedFileState, values });
  }

  previewBulkMetadataUpdate(request: BulkMetadataRequest): Promise<BulkMetadataPreview> {
    return invoke<BulkMetadataPreview>("preview_bulk_metadata_update", { request });
  }

  commitBulkMetadataUpdate(items: BulkMetadataCommitItem[]): Promise<BulkMetadataReport> {
    return invoke<BulkMetadataReport>("commit_bulk_metadata_update", { items });
  }
}
