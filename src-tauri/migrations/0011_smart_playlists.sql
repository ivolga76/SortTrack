CREATE TABLE smart_playlist_rules (
    playlist_id TEXT PRIMARY KEY REFERENCES playlists(id) ON DELETE CASCADE,
    genre TEXT,
    min_bpm REAL,
    max_bpm REAL,
    limit_count INTEGER NOT NULL DEFAULT 200 CHECK(limit_count BETWEEN 1 AND 1000),
    sort_by TEXT NOT NULL DEFAULT 'added' CHECK(sort_by IN ('added', 'title', 'bpm')),
    descending INTEGER NOT NULL DEFAULT 1 CHECK(descending IN (0, 1)),
    CHECK(genre IS NOT NULL OR min_bpm IS NOT NULL OR max_bpm IS NOT NULL),
    CHECK(min_bpm IS NULL OR (min_bpm BETWEEN 20 AND 300)),
    CHECK(max_bpm IS NULL OR (max_bpm BETWEEN 20 AND 300)),
    CHECK(min_bpm IS NULL OR max_bpm IS NULL OR min_bpm <= max_bpm)
);

INSERT INTO schema_migrations(version) VALUES (11);
PRAGMA user_version = 11;
