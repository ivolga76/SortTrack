import { describe, expect, it } from "vitest";
import { decodePlaylistDrag, encodePlaylistDrag, placementFromDrop } from "./playlistDrag";

describe("playlist drag payload", () => {
  it("round-trips a playlist item without trusting malformed input", () => {
    const payload = { trackId: "track-1", itemId: "item-1", sourcePlaylistId: "list-1" };
    expect(decodePlaylistDrag(encodePlaylistDrag(payload))).toEqual(payload);
    expect(decodePlaylistDrag("not json")).toBeNull();
    expect(decodePlaylistDrag('{"trackId":42}')).toBeNull();
  });

  it("adds the drop target without changing the source", () => {
    expect(placementFromDrop({ trackId: "track-1" }, "target", 3)).toEqual({
      trackId: "track-1",
      targetPlaylistId: "target",
      targetPosition: 3,
    });
  });
});
