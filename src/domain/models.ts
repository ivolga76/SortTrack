import type { DiagnosticsState, MetadataFileState, PlaylistExportState, PlaylistRemovalState, RelinkState } from "./stateTokens";

export type AnalysisState = "ready" | "queued" | "needs-review" | "missing";

export interface Track {
  id: string;
  title: string;
  artist: string;
  album: string;
  genre: string | null;
  subgenre: string | null;
  bpm: number | null;
  musicalKey: string | null;
  camelot: string | null;
  durationSeconds: number;
  format: "MP3" | "FLAC";
  bitrateKbps: number | null;
  path: string;
  rating: number;
  addedAt: string;
  analysisState: AnalysisState;
  coverColor: string;
}

export interface LibrarySummary {
  tracks: number;
  incoming: number;
  needsReview: number;
  duplicates: number;
}

export interface LibraryRootSettings {
  id: string;
  path: string;
  recursive: boolean;
  exclusions: string[];
  status: "active" | "unavailable" | "disabled";
  lastScannedAt: string | null;
}

export interface LibraryRootRemovalPreview {
  rootId: string;
  path: string;
  linkedFiles: number;
}

export interface LibraryRootRemovalReport {
  rootId: string;
  path: string;
  detachedFiles: number;
}

export interface PlaylistRemovalPreview {
  playlistId: string;
  name: string;
  itemCount: number;
  expectedState: PlaylistRemovalState;
}

export interface PlaylistRemovalReport {
  playlistId: string;
  name: string;
  removedItems: number;
}

export interface ScanError {
  path: string;
  reason: string;
}

export interface ScanOptions {
  recursive: boolean;
  exclusions: string[];
}

export interface ScanPreview extends ScanOptions {
  rootPath: string;
  discovered: number;
  warnings: ScanError[];
}

export interface ScanResult {
  rootPath: string;
  discovered: number;
  added: number;
  updated: number;
  moved: number;
  restored: number;
  missing: number;
  skipped: number;
  errors: ScanError[];
  cancelled: boolean;
}

export interface ScanProgress {
  id: string;
  rootPath: string;
  state: "running" | "paused" | "cancelling" | "completed" | "cancelled" | "failed";
  processedItems: number;
  totalItems: number;
  itemsPerSecond: number | null;
  etaSeconds: number | null;
  elapsedSeconds: number;
}

export type LibraryColumnId = "track" | "genre" | "bpm" | "key" | "format" | "duration";

export interface LibraryColumnLayout {
  id: LibraryColumnId;
  visible: boolean;
  width: number;
}

export interface LibraryTableLayout {
  columns: LibraryColumnLayout[];
}

export interface RuntimeProfile {
  mode: "installed" | "portable" | "browser";
  dataRoot: string;
}

export interface BackupCounts {
  tracks: number;
  trackFiles: number;
  playlists: number;
  metadataHistory: number;
  genreHistory: number;
  datasetExports: number;
  modelRuns: number;
}

export interface BackupReport {
  backupPath: string;
  databasePath: string;
  manifestPath: string;
  databaseSha256: string;
  manifestSha256: string;
  databaseSizeBytes: number;
  databaseSchemaVersion: number;
  counts: BackupCounts;
}

export interface RestorePreview {
  backupPath: string;
  createdAt: string;
  applicationVersion: string;
  databaseSha256: string;
  manifestSha256: string;
  databaseSizeBytes: number;
  databaseSchemaVersion: number;
  currentSchemaVersion: number;
  requiresMigration: boolean;
  counts: BackupCounts;
}

export interface RestorePrepared {
  backupPath: string;
  safetyBackupPath: string;
  expectedDatabaseSha256: string;
  restartRequired: boolean;
}

export interface TrackQuery {
  text: string;
  genre: string | null;
  onlyIncoming: boolean;
  minBpm: number | null;
  maxBpm: number | null;
  minRating: number | null;
  camelot: string | null;
  format: "mp3" | "flac" | null;
  fileStatus: "all" | "available" | "problems" | "missing";
  limit: number;
  offset: number;
  sortBy: "added" | "title" | "artist" | "bpm";
  descending: boolean;
}

export interface TrackPage {
  items: Track[];
  total: number;
  limit: number;
  offset: number;
}

export interface WaveformData {
  peaks: number[];
  cacheHit: boolean;
  algorithmVersion: string;
}

export interface CoverData {
  coverPath: string | null;
  mimeType: string | null;
  cacheHit: boolean;
}

export interface PlaylistItem {
  id: string;
  position: number;
  track: Track;
}

export interface Playlist {
  id: string;
  name: string;
  kind: "manual" | "smart";
  smartRule: SmartPlaylistRule | null;
  items: PlaylistItem[];
}

export interface SmartPlaylistRule {
  genre: string | null;
  subgenre: string | null;
  minBpm: number | null;
  maxBpm: number | null;
  minRating: number | null;
  camelot: string | null;
  limitCount: number;
  sortBy: "added" | "title" | "bpm";
  descending: boolean;
}

export interface PlaylistSnapshotReport {
  playlistId: string;
  name: string;
  itemCount: number;
}

export interface PlaylistPlacement {
  targetPlaylistId: string;
  targetPosition: number;
  trackId: string;
  itemId?: string;
  sourcePlaylistId?: string;
}

export type CopyPlanStatus = "ready" | "identical" | "conflict" | "missing" | "invalid" | "permission_error";
export type CopyConflictAction = "skip" | "rename";

export interface CopyPlanItem {
  id: string;
  trackTitle: string;
  sourcePath: string;
  destinationPath: string;
  sizeBytes: number;
  status: CopyPlanStatus;
  reason: string;
}

export interface CopyPlan {
  operationId: string;
  destinationRoot: string;
  totalFiles: number;
  totalBytes: number;
  requiredBytes: number;
  availableBytes: number;
  items: CopyPlanItem[];
}

export interface CopyResolution {
  itemId: string;
  action: CopyConflictAction;
}

export interface CopyReportItem {
  trackTitle: string;
  destinationPath: string;
  result: "copied" | "used_existing" | "skipped" | "failed";
  reason: string;
}

export interface CopyReport {
  operationId: string;
  copied: number;
  usedExisting: number;
  skipped: number;
  failed: number;
  bytesCopied: number;
  items: CopyReportItem[];
}

export interface CopyRecoveryItem {
  id: string;
  trackTitle: string;
  sourcePath: string;
  destinationPath: string;
  temporaryPath: string | null;
  state: string;
  reason: string;
  recommendedAction: string;
}

export interface CopyRecoveryOperation {
  operationId: string;
  destinationRoot: string;
  state: string;
  createdAt: string;
  totalItems: number;
  completedItems: number;
  pendingItems: number;
  items: CopyRecoveryItem[];
}

export interface CopyRecoveryReportItem {
  trackTitle: string;
  destinationPath: string;
  result: "recovered" | "cancelled" | "failed";
  reason: string;
}

export interface CopyRecoveryReport {
  operationId: string;
  recovered: number;
  cancelled: number;
  failed: number;
  items: CopyRecoveryReportItem[];
}

export type PlaylistExportFormat = "m3u8" | "traktor_nml";
export type PlaylistExportPathMode = "absolute" | "relative";
export type TraktorTargetProfile = "traktor_pro_3" | "traktor_pro_4";

export interface PlaylistExportRequest {
  playlistId: string;
  destinationPath: string;
  format: PlaylistExportFormat;
  pathMode: PlaylistExportPathMode;
  targetProfile: TraktorTargetProfile | null;
}

export interface PlaylistExportPreviewItem {
  position: number;
  title: string;
  artist: string;
  sourcePath: string;
  outputLocation: string;
  status: "ready" | "missing";
  reason: string;
}

export interface PlaylistExportPreview extends PlaylistExportRequest {
  playlistName: string;
  expectedState: PlaylistExportState;
  totalTracks: number;
  missingTracks: number;
  destinationExists: boolean;
  items: PlaylistExportPreviewItem[];
}

export interface PlaylistExportReport {
  outputPath: string;
  format: PlaylistExportFormat;
  totalTracks: number;
  missingTracks: number;
  bytesWritten: number;
  instructions: string;
}

export type AnalysisJobState = "queued" | "running" | "paused" | "completed" | "failed" | "cancelled" | "recovery_required";

export interface AnalysisJob {
  id: string;
  state: AnalysisJobState;
  totalItems: number;
  processedItems: number;
  failedItems: number;
  processedBytes: number;
  totalBytes: number;
  createdAt: string;
  error: string | null;
}

export interface GenreCandidate {
  rank: number;
  genre: string;
  subgenre: string | null;
  score: number;
  signals: string[];
}

export interface GenreSuggestion {
  trackId: string;
  title: string;
  artist: string;
  confirmedGenre: string | null;
  confirmedSubgenre: string | null;
  locked: boolean;
  candidates: GenreCandidate[];
  inferenceVersion: string;
  taxonomyVersion: number;
}

export interface DatasetClassCount {
  genre: string;
  tracks: number;
}

export interface GenreDatasetAudit {
  confirmedTracks: number;
  lockedTracks: number;
  eligibleTracks: number;
  unavailableTracks: number;
  targetTracks: number;
  classes: DatasetClassCount[];
  missingGenres: string[];
  ready: boolean;
  warnings: string[];
}

export interface GenreDatasetExportReport {
  outputPath: string;
  manifestSha256: string;
  totalEntries: number;
  trainEntries: number;
  calibrationEntries: number;
  testEntries: number;
  hashedFiles: number;
  cachedHashes: number;
  collapsedRecordings: number;
  conflictingRecordings: number;
  classCounts: DatasetClassCount[];
  warnings: string[];
}

export interface GenreClassMetric {
  genre: string;
  support: number;
  correct: number;
  recall: number;
}

export interface ConfusionCell {
  actual: string;
  predicted: string;
  count: number;
}

export interface GenreModelEvaluation {
  runId: string;
  modelVersion: string;
  featureVersion: string;
  datasetManifestSha256: string;
  top1Accuracy: number;
  meetsReleaseThreshold: boolean;
  trainEntries: number;
  calibrationEntries: number;
  testEntries: number;
  failedFeatures: number;
  classMetrics: GenreClassMetric[];
  confusion: ConfusionCell[];
  warnings: string[];
}

export interface DuplicateFile {
  fileId: string;
  trackId: string;
  title: string;
  artist: string;
  format: "MP3" | "FLAC";
  path: string;
  sizeBytes: number;
  availabilityStatus: "available" | "corrupted" | "unreadable" | "missing";
  metadataCompleteness: number;
}

export interface DuplicateGroup {
  sha256: string;
  sizeBytes: number;
  recommendedFileId: string;
  recommendationReason: string;
  files: DuplicateFile[];
}

export interface DuplicateScanResult {
  hashedFiles: number;
  cachedFiles: number;
  failedFiles: number;
  candidateFiles: number;
  groups: DuplicateGroup[];
}

export interface MetadataValues {
  title: string | null;
  artist: string | null;
  album: string | null;
  albumArtist: string | null;
  trackNumber: string | null;
  discNumber: string | null;
  genre: string | null;
  year: string | null;
  composer: string | null;
  comment: string | null;
  bpm: number | null;
  musicalKey: string | null;
  camelot: string | null;
}

export interface MetadataHistoryEntry {
  id: string;
  createdAt: string;
  before: MetadataValues;
  after: MetadataValues;
}

export interface TrackRatingHistoryEntry {
  id: string;
  createdAt: string;
  beforeRating: number;
  afterRating: number;
}

export interface TrackMetadataDetail {
  trackId: string;
  fileId: string;
  path: string;
  format: "MP3" | "FLAC";
  values: MetadataValues;
  history: MetadataHistoryEntry[];
  ratingHistory: TrackRatingHistoryEntry[];
}

export interface BulkRatingReportItem {
  trackId: string;
  title: string;
  status: "saved" | "unchanged" | "failed";
  reason: string;
}

export interface BulkRatingReport {
  targetRating: number;
  totalTracks: number;
  savedTracks: number;
  unchangedTracks: number;
  failedTracks: number;
  items: BulkRatingReportItem[];
}

export interface MetadataChange {
  field: string;
  label: string;
  before: string | null;
  after: string | null;
}

export interface MetadataPreview {
  trackId: string;
  fileId: string;
  expectedFileState: MetadataFileState;
  before: MetadataValues;
  after: MetadataValues;
  changes: MetadataChange[];
}

export type EditableMetadataField = Exclude<keyof MetadataValues, "camelot">;

export interface BulkMetadataRequest {
  trackIds: string[];
  fields: EditableMetadataField[];
  values: MetadataValues;
}

export interface BulkMetadataPreviewItem {
  trackId: string;
  title: string;
  path: string;
  status: "ready" | "skipped" | "failed";
  expectedFileState: MetadataFileState | null;
  after: MetadataValues | null;
  changes: MetadataChange[];
  reason: string;
}

export interface BulkMetadataPreview {
  totalFiles: number;
  readyFiles: number;
  skippedFiles: number;
  failedFiles: number;
  items: BulkMetadataPreviewItem[];
}

export interface BulkMetadataCommitItem {
  trackId: string;
  expectedFileState: MetadataFileState;
  values: MetadataValues;
}

export interface BulkMetadataReportItem {
  trackId: string;
  title: string;
  path: string;
  status: "saved" | "failed";
  reason: string;
}

export interface BulkMetadataReport {
  savedFiles: number;
  failedFiles: number;
  items: BulkMetadataReportItem[];
}

export interface RelinkPreview {
  trackId: string;
  trackFileId: string;
  title: string;
  artist: string;
  oldPath: string;
  newPath: string;
  newLibraryRoot: string;
  willRegisterLibraryRoot: boolean;
  format: "MP3" | "FLAC";
  sizeBytes: number;
  modifiedAtMs: number;
  fileIdentity: string;
  availabilityStatus: "available" | "corrupted";
  warning: string | null;
  expectedState: RelinkState;
}

export interface RelinkReport {
  trackId: string;
  trackFileId: string;
  oldPath: string;
  newPath: string;
  newLibraryRoot: string;
  registeredLibraryRoot: boolean;
  availabilityStatus: "available" | "corrupted";
}

export interface DiagnosticsPreview {
  expectedState: DiagnosticsState;
  logFiles: number;
  crashReports: number;
  totalBytes: number;
  redactPaths: boolean;
  includedFields: string[];
  warnings: string[];
}

export interface DiagnosticsExportReport {
  path: string;
  sizeBytes: number;
  sha256: string;
  logFiles: number;
  crashReports: number;
  pathsRedacted: boolean;
}
