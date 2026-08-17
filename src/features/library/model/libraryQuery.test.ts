import { describe, expect, it } from "vitest";
import { activeAdvancedFilterCount, buildTrackQuery, CAMELOT_KEYS, DEFAULT_LIBRARY_QUERY_STATE, requiresDefaultLibraryView, TRACK_PAGE_SIZE } from "./libraryQuery";

describe("library query model", () => {
  it("maps UI strings to the stable backend query contract", () => {
    expect(buildTrackQuery({
      ...DEFAULT_LIBRARY_QUERY_STATE,
      text: "acid",
      genre: "Techno",
      onlyIncoming: false,
      minBpm: "130",
      maxBpm: "150.5",
      minRating: "4",
      camelot: "4A",
      format: "flac",
      fileStatus: "available",
      sortBy: "artist",
      descending: false,
    }, 200)).toEqual({
      text: "acid",
      genre: "Techno",
      onlyIncoming: false,
      minBpm: 130,
      maxBpm: 150.5,
      minRating: 4,
      camelot: "4A",
      format: "flac",
      fileStatus: "available",
      limit: TRACK_PAGE_SIZE,
      offset: 200,
      sortBy: "artist",
      descending: false,
    });
  });

  it("keeps empty numeric filters null instead of coercing them to zero", () => {
    expect(buildTrackQuery({ ...DEFAULT_LIBRARY_QUERY_STATE, minBpm: "  " }, 0).minBpm).toBeNull();
  });

  it("counts only advanced filters and recognizes reveal reset requirements", () => {
    const state = { ...DEFAULT_LIBRARY_QUERY_STATE, onlyIncoming: false, minBpm: "120", camelot: "8A", fileStatus: "missing" as const };
    expect(activeAdvancedFilterCount(state)).toBe(3);
    expect(requiresDefaultLibraryView(state)).toBe(true);
    expect(requiresDefaultLibraryView({ ...DEFAULT_LIBRARY_QUERY_STATE, onlyIncoming: false })).toBe(false);
  });

  it("publishes every canonical Camelot key once", () => {
    expect(CAMELOT_KEYS).toHaveLength(24);
    expect(new Set(CAMELOT_KEYS).size).toBe(24);
    expect(CAMELOT_KEYS).toEqual(expect.arrayContaining(["1A", "1B", "12A", "12B"]));
  });
});
