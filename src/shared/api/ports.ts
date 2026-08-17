import type {
  AnalysisJob,
  BackupReport,
  BulkMetadataCommitItem,
  BulkMetadataPreview,
  BulkMetadataReport,
  BulkMetadataRequest,
  BulkRatingReport,
  CopyPlan,
  CopyRecoveryOperation,
  CopyRecoveryReport,
  CopyReport,
  CopyResolution,
  DiagnosticsExportReport,
  DiagnosticsPreview,
  DuplicateGroup,
  DuplicateScanResult,
  GenreDatasetAudit,
  GenreDatasetExportReport,
  GenreModelEvaluation,
  GenreSuggestion,
  LibraryRootRemovalPreview,
  LibraryRootRemovalReport,
  LibraryRootSettings,
  LibrarySummary,
  LibraryTableLayout,
  MetadataPreview,
  MetadataValues,
  Playlist,
  PlaylistExportFormat,
  PlaylistExportPathMode,
  PlaylistExportPreview,
  PlaylistExportReport,
  PlaylistExportRequest,
  PlaylistPlacement,
  PlaylistRemovalPreview,
  PlaylistRemovalReport,
  PlaylistSnapshotReport,
  RelinkPreview,
  RelinkReport,
  RestorePrepared,
  RestorePreview,
  RuntimeProfile,
  ScanOptions,
  ScanPreview,
  ScanProgress,
  ScanResult,
  SmartPlaylistRule,
  TrackMetadataDetail,
  TrackPage,
  TrackQuery,
  TraktorTargetProfile,
  WaveformData,
} from "../../domain/models";
import type { MetadataFileState, PlaylistExportState } from "../../domain/stateTokens";

export interface RuntimeCapability {
  readonly isDesktop: boolean;
}

export interface LibraryBrowsePort {
  getSummary(): Promise<LibrarySummary>;
  searchTracks(query: TrackQuery): Promise<TrackPage>;
  locateTrackPage(trackId: string, pageSize: number): Promise<number | null>;
  getLibraryTableLayout(): Promise<LibraryTableLayout>;
  saveLibraryTableLayout(layout: LibraryTableLayout): Promise<LibraryTableLayout>;
}

export interface TrackRatingPort {
  setTrackRating(trackId: string, expectedRating: number, nextRating: number): Promise<void>;
  bulkSetTrackRating(trackIds: string[], targetRating: number): Promise<BulkRatingReport>;
}

export interface LibraryScanPort extends RuntimeCapability {
  listLibraryRoots(): Promise<LibraryRootSettings[]>;
  refreshLibraryRootAvailability(): Promise<LibraryRootSettings[]>;
  previewLibraryRootRemoval(rootId: string): Promise<LibraryRootRemovalPreview>;
  removeLibraryRootRegistration(preview: LibraryRootRemovalPreview): Promise<LibraryRootRemovalReport>;
  setLibraryRootEnabled(rootId: string, expectedStatus: LibraryRootSettings["status"], enabled: boolean): Promise<void>;
  pickAndPreviewScanFolder(): Promise<ScanPreview | null>;
  previewScanFolder(rootPath: string, options: ScanOptions): Promise<ScanPreview>;
  scanFolder(preview: ScanPreview): Promise<ScanResult>;
  getScanProgress(): Promise<ScanProgress | null>;
  pauseScan(jobId: string): Promise<void>;
  resumeScan(jobId: string): Promise<void>;
  cancelScan(jobId: string): Promise<void>;
}

export interface PlaybackPort {
  preparePlayback(trackId: string): Promise<string | null>;
  getTrackWaveform(trackId: string): Promise<WaveformData | null>;
  getTrackCover(trackId: string): Promise<string | null>;
}

export interface PlaylistPort {
  listPlaylists(): Promise<Playlist[]>;
  createPlaylist(name: string): Promise<string>;
  createSmartPlaylist(name: string, rule: SmartPlaylistRule): Promise<string>;
  snapshotSmartPlaylist(smartPlaylistId: string, name: string): Promise<PlaylistSnapshotReport>;
  updateSmartPlaylistRule(smartPlaylistId: string, expectedRule: SmartPlaylistRule, nextRule: SmartPlaylistRule): Promise<void>;
  renamePlaylist(playlistId: string, name: string): Promise<void>;
  previewPlaylistRemoval(playlistId: string): Promise<PlaylistRemovalPreview>;
  removePlaylist(preview: PlaylistRemovalPreview): Promise<PlaylistRemovalReport>;
  placeTrack(placement: PlaylistPlacement): Promise<void>;
  removePlaylistItem(playlistId: string, itemId: string): Promise<void>;
}

export interface FileCopyPort extends RuntimeCapability {
  pickAndPlanPlaylistCopy(playlistId: string): Promise<CopyPlan | null>;
  executePlaylistCopy(operationId: string, resolutions: CopyResolution[]): Promise<CopyReport>;
  listCopyRecoveries(): Promise<CopyRecoveryOperation[]>;
  recoverCopyOperation(operationId: string, action: "resume" | "cancel_safe"): Promise<CopyRecoveryReport>;
}

export interface PlaylistExportPort {
  pickAndPreviewPlaylistExport(playlistId: string, format: PlaylistExportFormat, pathMode: PlaylistExportPathMode, targetProfile: TraktorTargetProfile | null): Promise<PlaylistExportPreview | null>;
  commitPlaylistExport(request: PlaylistExportRequest, expectedState: PlaylistExportState): Promise<PlaylistExportReport>;
}

export interface AnalysisPort {
  listAnalysisJobs(): Promise<AnalysisJob[]>;
  enqueueAudioAnalysis(trackIds: string[]): Promise<AnalysisJob>;
  runAnalysisJob(jobId: string): Promise<AnalysisJob>;
  pauseAnalysisJob(jobId: string): Promise<void>;
  cancelAnalysisJob(jobId: string): Promise<void>;
  suggestTrackGenres(trackIds: string[]): Promise<GenreSuggestion[]>;
  confirmTrackGenre(trackId: string, genre: string, subgenre: string | null, locked: boolean): Promise<void>;
  auditGenreDataset(): Promise<GenreDatasetAudit>;
  pickAndExportGenreDataset(): Promise<GenreDatasetExportReport | null>;
  evaluateLatestGenreModel(): Promise<GenreModelEvaluation>;
  getLatestGenreModelEvaluation(): Promise<GenreModelEvaluation | null>;
  pickAndExportLatestModelReport(): Promise<string | null>;
}

export interface DuplicatesPort {
  listDuplicateGroups(): Promise<DuplicateGroup[]>;
  scanExactDuplicates(): Promise<DuplicateScanResult>;
}

export interface MetadataPort extends RuntimeCapability {
  pickAndPreviewRelink(trackId: string): Promise<RelinkPreview | null>;
  commitRelink(preview: RelinkPreview): Promise<RelinkReport>;
  getTrackMetadata(trackId: string): Promise<TrackMetadataDetail>;
  previewMetadataUpdate(trackId: string, values: MetadataValues): Promise<MetadataPreview>;
  commitMetadataUpdate(trackId: string, expectedFileState: MetadataFileState, values: MetadataValues): Promise<TrackMetadataDetail>;
  previewBulkMetadataUpdate(request: BulkMetadataRequest): Promise<BulkMetadataPreview>;
  commitBulkMetadataUpdate(items: BulkMetadataCommitItem[]): Promise<BulkMetadataReport>;
}

export interface SettingsPort extends RuntimeCapability {
  getRuntimeProfile(): Promise<RuntimeProfile>;
  previewDiagnostics(redactPaths: boolean): Promise<DiagnosticsPreview>;
  pickAndExportDiagnostics(preview: DiagnosticsPreview, redactPaths: boolean): Promise<DiagnosticsExportReport | null>;
  openLogDirectory(): Promise<void>;
  clearDiagnostics(): Promise<number>;
  pickAndCreateBackup(): Promise<BackupReport | null>;
  pickAndPreviewRestore(): Promise<RestorePreview | null>;
  prepareRestoreAndRestart(preview: RestorePreview): Promise<RestorePrepared>;
}

export interface AppGateway extends
  RuntimeCapability,
  LibraryBrowsePort,
  TrackRatingPort,
  LibraryScanPort,
  PlaybackPort,
  PlaylistPort,
  FileCopyPort,
  PlaylistExportPort,
  AnalysisPort,
  DuplicatesPort,
  MetadataPort,
  SettingsPort {}

/** @deprecated Use a feature-specific port or AppGateway only at bootstrap. */
export interface LibraryGateway extends AppGateway {}
