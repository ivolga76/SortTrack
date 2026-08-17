import fs from "node:fs";
import path from "node:path";
import { performance } from "node:perf_hooks";
import { fileURLToPath } from "node:url";
import { DatabaseSync } from "node:sqlite";

function percentile(sorted, ratio) {
  return sorted[Math.min(sorted.length - 1, Math.max(0, Math.ceil(sorted.length * ratio) - 1))];
}

export function summarizeDurations(values) {
  if (values.length === 0) throw new Error("At least one duration is required");
  const sorted = [...values].sort((left, right) => left - right);
  return {
    minimumMs: Number(sorted[0].toFixed(3)),
    p50Ms: Number(percentile(sorted, 0.5).toFixed(3)),
    p95Ms: Number(percentile(sorted, 0.95).toFixed(3)),
    maximumMs: Number(sorted.at(-1).toFixed(3)),
  };
}

export function parseQueryBenchmarkArguments(args) {
  let databasePath = null;
  let iterations = 30;
  for (let index = 0; index < args.length; index += 2) {
    const flag = args[index];
    const value = args[index + 1];
    if (!value) throw new Error(`Missing value for ${flag ?? "argument"}`);
    if (flag === "--database") databasePath = path.resolve(value);
    else if (flag === "--iterations") {
      if (!/^\d+$/.test(value)) throw new Error("iterations must be an integer");
      iterations = Number(value);
      if (iterations < 5 || iterations > 500) throw new Error("iterations must be between 5 and 500");
    } else throw new Error(`Unknown argument: ${flag}`);
  }
  if (!databasePath) throw new Error("--database is required");
  return { databasePath, iterations };
}

function assertSyntheticProfile(databasePath) {
  const manifestPath = `${databasePath}.manifest.json`;
  if (!fs.existsSync(databasePath) || !fs.existsSync(manifestPath)) {
    throw new Error("The database and its adjacent synthetic manifest are required");
  }
  const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
  if (manifest.schema !== "sorttrack-synthetic-benchmark-profile-1" || manifest.synthetic !== true || manifest.containsAudio !== false) {
    throw new Error("Refusing to benchmark a database that is not marked as a synthetic SortTrack profile");
  }
  return manifest;
}

export function benchmarkCurrentQueries({ databasePath, iterations }) {
  const manifest = assertSyntheticProfile(databasePath);
  const database = new DatabaseSync(databasePath, { readOnly: true });
  const scenarios = [
    {
      name: "library_first_page",
      statement: database.prepare("SELECT t.id FROM tracks t JOIN track_files f ON f.id = (SELECT preferred.id FROM track_files preferred WHERE preferred.track_id = t.id ORDER BY CASE preferred.availability_status WHEN 'available' THEN 0 WHEN 'corrupted' THEN 1 WHEN 'unreadable' THEN 2 ELSE 3 END, CASE lower(preferred.extension) WHEN 'flac' THEN 0 ELSE 1 END, preferred.updated_at DESC, preferred.id LIMIT 1) ORDER BY t.created_at DESC, t.id ASC LIMIT 100 OFFSET 0"),
    },
    {
      name: "library_deep_offset_page",
      statement: database.prepare("SELECT t.id FROM tracks t JOIN track_files f ON f.id = (SELECT preferred.id FROM track_files preferred WHERE preferred.track_id = t.id ORDER BY CASE preferred.availability_status WHEN 'available' THEN 0 WHEN 'corrupted' THEN 1 WHEN 'unreadable' THEN 2 ELSE 3 END, CASE lower(preferred.extension) WHEN 'flac' THEN 0 ELSE 1 END, preferred.updated_at DESC, preferred.id LIMIT 1) ORDER BY t.created_at DESC, t.id ASC LIMIT 100 OFFSET 99900"),
    },
    {
      name: "fts_specific_track",
      statement: database.prepare("SELECT track_id FROM track_search WHERE track_search MATCH 'Synthetic AND 099999' LIMIT 100"),
    },
    {
      name: "combined_filter_count",
      statement: database.prepare("SELECT COUNT(*) AS count FROM tracks t JOIN track_files f ON f.track_id = t.id LEFT JOIN track_metadata m ON m.track_file_id = f.id WHERE t.confirmed_genre = 'Techno' AND m.bpm BETWEEN 130 AND 150 AND t.rating >= 3 AND m.camelot = '4A' AND lower(f.extension) = 'flac' AND f.availability_status = 'available'"),
    },
    {
      name: "playlist_10000_items",
      statement: database.prepare("SELECT pi.id, pi.position, t.id FROM playlist_items pi JOIN tracks t ON t.id = pi.track_id WHERE pi.playlist_id = 'benchmark-playlist-0000' ORDER BY pi.position"),
    },
  ];

  const results = {};
  try {
    for (const scenario of scenarios) {
      scenario.statement.all();
      const durations = [];
      for (let index = 0; index < iterations; index += 1) {
        const started = performance.now();
        scenario.statement.all();
        durations.push(performance.now() - started);
      }
      results[scenario.name] = summarizeDurations(durations);
    }
  } finally {
    database.close();
  }
  return {
    schema: "sorttrack-current-query-benchmark-1",
    databaseSha256: manifest.databaseSha256,
    counts: manifest.counts,
    iterations,
    results,
  };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const options = parseQueryBenchmarkArguments(process.argv.slice(2));
  process.stdout.write(`${JSON.stringify(benchmarkCurrentQueries(options), null, 2)}\n`);
}
