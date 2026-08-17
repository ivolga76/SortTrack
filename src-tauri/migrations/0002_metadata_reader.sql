ALTER TABLE track_files ADD COLUMN metadata_reader_version INTEGER NOT NULL DEFAULT 0;

INSERT INTO schema_migrations(version) VALUES (2);
PRAGMA user_version = 2;
