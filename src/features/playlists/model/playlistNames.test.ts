import { describe, expect, it } from "vitest";
import { nextManualPlaylistName, smartPlaylistSnapshotName } from "./playlistNames";

describe("playlist naming rules", () => {
  it("selects a free deterministic name after the current playlist count", () => {
    expect(nextManualPlaylistName([{ name: "Личный" }, { name: "Новый плейлист 3" }])).toBe("Новый плейлист 4");
  });

  it("keeps a snapshot name within the 100-character contract", () => {
    const name = smartPlaylistSnapshotName("Т".repeat(120));
    expect(Array.from(name)).toHaveLength(100);
    expect(name.endsWith(" — снимок")).toBe(true);
  });
});
