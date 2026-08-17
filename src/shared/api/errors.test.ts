import { describe, expect, it } from "vitest";
import { GatewayError, classifyGatewayError, normalizeGatewayError } from "./errors";

describe("gateway errors", () => {
  it("classifies stable state, input and infrastructure failures", () => {
    expect(classifyGatewayError("Плейлист изменился; откройте снова")).toBe("STALE_STATE");
    expect(classifyGatewayError("Рейтинг должен быть от 0 до 5")).toBe("INVALID_INPUT");
    expect(classifyGatewayError("filesystem error: access denied")).toBe("FILESYSTEM");
    expect(classifyGatewayError("database error: busy")).toBe("DATABASE");
  });

  it("preserves the user-facing text while attaching operation context", () => {
    const error = normalizeGatewayError("track is unavailable for playback", "prepare_track_playback");
    expect(error).toBeInstanceOf(GatewayError);
    expect(error).toMatchObject({ code: "UNAVAILABLE", operation: "prepare_track_playback", retryable: true });
    expect(String(error)).toBe("track is unavailable for playback");
  });

  it("does not wrap an already normalized error twice", () => {
    const original = new GatewayError("NOT_FOUND", "missing", "search_tracks");
    expect(normalizeGatewayError(original, "other")).toBe(original);
  });

  it("accepts the versioned structured Rust error without exposing JSON to the UI", () => {
    const error = normalizeGatewayError({ schema: "sorttrack-error-1", code: "STALE_STATE", message: "playlist changed", retryable: false }, "remove_playlist");
    expect(error).toMatchObject({ code: "STALE_STATE", message: "playlist changed", operation: "remove_playlist" });
    expect(String(error)).toBe("playlist changed");
  });
});
