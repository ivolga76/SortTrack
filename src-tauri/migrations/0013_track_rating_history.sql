CREATE TABLE track_rating_history (
    id TEXT PRIMARY KEY,
    track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    before_rating INTEGER NOT NULL CHECK(before_rating BETWEEN 0 AND 5),
    after_rating INTEGER NOT NULL CHECK(after_rating BETWEEN 0 AND 5),
    source TEXT NOT NULL DEFAULT 'manual',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_track_rating_history_track_time
    ON track_rating_history(track_id, created_at DESC, id DESC);

INSERT INTO schema_migrations(version) VALUES (13);
PRAGMA user_version = 13;
