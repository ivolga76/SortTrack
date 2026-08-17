CREATE TABLE file_relink_history (
    id TEXT PRIMARY KEY,
    track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    track_file_id TEXT NOT NULL REFERENCES track_files(id) ON DELETE CASCADE,
    old_path TEXT NOT NULL,
    new_path TEXT NOT NULL,
    old_library_root_id TEXT,
    new_library_root_id TEXT,
    old_file_identity TEXT,
    new_file_identity TEXT,
    expected_state TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_file_relink_history_track_time
    ON file_relink_history(track_id, created_at DESC);

INSERT INTO schema_migrations(version) VALUES (8);
PRAGMA user_version = 8;
