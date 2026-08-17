DELETE FROM track_search;

CREATE INDEX idx_tracks_genre_created ON tracks(confirmed_genre, created_at DESC);
CREATE INDEX idx_tracks_title ON tracks(preferred_title COLLATE NOCASE);
CREATE INDEX idx_tracks_artist ON tracks(preferred_artist COLLATE NOCASE);

INSERT INTO track_search(track_id, title, artist, album, genre, subgenre, filename, path, comment, user_notes)
SELECT t.id, coalesce(m.title, t.preferred_title, f.filename), coalesce(m.artist, t.preferred_artist, ''), coalesce(m.album, ''), coalesce(t.confirmed_genre, ''), coalesce(t.confirmed_subgenre, ''), f.filename, f.canonical_path, coalesce(m.comment, ''), coalesce(t.user_notes, '')
FROM tracks t
JOIN track_files f ON f.track_id = t.id
LEFT JOIN track_metadata m ON m.track_file_id = f.id;

CREATE TRIGGER track_search_tracks_update AFTER UPDATE OF preferred_title, preferred_artist, confirmed_genre, confirmed_subgenre, user_notes ON tracks BEGIN
    DELETE FROM track_search WHERE track_id = NEW.id;
    INSERT INTO track_search(track_id, title, artist, album, genre, subgenre, filename, path, comment, user_notes)
    SELECT NEW.id, coalesce(m.title, NEW.preferred_title, f.filename), coalesce(m.artist, NEW.preferred_artist, ''), coalesce(m.album, ''), coalesce(NEW.confirmed_genre, ''), coalesce(NEW.confirmed_subgenre, ''), f.filename, f.canonical_path, coalesce(m.comment, ''), coalesce(NEW.user_notes, '')
    FROM track_files f LEFT JOIN track_metadata m ON m.track_file_id = f.id WHERE f.track_id = NEW.id;
END;

CREATE TRIGGER track_search_tracks_delete AFTER DELETE ON tracks BEGIN
    DELETE FROM track_search WHERE track_id = OLD.id;
END;

CREATE TRIGGER track_search_files_insert AFTER INSERT ON track_files BEGIN
    DELETE FROM track_search WHERE track_id = NEW.track_id;
    INSERT INTO track_search(track_id, title, artist, album, genre, subgenre, filename, path, comment, user_notes)
    SELECT t.id, coalesce(m.title, t.preferred_title, f.filename), coalesce(m.artist, t.preferred_artist, ''), coalesce(m.album, ''), coalesce(t.confirmed_genre, ''), coalesce(t.confirmed_subgenre, ''), f.filename, f.canonical_path, coalesce(m.comment, ''), coalesce(t.user_notes, '')
    FROM tracks t JOIN track_files f ON f.track_id = t.id LEFT JOIN track_metadata m ON m.track_file_id = f.id WHERE t.id = NEW.track_id;
END;

CREATE TRIGGER track_search_files_update AFTER UPDATE OF track_id, canonical_path, filename ON track_files BEGIN
    DELETE FROM track_search WHERE track_id IN (OLD.track_id, NEW.track_id);
    INSERT INTO track_search(track_id, title, artist, album, genre, subgenre, filename, path, comment, user_notes)
    SELECT t.id, coalesce(m.title, t.preferred_title, f.filename), coalesce(m.artist, t.preferred_artist, ''), coalesce(m.album, ''), coalesce(t.confirmed_genre, ''), coalesce(t.confirmed_subgenre, ''), f.filename, f.canonical_path, coalesce(m.comment, ''), coalesce(t.user_notes, '')
    FROM tracks t JOIN track_files f ON f.track_id = t.id LEFT JOIN track_metadata m ON m.track_file_id = f.id WHERE t.id IN (OLD.track_id, NEW.track_id);
END;

CREATE TRIGGER track_search_files_delete AFTER DELETE ON track_files BEGIN
    DELETE FROM track_search WHERE track_id = OLD.track_id;
    INSERT INTO track_search(track_id, title, artist, album, genre, subgenre, filename, path, comment, user_notes)
    SELECT t.id, coalesce(m.title, t.preferred_title, f.filename), coalesce(m.artist, t.preferred_artist, ''), coalesce(m.album, ''), coalesce(t.confirmed_genre, ''), coalesce(t.confirmed_subgenre, ''), f.filename, f.canonical_path, coalesce(m.comment, ''), coalesce(t.user_notes, '')
    FROM tracks t JOIN track_files f ON f.track_id = t.id LEFT JOIN track_metadata m ON m.track_file_id = f.id WHERE t.id = OLD.track_id;
END;

CREATE TRIGGER track_search_metadata_insert AFTER INSERT ON track_metadata BEGIN
    DELETE FROM track_search WHERE track_id = (SELECT track_id FROM track_files WHERE id = NEW.track_file_id);
    INSERT INTO track_search(track_id, title, artist, album, genre, subgenre, filename, path, comment, user_notes)
    SELECT t.id, coalesce(m.title, t.preferred_title, f.filename), coalesce(m.artist, t.preferred_artist, ''), coalesce(m.album, ''), coalesce(t.confirmed_genre, ''), coalesce(t.confirmed_subgenre, ''), f.filename, f.canonical_path, coalesce(m.comment, ''), coalesce(t.user_notes, '')
    FROM tracks t JOIN track_files f ON f.track_id = t.id LEFT JOIN track_metadata m ON m.track_file_id = f.id WHERE t.id = (SELECT track_id FROM track_files WHERE id = NEW.track_file_id);
END;

CREATE TRIGGER track_search_metadata_update AFTER UPDATE ON track_metadata BEGIN
    DELETE FROM track_search WHERE track_id = (SELECT track_id FROM track_files WHERE id = NEW.track_file_id);
    INSERT INTO track_search(track_id, title, artist, album, genre, subgenre, filename, path, comment, user_notes)
    SELECT t.id, coalesce(m.title, t.preferred_title, f.filename), coalesce(m.artist, t.preferred_artist, ''), coalesce(m.album, ''), coalesce(t.confirmed_genre, ''), coalesce(t.confirmed_subgenre, ''), f.filename, f.canonical_path, coalesce(m.comment, ''), coalesce(t.user_notes, '')
    FROM tracks t JOIN track_files f ON f.track_id = t.id LEFT JOIN track_metadata m ON m.track_file_id = f.id WHERE t.id = (SELECT track_id FROM track_files WHERE id = NEW.track_file_id);
END;

CREATE TRIGGER track_search_metadata_delete AFTER DELETE ON track_metadata BEGIN
    DELETE FROM track_search WHERE track_id = (SELECT track_id FROM track_files WHERE id = OLD.track_file_id);
    INSERT INTO track_search(track_id, title, artist, album, genre, subgenre, filename, path, comment, user_notes)
    SELECT t.id, coalesce(t.preferred_title, f.filename), coalesce(t.preferred_artist, ''), '', coalesce(t.confirmed_genre, ''), coalesce(t.confirmed_subgenre, ''), f.filename, f.canonical_path, '', coalesce(t.user_notes, '')
    FROM tracks t JOIN track_files f ON f.track_id = t.id WHERE t.id = (SELECT track_id FROM track_files WHERE id = OLD.track_file_id);
END;

INSERT INTO schema_migrations(version) VALUES (9);
PRAGMA user_version = 9;
