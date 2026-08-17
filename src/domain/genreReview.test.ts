import { describe, expect, it } from "vitest";
import type { GenreSuggestion } from "./models";
import { genreReviewProgress, remainingGenreSuggestions } from "./genreReview";

const suggestion = (trackId: string): GenreSuggestion => ({
  trackId,
  title: trackId,
  artist: "",
  confirmedGenre: null,
  confirmedSubgenre: null,
  locked: false,
  inferenceVersion: "test",
  taxonomyVersion: 1,
  candidates: [],
});

describe("genre review queue", () => {
  it("removes skipped tracks only from the current local queue", () => {
    const source = [suggestion("one"), suggestion("two")];
    expect(remainingGenreSuggestions(source, new Set(["one"])).map((item) => item.trackId)).toEqual(["two"]);
    expect(source).toHaveLength(2);
  });

  it("keeps progress within zero and one hundred percent", () => {
    expect(genreReviewProgress(10, 7)).toBe(30);
    expect(genreReviewProgress(10, -1)).toBe(100);
    expect(genreReviewProgress(0, 0)).toBe(100);
  });
});
