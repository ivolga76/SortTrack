import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { DatabaseSync } from "node:sqlite";
import { applyCurrentMigrations, assertHealthyDatabase, CURRENT_SCHEMA_VERSION } from "./sqlite-profile.mjs";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDirectory, "..");
const genres = ["Breaks", "Drum & Bass", "House", "Techno", "Trance", "Hardcore", "Hardstyle", "Electropunk"];
const keys = [
  ["Ab minor", "1A"], ["Eb minor", "2A"], ["Bb minor", "3A"], ["F minor", "4A"],
  ["C minor", "5A"], ["G minor", "6A"], ["D minor", "7A"], ["A minor", "8A"],
  ["E minor", "9A"], ["B minor", "10A"], ["F# minor", "11A"], ["C# minor", "12A"],
];

function parseInteger(value, name, minimum, maximum) {
  if (!/^\d+$/.test(value ?? "")) throw new Error(`${name} must be an integer`);
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed) || parsed < minimum || parsed > maximum) {
    throw new Error(`${name} must be between ${minimum} and ${maximum}`);
  }
  return parsed;
}

export function parseBenchmarkArguments(args) {
  const options = { outputPath: null, trackCount: 100_000, playlistCount: 1_000, largePlaylistItems: 10_000 };
  for (let index = 0; index < args.length; index += 2) {
    const flag = args[index];
    const value = args[index + 1];
    if (!value) throw new Error(`Missing value for ${flag ?? "argument"}`);
    if (flag === "--output") options.outputPath = path.resolve(value);
    else if (flag === "--tracks") options.trackCount = parseInteger(value, "tracks", 1, 100_000);
    else if (flag === "--playlists") options.playlistCount = parseInteger(value, "playlists", 1, 10_000);
    else if (flag === "--large-playlist-items") options.largePlaylistItems = parseInteger(value, "large playlist items", 0, 100_000);
    else throw new Error(`Unknown argument: ${flag}`);
  }
  if (!options.outputPath) throw new Error("--output is required; the generator never chooses a profile path automatically");
  if (options.largePlaylistItems > options.trackCount) throw new Error("large playlist items cannot exceed track count");
  return options;
}

function isInside(candidate, parent) {
  const relative = path.relative(parent, candidate);
  return relative === "" || (!relative.startsWith("..") && !path.isAbsolute(relative));
}

function assertSafeOutput(outputPath) {
  const installedDataRoot = process.env.LOCALAPPDATA ? path.resolve(process.env.LOCALAPPDATA, "SortTrack") : null;
  if (installedDataRoot && isInside(outputPath, installedDataRoot)) {
    throw new Error("Refusing to generate a benchmark database inside the installed SortTrack profile");
  }
  if (fs.existsSync(outputPath)) throw new Error(`Refusing to overwrite existing file: ${outputPath}`);
  const manifestPath = `${outputPath}.manifest.json`;
  if (fs.existsSync(manifestPath)) throw new Error(`Refusing to overwrite existing file: ${manifestPath}`);
}

function sha256File(filePath) {
  const hash = crypto.createHash("sha256");
  hash.update(fs.readFileSync(filePath));
  return hash.digest("hex");
}

function suspendTrackSearchMaintenance(database, migrationsDirectory) {
  const triggerRows = database.prepare("SELECT name FROM sqlite_master WHERE type = 'trigger' AND name LIKE 'track_search_%' ORDER BY name").all();
  if (triggerRows.length === 0) throw new Error("Track search maintenance triggers were not found");
  for (const row of triggerRows) {
    if (!/^[a-z0-9_]+$/.test(row.name)) throw new Error(`Unsafe trigger name: ${row.name}`);
    database.exec(`DROP TRIGGER ${row.name}`);
  }
  const migration = fs.readFileSync(path.join(migrationsDirectory, "0009_track_search_index.sql"), "utf8");
  const triggerStatements = [...migration.matchAll(/CREATE TRIGGER[\s\S]*?END;/g)].map((match) => match[0]);
  if (triggerStatements.length !== triggerRows.length) {
    throw new Error(`Expected ${triggerRows.length} search trigger definitions, found ${triggerStatements.length}`);
  }
  return triggerStatements;
}

function rebuildTrackSearch(database, triggerStatements) {
  database.exec(`
    DELETE FROM track_search;
    INSERT INTO track_search(track_id, title, artist, album, genre, subgenre, filename, path, comment, user_notes)
    SELECT t.id,
           coalesce(m.title, t.preferred_title, f.filename),
           coalesce(m.artist, t.preferred_artist, ''),
           coalesce(m.album, ''),
           coalesce(t.confirmed_genre, ''),
           coalesce(t.confirmed_subgenre, ''),
           f.filename,
           f.canonical_path,
           coalesce(m.comment, ''),
           coalesce(t.user_notes, '')
    FROM tracks t
    JOIN track_files f ON f.track_id = t.id
    LEFT JOIN track_metadata m ON m.track_file_id = f.id;
  `);
  for (const statement of triggerStatements) database.exec(statement);
}

export function generateBenchmarkProfile({ outputPath, trackCount, playlistCount, largePlaylistItems, root = projectRoot }) {
  const destination = path.resolve(outputPath);
  assertSafeOutput(destination);
  fs.mkdirSync(path.dirname(destination), { recursive: true });
  const partial = `${destination}.partial-${process.pid}`;
  const manifestPath = `${destination}.manifest.json`;
  const partialManifest = `${manifestPath}.partial-${process.pid}`;
  if (fs.existsSync(partial) || fs.existsSync(partialManifest)) {
    throw new Error("A generator partial file already exists; inspect it before retrying");
  }

  let database;
  try {
    database = new DatabaseSync(partial);
    database.exec("PRAGMA journal_mode = DELETE; PRAGMA synchronous = OFF; PRAGMA temp_store = MEMORY; PRAGMA foreign_keys = ON;");
    const migrationsDirectory = path.join(root, "src-tauri", "migrations");
    applyCurrentMigrations(database, migrationsDirectory);
    const searchTriggerStatements = suspendTrackSearchMaintenance(database, migrationsDirectory);

    const insertRoot = database.prepare("INSERT INTO library_roots(id, canonical_path, volume_id, status, last_scanned_at) VALUES (?, ?, ?, 'unavailable', ?)");
    const insertTrack = database.prepare("INSERT INTO tracks(id, preferred_title, preferred_artist, confirmed_genre, genre_locked, rating, created_at, updated_at) VALUES (?, ?, ?, ?, 1, ?, ?, ?)");
    const insertFile = database.prepare("INSERT INTO track_files(id, track_id, library_root_id, canonical_path, normalized_path_key, filename, extension, size_bytes, modified_at_ms, file_identity, codec, duration_ms, bitrate_kbps, sample_rate_hz, channels, availability_status, metadata_reader_version, last_scanned_at, created_at, updated_at) VALUES (?, ?, 'benchmark-root', ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 2, 'available', 1, ?, ?, ?)");
    const insertMetadata = database.prepare("INSERT INTO track_metadata(track_file_id, title, artist, album, original_genre_tag, year, bpm, musical_key, camelot, metadata_fingerprint, read_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)");
    const insertPlaylist = database.prepare("INSERT INTO playlists(id, name, kind, created_at, updated_at) VALUES (?, ?, 'manual', ?, ?)");
    const insertPlaylistItem = database.prepare("INSERT INTO playlist_items(id, playlist_id, track_id, position, added_at) VALUES (?, 'benchmark-playlist-0000', ?, ?, ?)");

    const generatedAt = "2026-08-12T00:00:00Z";
    database.exec("BEGIN IMMEDIATE");
    insertRoot.run("benchmark-root", "X:\\SortTrackBenchmark\\Music", "benchmark-volume", generatedAt);
    for (let index = 0; index < trackCount; index += 1) {
      const suffix = String(index).padStart(6, "0");
      const trackId = `benchmark-track-${suffix}`;
      const fileId = `benchmark-file-${suffix}`;
      const title = `Synthetic Track ${suffix}`;
      const artist = `Synthetic Artist ${String(index % 500).padStart(3, "0")}`;
      const album = `Synthetic Album ${String(index % 250).padStart(3, "0")}`;
      const genre = genres[index % genres.length];
      const [musicalKey, camelot] = keys[index % keys.length];
      const bpm = 80 + (index % 101);
      const extension = index % 3 === 0 ? "flac" : "mp3";
      const filename = `${artist} - ${title}.${extension}`;
      const canonicalPath = `X:\\SortTrackBenchmark\\Music\\${String(index % 100).padStart(2, "0")}\\${filename}`;
      const normalizedPath = canonicalPath.toLowerCase();
      const sizeBytes = extension === "flac" ? 45_000_000 + index : 12_000_000 + index;
      const modifiedAtMs = 1_785_600_000_000 + index;
      insertTrack.run(trackId, title, artist, genre, index % 6, generatedAt, generatedAt);
      insertFile.run(fileId, trackId, canonicalPath, normalizedPath, filename, extension, sizeBytes, modifiedAtMs, `benchmark-volume:${index}`, extension, 180_000 + (index % 360_000), extension === "mp3" ? 320 : null, 44_100, generatedAt, generatedAt, generatedAt);
      insertMetadata.run(fileId, title, artist, album, genre, String(1990 + (index % 36)), bpm, musicalKey, camelot, `benchmark-metadata-${suffix}`, generatedAt, generatedAt);
    }
    for (let index = 0; index < playlistCount; index += 1) {
      const suffix = String(index).padStart(4, "0");
      insertPlaylist.run(`benchmark-playlist-${suffix}`, `Synthetic Playlist ${suffix}`, generatedAt, generatedAt);
    }
    for (let index = 0; index < largePlaylistItems; index += 1) {
      const trackSuffix = String(index).padStart(6, "0");
      const itemSuffix = String(index).padStart(5, "0");
      insertPlaylistItem.run(`benchmark-item-${itemSuffix}`, `benchmark-track-${trackSuffix}`, index, generatedAt);
    }
    database.exec("COMMIT");
    database.exec("BEGIN IMMEDIATE");
    rebuildTrackSearch(database, searchTriggerStatements);
    database.exec("COMMIT");
    database.exec("ANALYZE");
    assertHealthyDatabase(database);

    const counts = {
      tracks: Number(database.prepare("SELECT COUNT(*) AS count FROM tracks").get().count),
      trackFiles: Number(database.prepare("SELECT COUNT(*) AS count FROM track_files").get().count),
      searchRows: Number(database.prepare("SELECT COUNT(*) AS count FROM track_search").get().count),
      playlists: Number(database.prepare("SELECT COUNT(*) AS count FROM playlists").get().count),
      largePlaylistItems: Number(database.prepare("SELECT COUNT(*) AS count FROM playlist_items WHERE playlist_id = 'benchmark-playlist-0000'").get().count),
    };
    if (counts.tracks !== trackCount || counts.trackFiles !== trackCount || counts.searchRows !== trackCount || counts.playlists !== playlistCount || counts.largePlaylistItems !== largePlaylistItems) {
      throw new Error(`Generated counts do not match request: ${JSON.stringify(counts)}`);
    }
    database.close();
    database = null;
    fs.renameSync(partial, destination);
    const manifest = {
      schema: "sorttrack-synthetic-benchmark-profile-1",
      applicationSchemaVersion: CURRENT_SCHEMA_VERSION,
      synthetic: true,
      containsAudio: false,
      generatedAt,
      generatorHost: { platform: os.platform(), arch: os.arch(), node: process.version },
      counts,
      databaseSizeBytes: fs.statSync(destination).size,
      databaseSha256: sha256File(destination),
    };
    fs.writeFileSync(partialManifest, `${JSON.stringify(manifest, null, 2)}\n`, { encoding: "utf8", flag: "wx" });
    fs.renameSync(partialManifest, manifestPath);
    return { outputPath: destination, manifestPath, ...manifest };
  } catch (error) {
    if (database) database.close();
    for (const ownedPath of [partial, partialManifest]) {
      if (fs.existsSync(ownedPath)) fs.rmSync(ownedPath, { force: true });
    }
    throw error;
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const options = parseBenchmarkArguments(process.argv.slice(2));
  const result = generateBenchmarkProfile(options);
  process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
}
