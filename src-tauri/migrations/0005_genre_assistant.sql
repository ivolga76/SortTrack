CREATE TABLE genre_taxonomies (
    version INTEGER PRIMARY KEY,
    labels_json TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO genre_taxonomies(version, labels_json) VALUES (
    1,
    '{"Breaks":[],"Drum & Bass":[],"House":[],"Techno":["Melodic Techno","Hard Techno","Percussive Techno","Peak Time Techno","Acid Techno","Raw Techno","Schranz"],"Trance":[],"Hardcore":[],"Hardstyle":[],"Electropunk":[]}'
);

CREATE TABLE track_genre_candidates (
    id TEXT PRIMARY KEY,
    track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    rank INTEGER NOT NULL CHECK (rank BETWEEN 1 AND 3),
    genre TEXT NOT NULL,
    subgenre TEXT,
    score REAL NOT NULL CHECK (score BETWEEN 0 AND 1),
    signals_json TEXT NOT NULL DEFAULT '[]',
    inference_version TEXT NOT NULL,
    taxonomy_version INTEGER NOT NULL REFERENCES genre_taxonomies(version),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(track_id, rank)
);

CREATE INDEX idx_track_genre_candidates_track ON track_genre_candidates(track_id, rank);

CREATE TABLE genre_confirmation_history (
    id TEXT PRIMARY KEY,
    track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    before_genre TEXT,
    before_subgenre TEXT,
    before_locked INTEGER NOT NULL,
    after_genre TEXT NOT NULL,
    after_subgenre TEXT,
    after_locked INTEGER NOT NULL,
    source TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_genre_confirmation_history_track ON genre_confirmation_history(track_id, created_at DESC);

INSERT INTO schema_migrations(version) VALUES (5);
PRAGMA user_version = 5;
