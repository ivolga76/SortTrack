CREATE TABLE genre_dataset_exports (
    id TEXT PRIMARY KEY,
    destination_path TEXT NOT NULL,
    manifest_sha256 TEXT NOT NULL,
    taxonomy_version INTEGER NOT NULL REFERENCES genre_taxonomies(version),
    total_entries INTEGER NOT NULL,
    train_entries INTEGER NOT NULL,
    calibration_entries INTEGER NOT NULL,
    test_entries INTEGER NOT NULL,
    class_counts_json TEXT NOT NULL,
    report_json TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO schema_migrations(version) VALUES (6);
PRAGMA user_version = 6;
