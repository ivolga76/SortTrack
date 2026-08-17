import { describe, expect, it } from "vitest";
import type { Track } from "../../../domain/models";
import { analysisTrackIds, genreTrackIds } from "./analysisSelection";

const track = (id: string, values: Partial<Track> = {}) => ({ id, bpm: 120, musicalKey: "C", genre: "Techno", ...values } as Track);

describe("analysis selection", () => {
  it("prefers checked tracks and then the focused track", () => {
    const tracks = [track("a"), track("b")];
    expect(analysisTrackIds(tracks, new Set(["b"]), tracks[0])).toEqual(["b"]);
    expect(genreTrackIds(tracks, new Set(), tracks[0])).toEqual(["a"]);
  });

  it("falls back to incomplete audio analysis and missing genre queues", () => {
    const tracks = [track("ready"), track("bpm", { bpm: null }), track("key", { musicalKey: null }), track("genre", { genre: null })];
    expect(analysisTrackIds(tracks, new Set(), null)).toEqual(["bpm", "key"]);
    expect(genreTrackIds(tracks, new Set(), null)).toEqual(["genre"]);
  });
});
