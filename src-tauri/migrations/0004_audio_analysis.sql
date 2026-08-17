CREATE TABLE track_audio_analysis (
    track_file_id TEXT PRIMARY KEY REFERENCES track_files(id) ON DELETE CASCADE,
    bpm REAL,
    bpm_score REAL,
    bpm_algorithm_version TEXT,
    musical_key TEXT,
    key_score REAL,
    key_algorithm_version TEXT,
    camelot TEXT,
    file_identity TEXT,
    size_bytes INTEGER NOT NULL,
    modified_at_ms INTEGER NOT NULL,
    analyzed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    error TEXT
);

CREATE TABLE background_job_items (
    id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL REFERENCES background_jobs(id) ON DELETE CASCADE,
    track_file_id TEXT NOT NULL REFERENCES track_files(id) ON DELETE RESTRICT,
    state TEXT NOT NULL CHECK (state IN ('queued', 'running', 'completed', 'failed', 'cancelled')),
    processed_bytes INTEGER NOT NULL DEFAULT 0,
    error TEXT,
    result_json TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    started_at TEXT,
    finished_at TEXT,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(job_id, track_file_id)
);

CREATE INDEX idx_background_job_items_queue ON background_job_items(job_id, state, created_at);

INSERT INTO schema_migrations(version) VALUES (4);
PRAGMA user_version = 4;
