-- Anonymized characterization fixture for the current SortTrack schema v13.
-- Paths are deliberately synthetic and must never point to user music.

INSERT INTO library_roots(
    id, canonical_path, volume_id, scan_subfolders, exclusions_json, status, created_at, last_scanned_at
) VALUES (
    'fixture-root', 'X:\SortTrackFixture\Music', 'fixture-volume', 1, '["archive","temp"]',
    'unavailable', '2026-08-01T10:00:00Z', '2026-08-02T10:00:00Z'
);

INSERT INTO tracks(
    id, preferred_title, preferred_artist, confirmed_genre, confirmed_subgenre,
    genre_locked, rating, created_at, updated_at
) VALUES
    ('fixture-track-1', 'Synthetic Pulse', 'Fixture Artist A', 'Techno', 'Raw Techno', 1, 5, '2026-08-01T10:01:00Z', '2026-08-01T10:01:00Z'),
    ('fixture-track-2', 'Synthetic Break', 'Fixture Artist B', 'Breaks', NULL, 1, 3, '2026-08-01T10:02:00Z', '2026-08-01T10:02:00Z'),
    ('fixture-track-3', 'Missing Fixture', 'Fixture Artist C', NULL, NULL, 0, 0, '2026-08-01T10:03:00Z', '2026-08-01T10:03:00Z');

INSERT INTO track_files(
    id, track_id, library_root_id, canonical_path, normalized_path_key, filename, extension,
    size_bytes, modified_at_ms, file_identity, sha256, sha256_state, codec, duration_ms,
    bitrate_kbps, sample_rate_hz, bit_depth, channels, availability_status,
    last_scanned_at, created_at, updated_at, metadata_reader_version
) VALUES
    ('fixture-file-1-flac', 'fixture-track-1', 'fixture-root', 'X:\SortTrackFixture\Music\Fixture Artist A - Synthetic Pulse.flac', 'x:\sorttrackfixture\music\fixture artist a - synthetic pulse.flac', 'Fixture Artist A - Synthetic Pulse.flac', 'flac', 50000000, 1785600000000, 'fixture-volume:1', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', 'ready', 'flac', 360000, NULL, 44100, 16, 2, 'available', '2026-08-02T10:00:00Z', '2026-08-01T10:01:00Z', '2026-08-02T10:00:00Z', 1),
    ('fixture-file-1-mp3', 'fixture-track-1', 'fixture-root', 'X:\SortTrackFixture\Music\Fixture Artist A - Synthetic Pulse.mp3', 'x:\sorttrackfixture\music\fixture artist a - synthetic pulse.mp3', 'Fixture Artist A - Synthetic Pulse.mp3', 'mp3', 14000000, 1785600001000, 'fixture-volume:2', NULL, 'not_computed', 'mp3', 360000, 320, 44100, NULL, 2, 'available', '2026-08-02T10:00:00Z', '2026-08-01T10:01:30Z', '2026-08-02T10:00:00Z', 1),
    ('fixture-file-2', 'fixture-track-2', 'fixture-root', 'X:\SortTrackFixture\Music\Fixture Artist B - Synthetic Break.mp3', 'x:\sorttrackfixture\music\fixture artist b - synthetic break.mp3', 'Fixture Artist B - Synthetic Break.mp3', 'mp3', 12000000, 1785600002000, 'fixture-volume:3', NULL, 'not_computed', 'mp3', 300000, 320, 48000, NULL, 2, 'available', '2026-08-02T10:00:00Z', '2026-08-01T10:02:00Z', '2026-08-02T10:00:00Z', 1),
    ('fixture-file-3', 'fixture-track-3', 'fixture-root', 'X:\SortTrackFixture\Music\Missing Fixture.flac', 'x:\sorttrackfixture\music\missing fixture.flac', 'Missing Fixture.flac', 'flac', 42000000, 1785600003000, 'fixture-volume:4', NULL, 'stale', 'flac', 280000, NULL, 44100, 24, 2, 'missing', '2026-08-02T10:00:00Z', '2026-08-01T10:03:00Z', '2026-08-02T10:00:00Z', 1);

INSERT INTO track_metadata(
    track_file_id, title, artist, album, album_artist, track_number, disc_number,
    original_genre_tag, year, composer, comment, bpm, musical_key, camelot,
    metadata_fingerprint, read_at, updated_at
) VALUES
    ('fixture-file-1-flac', 'Synthetic Pulse', 'Fixture Artist A', 'Fixture Album', 'Fixture Artist A', '1', '1', 'Raw Techno', '2024', NULL, NULL, 145.0, 'F minor', '4A', 'fixture-metadata-1', '2026-08-02T10:00:00Z', '2026-08-02T10:00:00Z'),
    ('fixture-file-1-mp3', 'Synthetic Pulse', 'Fixture Artist A', 'Fixture Album', 'Fixture Artist A', '1', '1', 'Raw Techno', '2024', NULL, NULL, 145.0, 'F minor', '4A', 'fixture-metadata-1-mp3', '2026-08-02T10:00:00Z', '2026-08-02T10:00:00Z'),
    ('fixture-file-2', 'Synthetic Break', 'Fixture Artist B', 'Fixture Album B', NULL, '2', NULL, 'Breaks', '2022', NULL, NULL, 132.0, 'A minor', '8A', 'fixture-metadata-2', '2026-08-02T10:00:00Z', '2026-08-02T10:00:00Z'),
    ('fixture-file-3', NULL, 'Fixture Artist C', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 'fixture-metadata-3', '2026-08-02T10:00:00Z', '2026-08-02T10:00:00Z');

INSERT INTO track_audio_analysis(
    track_file_id, bpm, bpm_score, bpm_algorithm_version, musical_key, key_score,
    key_algorithm_version, camelot, file_identity, size_bytes, modified_at_ms, analyzed_at
) VALUES (
    'fixture-file-2', 132.1, 0.92, 'fixture-bpm-v1', 'A minor', 0.88,
    'fixture-key-v1', '8A', 'fixture-volume:3', 12000000, 1785600002000, '2026-08-02T10:05:00Z'
);

INSERT INTO metadata_history(
    id, track_file_id, operation_id, before_json, after_json,
    file_state_before, file_state_after, created_at
) VALUES (
    'fixture-metadata-history-1', 'fixture-file-1-flac', 'fixture-operation-metadata-1',
    '{"genre":"Techno"}', '{"genre":"Raw Techno"}',
    'fixture-state-before', 'fixture-state-after', '2026-08-02T11:00:00Z'
);

INSERT INTO genre_confirmation_history(
    id, track_id, before_genre, before_subgenre, before_locked,
    after_genre, after_subgenre, after_locked, source, created_at
) VALUES (
    'fixture-genre-history-1', 'fixture-track-1', 'Techno', NULL, 0,
    'Techno', 'Raw Techno', 1, 'manual', '2026-08-02T11:05:00Z'
);

INSERT INTO track_rating_history(
    id, track_id, before_rating, after_rating, source, created_at
) VALUES (
    'fixture-rating-history-1', 'fixture-track-1', 4, 5, 'manual', '2026-08-02T11:10:00Z'
);

INSERT INTO playlists(id, name, kind, created_at, updated_at) VALUES
    ('fixture-playlist-manual', 'Fixture Manual Playlist', 'manual', '2026-08-02T12:00:00Z', '2026-08-02T12:00:00Z'),
    ('fixture-playlist-smart', 'Fixture Smart Playlist', 'smart', '2026-08-02T12:01:00Z', '2026-08-02T12:01:00Z');

INSERT INTO playlist_items(id, playlist_id, track_id, position, added_at) VALUES
    ('fixture-item-1', 'fixture-playlist-manual', 'fixture-track-1', 0, '2026-08-02T12:00:00Z'),
    ('fixture-item-2', 'fixture-playlist-manual', 'fixture-track-2', 1, '2026-08-02T12:00:01Z');

INSERT INTO smart_playlist_rules(
    playlist_id, genre, subgenre, min_bpm, max_bpm, min_rating, camelot,
    limit_count, sort_by, descending
) VALUES (
    'fixture-playlist-smart', 'Techno', NULL, 130.0, 160.0, 4, NULL, 100, 'bpm', 0
);

INSERT INTO background_jobs(
    id, job_type, state, priority, payload_json, processed_items, total_items,
    processed_bytes, total_bytes, created_at, started_at, finished_at, updated_at
) VALUES (
    'fixture-job-1', 'audio_analysis', 'completed', 0, '{"fixture":true}', 1, 1,
    12000000, 12000000, '2026-08-02T13:00:00Z', '2026-08-02T13:00:01Z',
    '2026-08-02T13:00:05Z', '2026-08-02T13:00:05Z'
);

INSERT INTO background_job_items(
    id, job_id, track_file_id, state, processed_bytes, result_json,
    created_at, started_at, finished_at, updated_at
) VALUES (
    'fixture-job-item-1', 'fixture-job-1', 'fixture-file-2', 'completed', 12000000,
    '{"bpm":132.1,"camelot":"8A"}', '2026-08-02T13:00:00Z',
    '2026-08-02T13:00:01Z', '2026-08-02T13:00:05Z', '2026-08-02T13:00:05Z'
);

INSERT INTO app_settings(key, value_json, updated_at) VALUES (
    'library_table_layout',
    '{"columns":[{"id":"track","visible":true,"width":320},{"id":"genre","visible":true,"width":170},{"id":"bpm","visible":true,"width":72},{"id":"key","visible":true,"width":130},{"id":"format","visible":true,"width":82},{"id":"duration","visible":true,"width":76}]}',
    '2026-08-02T14:00:00Z'
);
