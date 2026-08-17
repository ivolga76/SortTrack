import { describe, expect, it } from "vitest";
import type { Track } from "../../../domain/models";
import { currentQueueIndex, playableQueue } from "./playerQueue";

function track(id: string, analysisState: Track["analysisState"]): Track {
  return { id, analysisState } as Track;
}

describe("player feature queue", () => {
  it("keeps the visible order while excluding missing files", () => {
    expect(playableQueue([track("a", "ready"), track("missing", "missing"), track("b", "queued")]).map((item) => item.id)).toEqual(["a", "b"]);
  });

  it("uses -1 when no visible current track exists", () => {
    const queue = [track("a", "ready")];
    expect(currentQueueIndex(queue, "a")).toBe(0);
    expect(currentQueueIndex(queue, "other")).toBe(-1);
    expect(currentQueueIndex(queue, null)).toBe(-1);
  });
});
