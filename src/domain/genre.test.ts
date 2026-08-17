import { describe, expect, it } from "vitest";
import { genreLabel, normalizeGenre } from "./genre";

describe("normalizeGenre", () => {
  it.each(["DnB", "D&B", "Drum'n'Bass", "drum and bass"])(
    "normalizes %s",
    (value) => {
      expect(normalizeGenre(value)).toMatchObject({
        genre: "Drum & Bass",
        subgenre: null,
      });
    },
  );

  it("maps a techno subgenre to its parent", () => {
    expect(normalizeGenre("pictime techno")).toEqual({
      original: "pictime techno",
      genre: "Techno",
      subgenre: "Peak Time Techno",
    });
  });

  it("preserves an unknown source value", () => {
    expect(normalizeGenre("Future Rave")).toEqual({
      original: "Future Rave",
      genre: null,
      subgenre: null,
    });
  });
});

describe("genreLabel", () => {
  it("renders a hierarchy", () => {
    expect(genreLabel("Techno", "Acid Techno")).toBe("Techno → Acid Techno");
  });
});
