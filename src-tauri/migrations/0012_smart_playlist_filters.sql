ALTER TABLE smart_playlist_rules RENAME TO smart_playlist_rules_v11;

CREATE TABLE smart_playlist_rules (
    playlist_id TEXT PRIMARY KEY REFERENCES playlists(id) ON DELETE CASCADE,
    genre TEXT,
    subgenre TEXT,
    min_bpm REAL,
    max_bpm REAL,
    min_rating INTEGER CHECK(min_rating IS NULL OR min_rating BETWEEN 1 AND 5),
    camelot TEXT,
    limit_count INTEGER NOT NULL DEFAULT 200 CHECK(limit_count BETWEEN 1 AND 1000),
    sort_by TEXT NOT NULL DEFAULT 'added' CHECK(sort_by IN ('added', 'title', 'bpm')),
    descending INTEGER NOT NULL DEFAULT 1 CHECK(descending IN (0, 1)),
    CHECK(genre IS NOT NULL OR min_bpm IS NOT NULL OR max_bpm IS NOT NULL OR min_rating IS NOT NULL OR camelot IS NOT NULL),
    CHECK(subgenre IS NULL OR genre IS NOT NULL),
    CHECK(min_bpm IS NULL OR (min_bpm BETWEEN 20 AND 300)),
    CHECK(max_bpm IS NULL OR (max_bpm BETWEEN 20 AND 300)),
    CHECK(min_bpm IS NULL OR max_bpm IS NULL OR min_bpm <= max_bpm)
);

INSERT INTO smart_playlist_rules(playlist_id, genre, min_bpm, max_bpm, limit_count, sort_by, descending)
SELECT playlist_id, genre, min_bpm, max_bpm, limit_count, sort_by, descending
FROM smart_playlist_rules_v11;

DROP TABLE smart_playlist_rules_v11;

INSERT INTO schema_migrations(version) VALUES (12);
PRAGMA user_version = 12;
