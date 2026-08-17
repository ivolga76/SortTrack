CREATE TABLE audio_feature_cache (
    track_file_id TEXT PRIMARY KEY REFERENCES track_files(id) ON DELETE CASCADE,
    extractor_version TEXT NOT NULL,
    features_json TEXT NOT NULL,
    file_identity TEXT,
    size_bytes INTEGER NOT NULL,
    modified_at_ms INTEGER NOT NULL,
    extracted_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE genre_model_runs (
    id TEXT PRIMARY KEY,
    dataset_export_id TEXT REFERENCES genre_dataset_exports(id) ON DELETE SET NULL,
    model_version TEXT NOT NULL,
    feature_version TEXT NOT NULL,
    taxonomy_version INTEGER NOT NULL REFERENCES genre_taxonomies(version),
    top1_accuracy REAL NOT NULL,
    meets_release_threshold INTEGER NOT NULL CHECK (meets_release_threshold IN (0, 1)),
    model_json TEXT NOT NULL,
    metrics_json TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO schema_migrations(version) VALUES (7);
PRAGMA user_version = 7;
