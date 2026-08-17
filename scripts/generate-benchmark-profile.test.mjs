import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { DatabaseSync } from "node:sqlite";
import { describe, expect, it } from "vitest";
import { generateBenchmarkProfile, parseBenchmarkArguments } from "./generate-benchmark-profile.mjs";

describe("synthetic benchmark profile generator", () => {
  it("requires an explicit bounded destination and counts", () => {
    expect(() => parseBenchmarkArguments([])).toThrow("--output is required");
    expect(() => parseBenchmarkArguments(["--output", "profile.db", "--tracks", "100001"])).toThrow("tracks must be between");
    expect(() => parseBenchmarkArguments(["--output", "profile.db", "--tracks", "5", "--large-playlist-items", "6"])).toThrow("cannot exceed track count");
  });

  it("creates a healthy v13 profile made only from synthetic records", () => {
    const directory = fs.mkdtempSync(path.join(os.tmpdir(), "sorttrack-benchmark-test-"));
    const outputPath = path.join(directory, "benchmark.db");
    try {
      const report = generateBenchmarkProfile({ outputPath, trackCount: 25, playlistCount: 3, largePlaylistItems: 10 });
      expect(report).toMatchObject({
        synthetic: true,
        containsAudio: false,
        applicationSchemaVersion: 13,
        counts: { tracks: 25, trackFiles: 25, searchRows: 25, playlists: 3, largePlaylistItems: 10 },
      });
      expect(report.databaseSha256).toMatch(/^[a-f0-9]{64}$/);

      const database = new DatabaseSync(outputPath, { readOnly: true });
      expect(database.prepare("PRAGMA user_version").get().user_version).toBe(13);
      expect(database.prepare("SELECT COUNT(*) AS count FROM track_search WHERE track_search MATCH 'Synthetic'").get().count).toBe(25);
      database.close();
      expect(JSON.parse(fs.readFileSync(report.manifestPath, "utf8")).databaseSha256).toBe(report.databaseSha256);
      expect(() => generateBenchmarkProfile({ outputPath, trackCount: 1, playlistCount: 1, largePlaylistItems: 0 })).toThrow("Refusing to overwrite");
    } finally {
      fs.rmSync(directory, { recursive: true, force: true });
    }
  });
});
