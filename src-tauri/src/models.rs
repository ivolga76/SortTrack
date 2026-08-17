use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySummary {
    pub tracks: u64,
    pub incoming: u64,
    pub needs_review: u64,
    pub duplicates: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRootSettings {
    pub id: String,
    pub path: String,
    pub recursive: bool,
    pub exclusions: Vec<String>,
    pub status: String,
    pub last_scanned_at: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRootRemovalPreview {
    pub root_id: String,
    pub path: String,
    pub linked_files: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRootRemovalReport {
    pub root_id: String,
    pub path: String,
    pub detached_files: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistRemovalPreview {
    pub playlist_id: String,
    pub name: String,
    pub item_count: u64,
    pub expected_state: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistRemovalReport {
    pub playlist_id: String,
    pub name: String,
    pub removed_items: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryColumnLayout {
    pub id: String,
    pub visible: bool,
    pub width: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryTableLayout {
    pub columns: Vec<LibraryColumnLayout>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupCounts {
    pub tracks: u64,
    pub track_files: u64,
    pub playlists: u64,
    pub metadata_history: u64,
    pub genre_history: u64,
    pub dataset_exports: u64,
    pub model_runs: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupReport {
    pub backup_path: String,
    pub database_path: String,
    pub manifest_path: String,
    pub database_sha256: String,
    pub manifest_sha256: String,
    pub database_size_bytes: u64,
    pub database_schema_version: u32,
    pub counts: BackupCounts,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestorePreview {
    pub backup_path: String,
    pub created_at: String,
    pub application_version: String,
    pub database_sha256: String,
    pub manifest_sha256: String,
    pub database_size_bytes: u64,
    pub database_schema_version: u32,
    pub current_schema_version: u32,
    pub requires_migration: bool,
    pub counts: BackupCounts,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestorePrepared {
    pub backup_path: String,
    pub safety_backup_path: String,
    pub expected_database_sha256: String,
    pub restart_required: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub root_path: String,
    pub discovered: u64,
    pub added: u64,
    pub updated: u64,
    pub moved: u64,
    pub restored: u64,
    pub missing: u64,
    pub skipped: u64,
    pub errors: Vec<ScanError>,
    pub cancelled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanOptions {
    pub recursive: bool,
    pub exclusions: Vec<String>,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            recursive: true,
            exclusions: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanPreview {
    pub root_path: String,
    pub discovered: u64,
    pub recursive: bool,
    pub exclusions: Vec<String>,
    pub warnings: Vec<ScanError>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanError {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelinkPreview {
    pub track_id: String,
    pub track_file_id: String,
    pub title: String,
    pub artist: String,
    pub old_path: String,
    pub new_path: String,
    pub new_library_root: String,
    pub will_register_library_root: bool,
    pub format: String,
    pub size_bytes: u64,
    pub modified_at_ms: i64,
    pub file_identity: String,
    pub availability_status: String,
    pub warning: Option<String>,
    pub expected_state: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelinkReport {
    pub track_id: String,
    pub track_file_id: String,
    pub old_path: String,
    pub new_path: String,
    pub new_library_root: String,
    pub registered_library_root: bool,
    pub availability_status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackRow {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: Option<String>,
    pub subgenre: Option<String>,
    pub bpm: Option<f64>,
    pub musical_key: Option<String>,
    pub camelot: Option<String>,
    pub duration_seconds: u64,
    pub format: String,
    pub bitrate_kbps: Option<u64>,
    pub path: String,
    pub rating: u8,
    pub added_at: String,
    pub analysis_state: String,
    pub cover_color: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackPage {
    pub items: Vec<TrackRow>,
    pub total: u64,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WaveformData {
    pub peaks: Vec<f32>,
    pub cache_hit: bool,
    pub algorithm_version: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverData {
    pub cover_path: Option<String>,
    pub mime_type: Option<String>,
    pub cache_hit: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistDetail {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub smart_rule: Option<SmartPlaylistRule>,
    pub items: Vec<PlaylistItemRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartPlaylistRule {
    pub genre: Option<String>,
    pub subgenre: Option<String>,
    pub min_bpm: Option<f64>,
    pub max_bpm: Option<f64>,
    pub min_rating: Option<u8>,
    pub camelot: Option<String>,
    pub limit_count: u32,
    pub sort_by: String,
    pub descending: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistSnapshotReport {
    pub playlist_id: String,
    pub name: String,
    pub item_count: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistItemRow {
    pub id: String,
    pub position: u32,
    pub track: TrackRow,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyPlan {
    pub operation_id: String,
    pub destination_root: String,
    pub total_files: u64,
    pub total_bytes: u64,
    pub required_bytes: u64,
    pub available_bytes: u64,
    pub items: Vec<CopyPlanItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyPlanItem {
    pub id: String,
    pub track_title: String,
    pub source_path: String,
    pub destination_path: String,
    pub size_bytes: u64,
    pub status: String,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyResolution {
    pub item_id: String,
    pub action: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyReport {
    pub operation_id: String,
    pub copied: u64,
    pub used_existing: u64,
    pub skipped: u64,
    pub failed: u64,
    pub bytes_copied: u64,
    pub items: Vec<CopyReportItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyReportItem {
    pub track_title: String,
    pub destination_path: String,
    pub result: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyRecoveryOperation {
    pub operation_id: String,
    pub destination_root: String,
    pub state: String,
    pub created_at: String,
    pub total_items: u64,
    pub completed_items: u64,
    pub pending_items: u64,
    pub items: Vec<CopyRecoveryItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyRecoveryItem {
    pub id: String,
    pub track_title: String,
    pub source_path: String,
    pub destination_path: String,
    pub temporary_path: Option<String>,
    pub state: String,
    pub reason: String,
    pub recommended_action: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyRecoveryReport {
    pub operation_id: String,
    pub recovered: u64,
    pub cancelled: u64,
    pub failed: u64,
    pub items: Vec<CopyRecoveryReportItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyRecoveryReportItem {
    pub track_title: String,
    pub destination_path: String,
    pub result: String,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistExportRequest {
    pub playlist_id: String,
    pub destination_path: String,
    pub format: String,
    pub path_mode: String,
    pub target_profile: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistExportPreview {
    pub playlist_id: String,
    pub playlist_name: String,
    pub destination_path: String,
    pub format: String,
    pub path_mode: String,
    pub target_profile: Option<String>,
    pub expected_state: String,
    pub total_tracks: u64,
    pub missing_tracks: u64,
    pub destination_exists: bool,
    pub items: Vec<PlaylistExportPreviewItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistExportPreviewItem {
    pub position: u64,
    pub title: String,
    pub artist: String,
    pub source_path: String,
    pub output_location: String,
    pub status: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistExportReport {
    pub output_path: String,
    pub format: String,
    pub total_tracks: u64,
    pub missing_tracks: u64,
    pub bytes_written: u64,
    pub instructions: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisJob {
    pub id: String,
    pub state: String,
    pub total_items: u64,
    pub processed_items: u64,
    pub failed_items: u64,
    pub processed_bytes: u64,
    pub total_bytes: u64,
    pub created_at: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenreCandidate {
    pub rank: u8,
    pub genre: String,
    pub subgenre: Option<String>,
    pub score: f64,
    pub signals: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenreSuggestion {
    pub track_id: String,
    pub title: String,
    pub artist: String,
    pub confirmed_genre: Option<String>,
    pub confirmed_subgenre: Option<String>,
    pub locked: bool,
    pub candidates: Vec<GenreCandidate>,
    pub inference_version: String,
    pub taxonomy_version: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetClassCount {
    pub genre: String,
    pub tracks: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenreDatasetAudit {
    pub confirmed_tracks: u64,
    pub locked_tracks: u64,
    pub eligible_tracks: u64,
    pub unavailable_tracks: u64,
    pub target_tracks: u64,
    pub classes: Vec<DatasetClassCount>,
    pub missing_genres: Vec<String>,
    pub ready: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenreDatasetExportReport {
    pub output_path: String,
    pub manifest_sha256: String,
    pub total_entries: u64,
    pub train_entries: u64,
    pub calibration_entries: u64,
    pub test_entries: u64,
    pub hashed_files: u64,
    pub cached_hashes: u64,
    pub collapsed_recordings: u64,
    pub conflicting_recordings: u64,
    pub class_counts: Vec<DatasetClassCount>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenreClassMetric {
    pub genre: String,
    pub support: u64,
    pub correct: u64,
    pub recall: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfusionCell {
    pub actual: String,
    pub predicted: String,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenreModelEvaluation {
    pub run_id: String,
    pub model_version: String,
    pub feature_version: String,
    pub dataset_manifest_sha256: String,
    pub top1_accuracy: f64,
    pub meets_release_threshold: bool,
    pub train_entries: u64,
    pub calibration_entries: u64,
    pub test_entries: u64,
    pub failed_features: u64,
    pub class_metrics: Vec<GenreClassMetric>,
    pub confusion: Vec<ConfusionCell>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioAnalysisResult {
    pub bpm: Option<f64>,
    pub bpm_score: f64,
    pub musical_key: Option<String>,
    pub key_score: f64,
    pub camelot: Option<String>,
    pub algorithm_version: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateScanResult {
    pub hashed_files: u64,
    pub cached_files: u64,
    pub failed_files: u64,
    pub candidate_files: u64,
    pub groups: Vec<DuplicateGroup>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroup {
    pub sha256: String,
    pub size_bytes: u64,
    pub recommended_file_id: String,
    pub recommendation_reason: String,
    pub files: Vec<DuplicateFile>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateFile {
    pub file_id: String,
    pub track_id: String,
    pub title: String,
    pub artist: String,
    pub format: String,
    pub path: String,
    pub size_bytes: u64,
    pub availability_status: String,
    pub metadata_completeness: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataValues {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub track_number: Option<String>,
    pub disc_number: Option<String>,
    pub genre: Option<String>,
    pub year: Option<String>,
    pub composer: Option<String>,
    pub comment: Option<String>,
    pub bpm: Option<f64>,
    pub musical_key: Option<String>,
    pub camelot: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackMetadataDetail {
    pub track_id: String,
    pub file_id: String,
    pub path: String,
    pub format: String,
    pub values: MetadataValues,
    pub history: Vec<MetadataHistoryEntry>,
    pub rating_history: Vec<TrackRatingHistoryEntry>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataHistoryEntry {
    pub id: String,
    pub created_at: String,
    pub before: MetadataValues,
    pub after: MetadataValues,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackRatingHistoryEntry {
    pub id: String,
    pub created_at: String,
    pub before_rating: u8,
    pub after_rating: u8,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkRatingReport {
    pub target_rating: u8,
    pub total_tracks: u64,
    pub saved_tracks: u64,
    pub unchanged_tracks: u64,
    pub failed_tracks: u64,
    pub items: Vec<BulkRatingReportItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkRatingReportItem {
    pub track_id: String,
    pub title: String,
    pub status: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataPreview {
    pub track_id: String,
    pub file_id: String,
    pub expected_file_state: String,
    pub before: MetadataValues,
    pub after: MetadataValues,
    pub changes: Vec<MetadataChange>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataChange {
    pub field: String,
    pub label: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkMetadataRequest {
    pub track_ids: Vec<String>,
    pub fields: Vec<String>,
    pub values: MetadataValues,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkMetadataPreview {
    pub total_files: u64,
    pub ready_files: u64,
    pub skipped_files: u64,
    pub failed_files: u64,
    pub items: Vec<BulkMetadataPreviewItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkMetadataPreviewItem {
    pub track_id: String,
    pub title: String,
    pub path: String,
    pub status: String,
    pub expected_file_state: Option<String>,
    pub after: Option<MetadataValues>,
    pub changes: Vec<MetadataChange>,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkMetadataCommitItem {
    pub track_id: String,
    pub expected_file_state: String,
    pub values: MetadataValues,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkMetadataReport {
    pub saved_files: u64,
    pub failed_files: u64,
    pub items: Vec<BulkMetadataReportItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkMetadataReportItem {
    pub track_id: String,
    pub title: String,
    pub path: String,
    pub status: String,
    pub reason: String,
}

#[cfg(test)]
mod ipc_contract_fixture_tests {
    use super::{BulkMetadataRequest, MetadataValues, PlaylistExportRequest, ScanOptions};
    use serde_json::Value;

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../docs/refactoring/ipc-contract-fixtures-v1.json"
        ))
        .expect("IPC fixture must be valid JSON")
    }

    #[test]
    fn shared_request_fixtures_deserialize_with_camel_case_contracts() {
        let fixture = fixture();
        assert_eq!(fixture["schema"], "sorttrack-ipc-dto-fixtures-1");

        let scan: ScanOptions = serde_json::from_value(fixture["scanOptions"].clone())
            .expect("scanOptions must match Rust DTO");
        assert!(scan.recursive);
        assert_eq!(scan.exclusions, vec!["archive", "temp/sets"]);

        let export: PlaylistExportRequest =
            serde_json::from_value(fixture["playlistExportRequest"].clone())
                .expect("playlistExportRequest must match Rust DTO");
        assert_eq!(export.playlist_id, "playlist-fixture");
        assert_eq!(export.target_profile.as_deref(), Some("traktor_pro_4"));

        let metadata: MetadataValues = serde_json::from_value(fixture["metadataValues"].clone())
            .expect("metadataValues must match Rust DTO");
        assert_eq!(metadata.musical_key.as_deref(), Some("F minor"));
        assert_eq!(metadata.camelot.as_deref(), Some("4A"));

        let bulk: BulkMetadataRequest =
            serde_json::from_value(fixture["bulkMetadataRequest"].clone())
                .expect("bulkMetadataRequest must match Rust DTO");
        assert_eq!(bulk.track_ids, vec!["track-a", "track-b"]);
        assert_eq!(bulk.fields, vec!["genre", "bpm"]);
    }
}
