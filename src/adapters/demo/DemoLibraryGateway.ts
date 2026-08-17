import type { AnalysisJob, BackupReport, BulkMetadataCommitItem, BulkMetadataPreview, BulkMetadataReport, BulkMetadataRequest, BulkRatingReport, CopyPlan, CopyRecoveryOperation, CopyRecoveryReport, CopyReport, CopyResolution, CoverData, DiagnosticsExportReport, DiagnosticsPreview, DuplicateGroup, DuplicateScanResult, GenreDatasetAudit, GenreDatasetExportReport, GenreModelEvaluation, GenreSuggestion, LibraryRootSettings, LibrarySummary, LibraryTableLayout, MetadataPreview, MetadataValues, Playlist, PlaylistExportFormat, PlaylistExportPathMode, PlaylistExportPreview, PlaylistExportReport, PlaylistExportRequest, PlaylistPlacement, RelinkPreview, RelinkReport, RestorePrepared, RestorePreview, RuntimeProfile, ScanOptions, ScanPreview, ScanProgress, ScanResult, Track, TrackMetadataDetail, TrackPage, TrackQuery, TrackRatingHistoryEntry, TraktorTargetProfile, WaveformData } from "../../domain/models";
import type { LibraryRootRemovalPreview, LibraryRootRemovalReport } from "../../domain/models";
import type { PlaylistRemovalPreview, PlaylistRemovalReport } from "../../domain/models";
import type { PlaylistSnapshotReport, SmartPlaylistRule } from "../../domain/models";
import type { AppGateway } from "../../shared/api/ports";
import { expectedStateToken } from "../../domain/stateTokens";
import { gatewayError } from "../../shared/api/errors";

const demoTracks: Track[] = [
  {
    id: "trk-001",
    title: "Cold Circuit",
    artist: "Nera Vector",
    album: "Signal Path",
    genre: "Techno",
    subgenre: "Raw Techno",
    bpm: 142,
    musicalKey: "F minor",
    camelot: "4A",
    durationSeconds: 371,
    format: "FLAC",
    bitrateKbps: null,
    path: "D:\\Music\\Incoming\\Nera Vector - Cold Circuit.flac",
    rating: 4,
    addedAt: "2026-08-08T12:00:00Z",
    analysisState: "ready",
    coverColor: "#d5ff45",
  },
  {
    id: "trk-002",
    title: "Gravel Pulse",
    artist: "Mono District",
    album: "Unsorted Transmission",
    genre: "Techno",
    subgenre: "Hard Techno",
    bpm: 154,
    musicalKey: "D minor",
    camelot: "7A",
    durationSeconds: 298,
    format: "MP3",
    bitrateKbps: 320,
    path: "D:\\Music\\Incoming\\Mono District - Gravel Pulse.mp3",
    rating: 3,
    addedAt: "2026-08-08T12:05:00Z",
    analysisState: "needs-review",
    coverColor: "#ff6f61",
  },
  {
    id: "trk-003",
    title: "Phase Bloom",
    artist: "Iria",
    album: "Night Geometry",
    genre: "Techno",
    subgenre: "Melodic Techno",
    bpm: 126,
    musicalKey: "A minor",
    camelot: "8A",
    durationSeconds: 412,
    format: "FLAC",
    bitrateKbps: null,
    path: "D:\\Music\\Incoming\\Iria - Phase Bloom.flac",
    rating: 5,
    addedAt: "2026-08-08T12:10:00Z",
    analysisState: "ready",
    coverColor: "#80d8ff",
  },
  {
    id: "trk-004",
    title: "No Return",
    artist: "Kinetic Vandal",
    album: "Drop Folder 08",
    genre: "Drum & Bass",
    subgenre: null,
    bpm: 174,
    musicalKey: "G minor",
    camelot: "6A",
    durationSeconds: 267,
    format: "MP3",
    bitrateKbps: 320,
    path: "D:\\Music\\Incoming\\Kinetic Vandal - No Return.mp3",
    rating: 0,
    addedAt: "2026-08-08T12:15:00Z",
    analysisState: "needs-review",
    coverColor: "#be8cff",
  },
  {
    id: "trk-005",
    title: "Broken Relay",
    artist: "Static Youth",
    album: "No Album",
    genre: null,
    subgenre: null,
    bpm: null,
    musicalKey: null,
    camelot: null,
    durationSeconds: 226,
    format: "FLAC",
    bitrateKbps: null,
    path: "D:\\Music\\Incoming\\Static Youth - Broken Relay.flac",
    rating: 0,
    addedAt: "2026-08-08T12:20:00Z",
    analysisState: "queued",
    coverColor: "#ffd166",
  },
];

export class DemoLibraryGateway implements AppGateway {
  readonly isDesktop = false;
  private libraryRoots: LibraryRootSettings[] = [{ id: "root-demo", path: "D:\\Music\\Incoming", recursive: true, exclusions: ["archive", "temp"], status: "active", lastScannedAt: "2026-08-09T10:00:00Z" }];
  private playlists: Playlist[] = [
    { id: "playlist-demo-1", name: "Friday Hard Set", kind: "manual", smartRule: null, items: demoTracks.slice(0, 2).map((track, position) => ({ id: `demo-a-${track.id}`, position, track })) },
    { id: "playlist-demo-2", name: "Melodic Warm-up", kind: "manual", smartRule: null, items: demoTracks.slice(2, 3).map((track, position) => ({ id: `demo-b-${track.id}`, position, track })) },
  ];
  private metadataHistory: TrackMetadataDetail["history"] = [];
  private ratingHistory: Record<string, TrackRatingHistoryEntry[]> = {};
  private analysisJobs: AnalysisJob[] = [];
  private genreLocks = new Set<string>();

  async getSummary(): Promise<LibrarySummary> {
    return { tracks: 82431, incoming: 148, needsReview: 36, duplicates: 12 };
  }

  async listLibraryRoots(): Promise<LibraryRootSettings[]> {
    return this.libraryRoots.map((root) => ({ ...root, exclusions: [...root.exclusions] }));
  }
  async refreshLibraryRootAvailability(): Promise<LibraryRootSettings[]> {
    return this.listLibraryRoots();
  }
  async previewLibraryRootRemoval(rootId: string): Promise<LibraryRootRemovalPreview> {
    const root = this.libraryRoots.find((item) => item.id === rootId);
    if (!root || root.status !== "disabled") throw gatewayError("Сначала отключите папку", "remove_library_root_registration");
    return { rootId, path: root.path, linkedFiles: 82431 };
  }
  async removeLibraryRootRegistration(preview: LibraryRootRemovalPreview): Promise<LibraryRootRemovalReport> {
    this.libraryRoots = this.libraryRoots.filter((root) => root.id !== preview.rootId);
    return { rootId: preview.rootId, path: preview.path, detachedFiles: preview.linkedFiles };
  }

  async setLibraryRootEnabled(rootId: string, expectedStatus: LibraryRootSettings["status"], enabled: boolean): Promise<void> {
    const root = this.libraryRoots.find((item) => item.id === rootId);
    if (!root || root.status !== expectedStatus) throw gatewayError("Статус папки изменился", "set_library_root_enabled");
    root.status = enabled ? "active" : "disabled";
  }

  async getLibraryTableLayout(): Promise<LibraryTableLayout> {
    return { columns: [
      { id: "track", visible: true, width: 320 },
      { id: "genre", visible: true, width: 170 },
      { id: "bpm", visible: true, width: 72 },
      { id: "key", visible: true, width: 130 },
      { id: "format", visible: true, width: 82 },
      { id: "duration", visible: true, width: 76 },
    ] };
  }

  async saveLibraryTableLayout(layout: LibraryTableLayout): Promise<LibraryTableLayout> { return layout; }

  async getRuntimeProfile(): Promise<RuntimeProfile> {
    return { mode: "browser", dataRoot: "Демонстрационный режим — данные не сохраняются" };
  }

  async previewDiagnostics(redactPaths: boolean): Promise<DiagnosticsPreview> {
    return {
      expectedState: expectedStateToken<"diagnostics">("demo-diagnostics-state"),
      logFiles: 2,
      crashReports: 0,
      totalBytes: 18_432,
      redactPaths,
      includedFields: ["SortTrack version", "local application logs"],
      warnings: ["Music files and the library database are excluded", "Nothing is sent automatically"],
    };
  }

  async pickAndExportDiagnostics(_preview: DiagnosticsPreview, redactPaths: boolean): Promise<DiagnosticsExportReport> {
    return { path: "D:\\SortTrack-diagnostics-demo.json", sizeBytes: 18_432, sha256: "demo-diagnostics-sha256", logFiles: 2, crashReports: 0, pathsRedacted: redactPaths };
  }

  async openLogDirectory(): Promise<void> {}

  async clearDiagnostics(): Promise<number> { return 2; }

  async pickAndCreateBackup(): Promise<BackupReport> {
    return {
      backupPath: "D:\\SortTrack backups\\SortTrack-backup-demo",
      databasePath: "D:\\SortTrack backups\\SortTrack-backup-demo\\sorttrack.sqlite3",
      manifestPath: "D:\\SortTrack backups\\SortTrack-backup-demo\\manifest.json",
      databaseSha256: "demo-database-sha256",
      manifestSha256: "demo-manifest-sha256",
      databaseSizeBytes: 12_582_912,
      databaseSchemaVersion: 13,
      counts: { tracks: 82431, trackFiles: 82904, playlists: 2, metadataHistory: 38, genreHistory: 742, datasetExports: 1, modelRuns: 1 },
    };
  }

  async pickAndPreviewRestore(): Promise<RestorePreview> {
    const backup = await this.pickAndCreateBackup();
    return {
      backupPath: backup.backupPath,
      createdAt: "2026-08-09T10:00:00+03:00",
      applicationVersion: "0.1.0",
      databaseSha256: backup.databaseSha256,
      manifestSha256: backup.manifestSha256,
      databaseSizeBytes: backup.databaseSizeBytes,
      databaseSchemaVersion: 13,
      currentSchemaVersion: 10,
      requiresMigration: false,
      counts: backup.counts,
    };
  }

  async prepareRestoreAndRestart(preview: RestorePreview): Promise<RestorePrepared> {
    return { backupPath: preview.backupPath, safetyBackupPath: "D:\\SortTrack Data\\database\\SortTrack-backup-safety", expectedDatabaseSha256: preview.databaseSha256, restartRequired: true };
  }

  async searchTracks(query: TrackQuery): Promise<TrackPage> {
    const needle = query.text.trim().toLocaleLowerCase("ru-RU");
    const filtered = demoTracks.filter((track) => {
      const matchesText =
        !needle ||
        [track.title, track.artist, track.album, track.genre, track.subgenre, track.path]
          .filter(Boolean)
          .some((value) => value!.toLocaleLowerCase("ru-RU").includes(needle));
      const matchesGenre = !query.genre || track.genre === query.genre;
      const matchesIncoming = !query.onlyIncoming || !track.genre;
      const matchesBpm = (query.minBpm === null || (track.bpm ?? -Infinity) >= query.minBpm) && (query.maxBpm === null || (track.bpm ?? Infinity) <= query.maxBpm);
      const matchesRating = query.minRating === null || track.rating >= query.minRating;
      const matchesCamelot = !query.camelot || track.camelot === query.camelot;
      const matchesFormat = !query.format || track.format.toLowerCase() === query.format;
      const matchesStatus = query.fileStatus === "all" || (query.fileStatus === "available" && track.analysisState !== "missing") || (query.fileStatus === "missing" && track.analysisState === "missing") || (query.fileStatus === "problems" && track.analysisState === "needs-review");
      return matchesText && matchesGenre && matchesIncoming && matchesBpm && matchesRating && matchesCamelot && matchesFormat && matchesStatus;
    });
    const direction = query.descending ? -1 : 1;
    filtered.sort((left, right) => {
      if (query.sortBy === "title") return direction * left.title.localeCompare(right.title);
      if (query.sortBy === "artist") return direction * left.artist.localeCompare(right.artist);
      if (query.sortBy === "bpm") return direction * ((left.bpm ?? -1) - (right.bpm ?? -1));
      return direction * left.addedAt.localeCompare(right.addedAt);
    });
    return { items: filtered.slice(query.offset, query.offset + query.limit), total: filtered.length, limit: query.limit, offset: query.offset };
  }

  async locateTrackPage(trackId: string, pageSize: number): Promise<number | null> {
    const index = demoTracks.findIndex((track) => track.id === trackId);
    return index < 0 ? null : Math.floor(index / pageSize) * pageSize;
  }

  async setTrackRating(trackId: string, expectedRating: number, nextRating: number): Promise<void> {
    const track = demoTracks.find((item) => item.id === trackId);
    if (!track || track.rating !== expectedRating) throw gatewayError("Рейтинг изменился; обновите библиотеку", "set_track_rating");
    if (!Number.isInteger(nextRating) || nextRating < 0 || nextRating > 5) throw gatewayError("Рейтинг должен быть от 0 до 5", "set_track_rating");
    track.rating = nextRating;
    this.ratingHistory[trackId] ??= [];
    this.ratingHistory[trackId].unshift({ id: `rating-${Date.now()}`, createdAt: new Date().toISOString(), beforeRating: expectedRating, afterRating: nextRating });
    for (const playlist of this.playlists) {
      for (const item of playlist.items) if (item.track.id === trackId) item.track.rating = nextRating;
    }
  }

  async bulkSetTrackRating(trackIds: string[], targetRating: number): Promise<BulkRatingReport> {
    const unique = [...new Set(trackIds)];
    const items = [];
    for (const trackId of unique) {
      const track = demoTracks.find((item) => item.id === trackId);
      if (!track) { items.push({ trackId, title: trackId, status: "failed" as const, reason: "Трек не найден" }); continue; }
      if (track.rating === targetRating) { items.push({ trackId, title: track.title, status: "unchanged" as const, reason: "Рейтинг уже установлен" }); continue; }
      await this.setTrackRating(trackId, track.rating, targetRating);
      items.push({ trackId, title: track.title, status: "saved" as const, reason: "Рейтинг сохранён" });
    }
    return { targetRating, totalTracks: items.length, savedTracks: items.filter((item) => item.status === "saved").length, unchangedTracks: items.filter((item) => item.status === "unchanged").length, failedTracks: items.filter((item) => item.status === "failed").length, items };
  }

  async pickAndPreviewScanFolder(): Promise<ScanPreview> {
    return { rootPath: "D:\\Music\\Incoming", discovered: demoTracks.length, recursive: true, exclusions: [], warnings: [] };
  }
  async previewScanFolder(rootPath: string, options: ScanOptions): Promise<ScanPreview> {
    return { rootPath, discovered: options.recursive ? demoTracks.length : 2, ...options, warnings: [] };
  }
  async scanFolder(preview: ScanPreview): Promise<ScanResult> {
    return { rootPath: preview.rootPath, discovered: preview.discovered, added: preview.discovered, updated: 0, moved: 0, restored: 0, missing: 0, skipped: 0, errors: [], cancelled: false };
  }
  async getScanProgress(): Promise<ScanProgress | null> { return null; }
  async pauseScan(): Promise<void> {}
  async resumeScan(): Promise<void> {}
  async cancelScan(): Promise<void> {}

  async pickAndPreviewRelink(): Promise<null> { return null; }
  async commitRelink(): Promise<RelinkReport> { throw gatewayError("Перепривязка доступна в desktop-приложении", "commit_track_relink"); }

  async preparePlayback(): Promise<string | null> {
    return null;
  }

  async getTrackWaveform(): Promise<WaveformData> {
    return { peaks: Array.from({ length: 96 }, (_, index) => 0.18 + ((index * 17) % 23) / 34), cacheHit: true, algorithmVersion: "demo" };
  }

  async getTrackCover(): Promise<null> { return null; }

  async listPlaylists(): Promise<Playlist[]> {
    return structuredClone(this.playlists);
  }

  async createPlaylist(name: string): Promise<string> {
    const id = `playlist-${Date.now()}`;
    this.playlists.push({ id, name, kind: "manual", smartRule: null, items: [] });
    return id;
  }

  async createSmartPlaylist(name: string, rule: SmartPlaylistRule): Promise<string> {
    const id = `smart-playlist-${Date.now()}`;
    const matches = demoTracks.filter((track) => (!rule.genre || track.genre === rule.genre) && (!rule.subgenre || track.subgenre === rule.subgenre) && (rule.minBpm === null || (track.bpm ?? -Infinity) >= rule.minBpm) && (rule.maxBpm === null || (track.bpm ?? Infinity) <= rule.maxBpm) && (rule.minRating === null || track.rating >= rule.minRating) && (!rule.camelot || track.camelot === rule.camelot)).slice(0, rule.limitCount);
    this.playlists.push({ id, name, kind: "smart", smartRule: rule, items: matches.map((track, position) => ({ id: `smart-${id}-${track.id}`, position, track })) });
    return id;
  }

  async snapshotSmartPlaylist(smartPlaylistId: string, name: string): Promise<PlaylistSnapshotReport> {
    const source = this.playlists.find((item) => item.id === smartPlaylistId && item.kind === "smart");
    if (!source) throw gatewayError("Автоплейлист не найден", "snapshot_smart_playlist");
    const playlistId = `playlist-snapshot-${Date.now()}`;
    const items = source.items.map((item, position) => ({
      id: `snapshot-${playlistId}-${item.track.id}`,
      position,
      track: structuredClone(item.track),
    }));
    this.playlists.push({ id: playlistId, name, kind: "manual", smartRule: null, items });
    return { playlistId, name, itemCount: items.length };
  }

  async updateSmartPlaylistRule(smartPlaylistId: string, expectedRule: SmartPlaylistRule, nextRule: SmartPlaylistRule): Promise<void> {
    const playlist = this.playlists.find((item) => item.id === smartPlaylistId && item.kind === "smart");
    if (!playlist?.smartRule) throw gatewayError("Автоплейлист не найден", "update_smart_playlist_rule");
    if (JSON.stringify(playlist.smartRule) !== JSON.stringify(expectedRule)) throw gatewayError("Правило изменилось; откройте редактор снова", "update_smart_playlist_rule");
    playlist.smartRule = structuredClone(nextRule);
    const direction = nextRule.descending ? -1 : 1;
    const matches = demoTracks
      .filter((track) => (!nextRule.genre || track.genre === nextRule.genre) && (!nextRule.subgenre || track.subgenre === nextRule.subgenre) && (nextRule.minBpm === null || (track.bpm ?? -Infinity) >= nextRule.minBpm) && (nextRule.maxBpm === null || (track.bpm ?? Infinity) <= nextRule.maxBpm) && (nextRule.minRating === null || track.rating >= nextRule.minRating) && (!nextRule.camelot || track.camelot === nextRule.camelot))
      .sort((left, right) => direction * (nextRule.sortBy === "title" ? left.title.localeCompare(right.title) : nextRule.sortBy === "bpm" ? (left.bpm ?? -Infinity) - (right.bpm ?? -Infinity) : left.id.localeCompare(right.id)))
      .slice(0, nextRule.limitCount);
    playlist.items = matches.map((track, position) => ({ id: `smart-${smartPlaylistId}-${track.id}`, position, track }));
  }

  async renamePlaylist(playlistId: string, name: string): Promise<void> {
    const playlist = this.playlists.find((item) => item.id === playlistId);
    if (playlist) playlist.name = name.trim();
  }

  async previewPlaylistRemoval(playlistId: string): Promise<PlaylistRemovalPreview> {
    const playlist = this.playlists.find((item) => item.id === playlistId);
    if (!playlist) throw gatewayError("Плейлист не найден", "preview_playlist_removal");
    return { playlistId, name: playlist.name, itemCount: playlist.items.length, expectedState: expectedStateToken<"playlist-removal">(`${playlist.name}:${playlist.items.map((item) => item.id).join(",")}`) };
  }
  async removePlaylist(preview: PlaylistRemovalPreview): Promise<PlaylistRemovalReport> {
    const current = await this.previewPlaylistRemoval(preview.playlistId);
    if (current.expectedState !== preview.expectedState) throw gatewayError("Плейлист изменился", "remove_playlist");
    const index = this.playlists.findIndex((item) => item.id === preview.playlistId);
    if (index < 0) throw gatewayError("Плейлист не найден", "remove_playlist");
    this.playlists.splice(index, 1);
    return { playlistId: preview.playlistId, name: current.name, removedItems: current.itemCount };
  }

  async placeTrack(placement: PlaylistPlacement): Promise<void> {
    const target = this.playlists.find((playlist) => playlist.id === placement.targetPlaylistId);
    if (!target || target.kind !== "manual") throw gatewayError("Целевой плейлист недоступен для изменения", "place_track_in_playlist");
    let track = demoTracks.find((item) => item.id === placement.trackId);
    let itemId = placement.itemId ?? `demo-item-${placement.targetPlaylistId}-${placement.trackId}-${Date.now()}`;
    if (placement.sourcePlaylistId && placement.itemId) {
      const source = this.playlists.find((playlist) => playlist.id === placement.sourcePlaylistId);
      const sourceIndex = source?.items.findIndex((item) => item.id === placement.itemId) ?? -1;
      if (!source || sourceIndex < 0) throw gatewayError("Исходный элемент плейлиста не найден", "place_track_in_playlist");
      const [moved] = source.items.splice(sourceIndex, 1);
      track = moved.track;
      itemId = moved.id;
      source.items.forEach((item, position) => { item.position = position; });
    }
    if (!track) throw gatewayError("Трек не найден", "place_track_in_playlist");
    const targetPosition = Math.max(0, Math.min(placement.targetPosition, target.items.length));
    target.items.splice(targetPosition, 0, { id: itemId, position: targetPosition, track: structuredClone(track) });
    target.items.forEach((item, position) => { item.position = position; });
  }

  async removePlaylistItem(playlistId: string, itemId: string): Promise<void> {
    const playlist = this.playlists.find((item) => item.id === playlistId && item.kind === "manual");
    if (!playlist) throw gatewayError("Плейлист не найден", "remove_playlist_item");
    playlist.items = playlist.items.filter((item) => item.id !== itemId);
    playlist.items.forEach((item, position) => { item.position = position; });
  }

  async pickAndPlanPlaylistCopy(): Promise<CopyPlan | null> { return null; }
  async executePlaylistCopy(operationId: string, _resolutions: CopyResolution[]): Promise<CopyReport> {
    return { operationId, copied: 0, usedExisting: 0, skipped: 0, failed: 0, bytesCopied: 0, items: [] };
  }
  async listCopyRecoveries(): Promise<CopyRecoveryOperation[]> { return []; }
  async recoverCopyOperation(operationId: string, _action: "resume" | "cancel_safe"): Promise<CopyRecoveryReport> {
    return { operationId, recovered: 0, cancelled: 0, failed: 0, items: [] };
  }

  async pickAndPreviewPlaylistExport(playlistId: string, format: PlaylistExportFormat, pathMode: PlaylistExportPathMode, targetProfile: TraktorTargetProfile | null): Promise<PlaylistExportPreview | null> {
    const playlist = this.playlists.find((item) => item.id === playlistId);
    if (!playlist) throw gatewayError("Плейлист не найден", "preview_playlist_export");
    const destinationPath = `D:\\Export\\${playlist.name}.${format === "traktor_nml" ? "nml" : "m3u8"}`;
    const items = playlist.items.map((item) => ({ position: item.position, title: item.track.title, artist: item.track.artist, sourcePath: item.track.path, outputLocation: item.track.path, status: "ready" as const, reason: "Готово" }));
    return { playlistId, playlistName: playlist.name, destinationPath, format, pathMode: format === "traktor_nml" ? "absolute" : pathMode, targetProfile: format === "traktor_nml" ? targetProfile : null, expectedState: expectedStateToken<"playlist-export">("demo-export-preview"), totalTracks: items.length, missingTracks: 0, destinationExists: false, items };
  }
  async commitPlaylistExport(request: PlaylistExportRequest, _expectedState: string): Promise<PlaylistExportReport> {
    return { outputPath: request.destinationPath, format: request.format, totalTracks: this.playlists.find((item) => item.id === request.playlistId)?.items.length ?? 0, missingTracks: 0, bytesWritten: 0, instructions: "Демонстрационный режим не записывает файл" };
  }

  async listAnalysisJobs(): Promise<AnalysisJob[]> { return structuredClone(this.analysisJobs); }
  async enqueueAudioAnalysis(trackIds: string[]): Promise<AnalysisJob> {
    const job: AnalysisJob = { id: `demo-analysis-${Date.now()}`, state: "queued", totalItems: trackIds.length, processedItems: 0, failedItems: 0, processedBytes: 0, totalBytes: 0, createdAt: new Date().toISOString(), error: null };
    this.analysisJobs.unshift(job);
    return structuredClone(job);
  }
  async runAnalysisJob(jobId: string): Promise<AnalysisJob> {
    const job = this.analysisJobs.find((item) => item.id === jobId);
    if (!job) throw gatewayError("Задание анализа не найдено", "run_analysis_job");
    job.state = "completed";
    job.processedItems = job.totalItems;
    return structuredClone(job);
  }
  async pauseAnalysisJob(jobId: string): Promise<void> {
    const job = this.analysisJobs.find((item) => item.id === jobId);
    if (job) job.state = "paused";
  }
  async cancelAnalysisJob(jobId: string): Promise<void> {
    const job = this.analysisJobs.find((item) => item.id === jobId);
    if (job) job.state = "cancelled";
  }

  async suggestTrackGenres(trackIds: string[]): Promise<GenreSuggestion[]> {
    return trackIds.flatMap((trackId) => {
      const track = demoTracks.find((item) => item.id === trackId);
      if (!track) return [];
      return [{ trackId, title: track.title, artist: track.artist, confirmedGenre: track.genre, confirmedSubgenre: track.subgenre, locked: this.genreLocks.has(trackId), candidates: [{ rank: 1, genre: track.genre ?? "Techno", subgenre: track.subgenre, score: 0.82, signals: ["demo metadata", "BPM"] }], inferenceVersion: "demo-genre-1", taxonomyVersion: 1 }];
    });
  }
  async confirmTrackGenre(trackId: string, genre: string, subgenre: string | null, locked: boolean): Promise<void> {
    const track = demoTracks.find((item) => item.id === trackId);
    if (!track) throw gatewayError("Трек не найден", "confirm_track_genre");
    track.genre = genre;
    track.subgenre = subgenre;
    if (locked) this.genreLocks.add(trackId); else this.genreLocks.delete(trackId);
  }
  async auditGenreDataset(): Promise<GenreDatasetAudit> {
    const classes = [...new Set(demoTracks.map((track) => track.genre).filter((genre): genre is string => Boolean(genre)))].map((genre) => ({ genre, tracks: demoTracks.filter((track) => track.genre === genre).length }));
    return { confirmedTracks: demoTracks.filter((track) => track.genre).length, lockedTracks: this.genreLocks.size, eligibleTracks: this.genreLocks.size, unavailableTracks: 0, targetTracks: 1000, classes, missingGenres: [], ready: false, warnings: ["Демонстрационный набор не экспортируется"] };
  }
  async pickAndExportGenreDataset(): Promise<GenreDatasetExportReport | null> { return null; }
  async evaluateLatestGenreModel(): Promise<GenreModelEvaluation> { throw gatewayError("Оценка модели доступна в desktop-приложении", "evaluate_genre_model"); }
  async getLatestGenreModelEvaluation(): Promise<GenreModelEvaluation | null> { return null; }
  async pickAndExportLatestModelReport(): Promise<string | null> { return null; }

  async listDuplicateGroups(): Promise<DuplicateGroup[]> { return structuredClone(demoDuplicateGroups); }
  async scanExactDuplicates(): Promise<DuplicateScanResult> { return { hashedFiles: 0, cachedFiles: 4, failedFiles: 0, candidateFiles: 4, groups: structuredClone(demoDuplicateGroups) }; }

  async getTrackMetadata(trackId: string): Promise<TrackMetadataDetail> {
    const track = demoTracks.find((item) => item.id === trackId);
    if (!track) throw gatewayError("Трек не найден", "get_track_metadata");
    return { trackId, fileId: `demo-file-${trackId}`, path: track.path, format: track.format, values: metadataFromTrack(track), history: structuredClone(this.metadataHistory), ratingHistory: structuredClone(this.ratingHistory[trackId] ?? []) };
  }
  async previewMetadataUpdate(trackId: string, values: MetadataValues): Promise<MetadataPreview> {
    const detail = await this.getTrackMetadata(trackId);
    const changes = (Object.keys(values) as (keyof MetadataValues)[]).filter((field) => detail.values[field] !== values[field]).map((field) => ({ field, label: field, before: detail.values[field] === null ? null : String(detail.values[field]), after: values[field] === null ? null : String(values[field]) }));
    if (!changes.length) throw gatewayError("Нет изменений для сохранения", "preview_metadata_update");
    return { trackId, fileId: detail.fileId, expectedFileState: expectedStateToken<"metadata-file">("demo-preview"), before: detail.values, after: values, changes };
  }
  async commitMetadataUpdate(trackId: string, _expectedFileState: string, values: MetadataValues): Promise<TrackMetadataDetail> {
    const track = demoTracks.find((item) => item.id === trackId);
    if (!track) throw gatewayError("Трек не найден", "commit_metadata_update");
    const before = metadataFromTrack(track);
    track.title = values.title ?? track.title;
    track.artist = values.artist ?? track.artist;
    track.album = values.album ?? "";
    track.genre = values.genre;
    track.bpm = values.bpm;
    track.musicalKey = values.musicalKey;
    track.camelot = values.camelot ?? camelotForKey(values.musicalKey);
    this.metadataHistory.unshift({ id: `demo-history-${Date.now()}`, createdAt: new Date().toISOString(), before, after: metadataFromTrack(track) });
    return this.getTrackMetadata(trackId);
  }

  async previewBulkMetadataUpdate(request: BulkMetadataRequest): Promise<BulkMetadataPreview> {
    const items = await Promise.all(request.trackIds.map(async (trackId) => {
      const detail = await this.getTrackMetadata(trackId);
      const values = structuredClone(detail.values);
      for (const field of request.fields) (values[field] as MetadataValues[typeof field]) = request.values[field];
      try {
        const item = await this.previewMetadataUpdate(trackId, values);
        return { trackId, title: detail.values.title || "Без названия", path: detail.path, status: "ready" as const, expectedFileState: item.expectedFileState, after: item.after, changes: item.changes, reason: "Готово к безопасной записи" };
      } catch (error) {
        return { trackId, title: detail.values.title || "Без названия", path: detail.path, status: "skipped" as const, expectedFileState: null, after: null, changes: [], reason: String(error) };
      }
    }));
    return { totalFiles: items.length, readyFiles: items.filter((item) => item.status === "ready").length, skippedFiles: items.filter((item) => item.status === "skipped").length, failedFiles: 0, items };
  }

  async commitBulkMetadataUpdate(items: BulkMetadataCommitItem[]): Promise<BulkMetadataReport> {
    const results = [];
    for (const item of items) {
      try {
        const saved = await this.commitMetadataUpdate(item.trackId, item.expectedFileState, item.values);
        results.push({ trackId: item.trackId, title: saved.values.title || "Без названия", path: saved.path, status: "saved" as const, reason: "Теги записаны и повторно проверены" });
      } catch (error) {
        results.push({ trackId: item.trackId, title: item.trackId, path: "", status: "failed" as const, reason: String(error) });
      }
    }
    return { savedFiles: results.filter((item) => item.status === "saved").length, failedFiles: results.filter((item) => item.status === "failed").length, items: results };
  }
}

function metadataFromTrack(track: Track): MetadataValues {
  return {
    title: track.title,
    artist: track.artist,
    album: track.album,
    albumArtist: null,
    trackNumber: null,
    discNumber: null,
    genre: track.genre,
    year: null,
    composer: null,
    comment: null,
    bpm: track.bpm,
    musicalKey: track.musicalKey,
    camelot: track.camelot,
  };
}

function camelotForKey(value: string | null): string | null {
  if (!value) return null;
  const normalized = value.trim().toLocaleLowerCase("en-US").replaceAll("♯", "#").replaceAll("♭", "b").replaceAll("major", "maj").replaceAll("minor", "min").replaceAll(" ", "");
  const keys: Record<string, string> = {
    abmin: "1A", "g#min": "1A", bmaj: "1B", cbmaj: "1B", ebmin: "2A", "d#min": "2A", "f#maj": "2B", gbmaj: "2B",
    bbmin: "3A", "a#min": "3A", dbmaj: "3B", "c#maj": "3B", fmin: "4A", abmaj: "4B", "g#maj": "4B", cmin: "5A",
    ebmaj: "5B", "d#maj": "5B", gmin: "6A", bbmaj: "6B", "a#maj": "6B", dmin: "7A", fmaj: "7B", amin: "8A",
    cmaj: "8B", emin: "9A", gmaj: "9B", bmin: "10A", dmaj: "10B", "f#min": "11A", gbmin: "11A", amaj: "11B",
    "c#min": "12A", dbmin: "12A", emaj: "12B",
  };
  return keys[normalized] ?? null;
}

const demoDuplicateGroups: DuplicateGroup[] = [
  {
    sha256: "de4db33f9d8a14d0c11f3b72df944ac2438e899623d48cbd58d97c84119ea772",
    sizeBytes: 54_123_812,
    recommendedFileId: "duplicate-file-a",
    recommendationReason: "Файлы полностью одинаковы; рекомендован вариант с более простым путём",
    files: [
      { fileId: "duplicate-file-a", trackId: "trk-001", title: "Cold Circuit", artist: "Nera Vector", format: "FLAC", path: "D:\\Music\\Incoming\\Nera Vector - Cold Circuit.flac", sizeBytes: 54_123_812, availabilityStatus: "available", metadataCompleteness: 88 },
      { fileId: "duplicate-file-b", trackId: "trk-duplicate", title: "Cold Circuit", artist: "Nera Vector", format: "FLAC", path: "D:\\Music\\Backup\\2025\\Nera Vector - Cold Circuit.flac", sizeBytes: 54_123_812, availabilityStatus: "available", metadataCompleteness: 88 },
    ],
  },
  {
    sha256: "41b54f0e8dc63539f428c1ccf355beec17ef72f79cd45b218a54203ca177be21",
    sizeBytes: 12_882_004,
    recommendedFileId: "duplicate-file-c",
    recommendationReason: "Рекомендована наиболее полная запись метаданных: 75%",
    files: [
      { fileId: "duplicate-file-c", trackId: "trk-002", title: "Gravel Pulse", artist: "Mono District", format: "MP3", path: "D:\\Music\\Incoming\\Mono District - Gravel Pulse.mp3", sizeBytes: 12_882_004, availabilityStatus: "available", metadataCompleteness: 75 },
      { fileId: "duplicate-file-d", trackId: "trk-duplicate-2", title: "Gravel Pulse", artist: "Mono District", format: "MP3", path: "D:\\Downloads\\Gravel Pulse.mp3", sizeBytes: 12_882_004, availabilityStatus: "available", metadataCompleteness: 38 },
    ],
  },
];
