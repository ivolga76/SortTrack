import { describe, expect, it } from "vitest";
import { adjacentTrackIndex, downsamplePeaks, formatPlaybackTime, queuedTrackIndex, waveformPointerRatio, waveformViewport } from "./player";

describe("player utilities", () => {
  it("formats duration without leaking invalid values", () => {
    expect(formatPlaybackTime(65.9)).toBe("1:05");
    expect(formatPlaybackTime(Number.NaN)).toBe("0:00");
  });

  it("does not wrap the queue unexpectedly", () => {
    expect(adjacentTrackIndex(["a", "b"], "a", 1)).toBe(1);
    expect(adjacentTrackIndex(["a", "b"], "b", 1)).toBeNull();
    expect(adjacentTrackIndex(["a", "b"], "a", -1)).toBeNull();
  });

  it("builds bounded waveform peaks", () => {
    const peaks = downsamplePeaks(new Float32Array([0, -0.5, 1, 0.25]), 2);
    expect(peaks).toEqual([0.5, 1]);
  });

  it("wraps only when repeat-all is enabled", () => {
    expect(queuedTrackIndex(["a", "b"], "b", 1, { wrap: false, shuffle: false })).toBeNull();
    expect(queuedTrackIndex(["a", "b"], "b", 1, { wrap: true, shuffle: false })).toBe(0);
    expect(queuedTrackIndex(["a", "b"], "a", -1, { wrap: true, shuffle: false })).toBe(1);
  });

  it("shuffle never selects the current track", () => {
    expect(queuedTrackIndex(["a", "b", "c"], "b", 1, { wrap: false, shuffle: true }, 0)).toBe(0);
    expect(queuedTrackIndex(["a", "b", "c"], "b", 1, { wrap: false, shuffle: true }, 0.99)).toBe(2);
  });

  it("creates a clamped zoomed waveform viewport", () => {
    const peaks = Array.from({ length: 400 }, (_, index) => index);
    const middle = waveformViewport(peaks, 0.5, 4);
    expect(middle.peaks).toHaveLength(100);
    expect(middle.startIndex).toBe(150);
    expect(middle.startRatio).toBe(0.375);
    expect(middle.endRatio).toBe(0.625);
    expect(waveformViewport(peaks, 0, 4).startIndex).toBe(0);
    expect(waveformViewport(peaks, 1, 4).startIndex).toBe(300);
  });

  it("maps a pointer inside the visible waveform range", () => {
    expect(waveformPointerRatio(0.5, 0.25, 0.75)).toBe(0.5);
    expect(waveformPointerRatio(-1, 0.25, 0.75)).toBe(0.25);
    expect(waveformPointerRatio(2, 0.25, 0.75)).toBe(0.75);
  });
});
