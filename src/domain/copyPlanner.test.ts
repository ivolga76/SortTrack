import { describe, expect, it } from "vitest";
import { isSafeWindowsFilename, planCopies } from "./copyPlanner";

describe("copy planner", () => {
  it("never silently overwrites a destination", () => {
    const [item] = planCopies(
      [{ trackId: "1", sourcePath: "D:\\in\\track.flac", filename: "track.flac", size: 10 }],
      "D:\\out",
      [{ path: "d:\\OUT\\track.flac", size: 20 }],
    );
    expect(item.decision).toBe("resolve-conflict");
  });

  it("skips a confirmed identical file", () => {
    const [item] = planCopies(
      [
        {
          trackId: "1",
          sourcePath: "D:\\in\\track.flac",
          filename: "track.flac",
          size: 10,
          sha256: "abc",
        },
      ],
      "D:\\out",
      [{ path: "D:\\out\\track.flac", size: 10, sha256: "abc" }],
    );
    expect(item.decision).toBe("skip-identical");
  });

  it.each(["CON.mp3", "bad:name.flac", "track. "])("rejects unsafe name %s", (filename) => {
    expect(isSafeWindowsFilename(filename)).toBe(false);
  });
});
