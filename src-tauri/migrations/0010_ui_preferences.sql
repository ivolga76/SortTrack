CREATE TABLE app_settings (
    key TEXT PRIMARY KEY NOT NULL CHECK(length(key) BETWEEN 1 AND 80),
    value_json TEXT NOT NULL CHECK(json_valid(value_json)),
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO schema_migrations(version) VALUES (10);
PRAGMA user_version = 10;
