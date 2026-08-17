PRAGMA foreign_keys = ON;

CREATE TABLE schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE library_roots (
    id TEXT PRIMARY KEY,
    canonical_path TEXT NOT NULL UNIQUE,
    volume_id TEXT,
    scan_subfolders INTEGER NOT NULL DEFAULT 1 CHECK (scan_subfolders IN (0, 1)),
    exclusions_json TEXT NOT NULL DEFAULT '[]',
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'unavailable', 'disabled')),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_scanned_at TEXT
);

CREATE TABLE tracks (
    id TEXT PRIMARY KEY,
    preferred_title TEXT,
    preferred_artist TEXT,
    confirmed_genre TEXT,
    confirmed_subgenre TEXT,
    genre_locked INTEGER NOT NULL DEFAULT 0 CHECK (genre_locked IN (0, 1)),
    rating INTEGER NOT NULL DEFAULT 0 CHECK (rating BETWEEN 0 AND 5),
    user_notes TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE track_files (
    id TEXT PRIMARY KEY,
    track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    library_root_id TEXT REFERENCES library_roots(id) ON DELETE SET NULL,
    canonical_path TEXT NOT NULL UNIQUE,
    normalized_path_key TEXT NOT NULL UNIQUE,
    filename TEXT NOT NULL,
    extension TEXT NOT NULL CHECK (lower(extension) IN ('mp3', 'flac')),
    size_bytes INTEGER NOT NULL CHECK (size_bytes >= 0),
    modified_at_ms INTEGER NOT NULL,
    file_identity TEXT,
    sha256 TEXT,
    sha256_state TEXT NOT NULL DEFAULT 'not_computed'
        CHECK (sha256_state IN ('not_computed', 'queued', 'ready', 'failed', 'stale')),
    acoustic_fingerprint TEXT,
    fingerprint_version TEXT,
    codec TEXT,
    duration_ms INTEGER CHECK (duration_ms IS NULL OR duration_ms >= 0),
    bitrate_kbps INTEGER,
    sample_rate_hz INTEGER,
    bit_depth INTEGER,
    channels INTEGER,
    availability_status TEXT NOT NULL DEFAULT 'available'
        CHECK (availability_status IN ('available', 'missing', 'unreadable', 'corrupted')),
    last_scanned_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_track_files_track_id ON track_files(track_id);
CREATE INDEX idx_track_files_root_id ON track_files(library_root_id);
CREATE INDEX idx_track_files_scan_signature ON track_files(size_bytes, modified_at_ms);
CREATE INDEX idx_track_files_sha256 ON track_files(sha256) WHERE sha256 IS NOT NULL;

CREATE TABLE track_metadata (
    track_file_id TEXT PRIMARY KEY REFERENCES track_files(id) ON DELETE CASCADE,
    title TEXT,
    artist TEXT,
    album TEXT,
    album_artist TEXT,
    track_number TEXT,
    disc_number TEXT,
    original_genre_tag TEXT,
    year TEXT,
    composer TEXT,
    comment TEXT,
    bpm REAL,
    musical_key TEXT,
    camelot TEXT,
    copyright TEXT,
    metadata_fingerprint TEXT NOT NULL,
    read_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE metadata_history (
    id TEXT PRIMARY KEY,
    track_file_id TEXT NOT NULL REFERENCES track_files(id) ON DELETE CASCADE,
    operation_id TEXT NOT NULL,
    before_json TEXT NOT NULL,
    after_json TEXT NOT NULL,
    file_state_before TEXT NOT NULL,
    file_state_after TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_metadata_history_file_time
    ON metadata_history(track_file_id, created_at DESC);

CREATE TABLE playlists (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    kind TEXT NOT NULL DEFAULT 'manual' CHECK (kind IN ('manual', 'smart')),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE playlist_items (
    id TEXT PRIMARY KEY,
    playlist_id TEXT NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
    track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK (position >= 0),
    added_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (playlist_id, position)
);

CREATE INDEX idx_playlist_items_track_id ON playlist_items(track_id);

CREATE TABLE background_jobs (
    id TEXT PRIMARY KEY,
    job_type TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN (
        'queued', 'running', 'paused', 'completed', 'failed', 'cancelled', 'recovery_required'
    )),
    priority INTEGER NOT NULL DEFAULT 0,
    payload_json TEXT NOT NULL DEFAULT '{}',
    processed_items INTEGER NOT NULL DEFAULT 0,
    total_items INTEGER,
    processed_bytes INTEGER NOT NULL DEFAULT 0,
    total_bytes INTEGER,
    error_json TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    started_at TEXT,
    finished_at TEXT,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_background_jobs_queue ON background_jobs(state, priority DESC, created_at);

CREATE TABLE file_operations (
    id TEXT PRIMARY KEY,
    operation_type TEXT NOT NULL CHECK (operation_type IN ('copy', 'copy_by_template')),
    destination_root TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN (
        'planned', 'running', 'completed', 'completed_with_errors', 'cancelled', 'recovery_required'
    )),
    policy_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    confirmed_at TEXT,
    finished_at TEXT
);

CREATE TABLE file_operation_items (
    id TEXT PRIMARY KEY,
    operation_id TEXT NOT NULL REFERENCES file_operations(id) ON DELETE CASCADE,
    track_file_id TEXT NOT NULL REFERENCES track_files(id) ON DELETE RESTRICT,
    source_path TEXT NOT NULL,
    destination_path TEXT NOT NULL,
    temporary_path TEXT,
    state TEXT NOT NULL CHECK (state IN (
        'planned', 'copying_to_temp', 'temp_copied', 'verified', 'destination_committed',
        'completed', 'failed_retryable', 'failed_final', 'cancelled_safe', 'recovery_required'
    )),
    reason TEXT,
    bytes_copied INTEGER NOT NULL DEFAULT 0,
    source_sha256 TEXT,
    destination_sha256 TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (operation_id, destination_path)
);

CREATE INDEX idx_file_operation_items_state
    ON file_operation_items(operation_id, state);

CREATE VIRTUAL TABLE track_search USING fts5(
    track_id UNINDEXED,
    title,
    artist,
    album,
    genre,
    subgenre,
    filename,
    path,
    comment,
    user_notes,
    tokenize = 'unicode61 remove_diacritics 2'
);

INSERT INTO schema_migrations(version) VALUES (1);
PRAGMA user_version = 1;
