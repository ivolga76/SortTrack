import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { buildAudioManifest, compareAudioManifests, parseManifestArguments } from "./mvp-file-manifest.mjs";

const temporaryRoots = [];

function temporaryRoot() {
  const root = mkdtempSync(join(tmpdir(), "sorttrack-mvp-manifest-"));
  temporaryRoots.push(root);
  return root;
}

afterEach(() => {
  for (const root of temporaryRoots.splice(0)) rmSync(root, { recursive: true, force: true });
});

describe("MVP file manifest", () => {
  it("hashes only MP3 and FLAC using stable relative paths", () => {
    const root = temporaryRoot();
    mkdirSync(join(root, "Жанр"));
    writeFileSync(join(root, "track.mp3"), "same audio bytes");
    writeFileSync(join(root, "Жанр", "TRACK.FLAC"), "same audio bytes");
    writeFileSync(join(root, "ignore.txt"), "not audio");

    const manifest = buildAudioManifest(root, "2026-08-17T00:00:00.000Z");

    expect(manifest.fileCount).toBe(2);
    expect(manifest.formatCounts).toEqual({ mp3: 1, flac: 1 });
    expect(manifest.files.map((file) => file.relativePath)).toEqual(["track.mp3", "Жанр/TRACK.FLAC"]);
    expect(manifest.files[0].sha256).toBe(manifest.files[1].sha256);
  });

  it("reports added, missing and content-changed audio", () => {
    const root = temporaryRoot();
    writeFileSync(join(root, "keep.mp3"), "before");
    writeFileSync(join(root, "missing.flac"), "missing");
    const expected = buildAudioManifest(root, "2026-08-17T00:00:00.000Z");

    rmSync(join(root, "missing.flac"));
    writeFileSync(join(root, "keep.mp3"), "after");
    writeFileSync(join(root, "added.flac"), "added");
    const actual = buildAudioManifest(root, "2026-08-17T01:00:00.000Z");

    expect(compareAudioManifests(expected, actual)).toEqual({
      unchanged: false,
      expectedFiles: 2,
      actualFiles: 2,
      missing: ["missing.flac"],
      added: ["added.flac"],
      changed: ["keep.mp3"],
    });
  });

  it("requires one explicit non-overwriting mode", () => {
    expect(parseManifestArguments(["--root", "music", "--output", "before.json"])).toEqual({ root: "music", output: "before.json", compare: null });
    expect(() => parseManifestArguments(["--root", "music"])).toThrow("exactly one");
    expect(() => parseManifestArguments(["--root", "music", "--output", "a", "--compare", "b"])).toThrow("exactly one");
  });
});
