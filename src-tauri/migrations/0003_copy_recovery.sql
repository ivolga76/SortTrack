ALTER TABLE file_operation_items ADD COLUMN planned_status TEXT NOT NULL DEFAULT 'ready';
ALTER TABLE file_operation_items ADD COLUMN chosen_action TEXT;
ALTER TABLE file_operation_items ADD COLUMN expected_size_bytes INTEGER NOT NULL DEFAULT 0;
ALTER TABLE file_operation_items ADD COLUMN expected_modified_at_ms INTEGER NOT NULL DEFAULT 0;

UPDATE file_operation_items
SET planned_status = CASE
        WHEN reason IN ('ready', 'identical', 'conflict', 'missing', 'invalid', 'permission_error') THEN reason
        ELSE 'ready'
    END,
    expected_size_bytes = coalesce((SELECT size_bytes FROM track_files WHERE id = track_file_id), 0),
    expected_modified_at_ms = coalesce((SELECT modified_at_ms FROM track_files WHERE id = track_file_id), 0);

INSERT INTO schema_migrations(version) VALUES (3);
PRAGMA user_version = 3;
