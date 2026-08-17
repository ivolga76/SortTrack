export function formatPlaybackTime(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) return "0:00";
  const whole = Math.floor(seconds);
  return `${Math.floor(whole / 60)}:${String(whole % 60).padStart(2, "0")}`;
}

export function adjacentTrackIndex(
  trackIds: string[],
  currentId: string | null,
  direction: -1 | 1,
): number | null {
  if (trackIds.length === 0) return null;
  const current = currentId ? trackIds.indexOf(currentId) : -1;
  if (current < 0) return direction === 1 ? 0 : trackIds.length - 1;
  const next = current + direction;
  return next >= 0 && next < trackIds.length ? next : null;
}

export function queuedTrackIndex(
  trackIds: string[],
  currentId: string | null,
  direction: -1 | 1,
  options: { wrap: boolean; shuffle: boolean },
  randomValue = Math.random(),
): number | null {
  if (trackIds.length === 0) return null;
  const current = currentId ? trackIds.indexOf(currentId) : -1;
  if (options.shuffle && direction === 1 && trackIds.length > 1) {
    const candidates = trackIds.map((_, index) => index).filter((index) => index !== current);
    return candidates[Math.min(candidates.length - 1, Math.floor(Math.max(0, Math.min(0.999999, randomValue)) * candidates.length))];
  }
  if (current < 0) return direction === 1 ? 0 : trackIds.length - 1;
  const next = current + direction;
  if (next >= 0 && next < trackIds.length) return next;
  if (!options.wrap) return null;
  return direction === 1 ? 0 : trackIds.length - 1;
}

export function downsamplePeaks(samples: Float32Array, bins: number): number[] {
  if (bins <= 0) return [];
  if (samples.length === 0) return Array.from({ length: bins }, () => 0.08);
  const samplesPerBin = Math.max(1, Math.floor(samples.length / bins));
  return Array.from({ length: bins }, (_, index) => {
    const start = index * samplesPerBin;
    const end = index === bins - 1 ? samples.length : Math.min(samples.length, start + samplesPerBin);
    let peak = 0;
    for (let cursor = start; cursor < end; cursor += 1) {
      peak = Math.max(peak, Math.abs(samples[cursor] ?? 0));
    }
    return Math.max(0.08, Math.min(1, peak));
  });
}

export const DEFAULT_WAVEFORM = Array.from({ length: 96 }, (_, index) =>
  0.18 + ((index * 17) % 23) / 34,
);

export interface WaveformViewport {
  peaks: number[];
  startRatio: number;
  endRatio: number;
  startIndex: number;
}

export function waveformViewport(peaks: number[], progress: number, zoom: 1 | 2 | 4): WaveformViewport {
  if (peaks.length === 0) return { peaks: [], startRatio: 0, endRatio: 1, startIndex: 0 };
  const safeProgress = Math.max(0, Math.min(1, Number.isFinite(progress) ? progress : 0));
  const visibleCount = Math.max(1, Math.ceil(peaks.length / zoom));
  const center = Math.round(safeProgress * (peaks.length - 1));
  const startIndex = Math.max(0, Math.min(peaks.length - visibleCount, center - Math.floor(visibleCount / 2)));
  const endIndex = Math.min(peaks.length, startIndex + visibleCount);
  return {
    peaks: peaks.slice(startIndex, endIndex),
    startRatio: startIndex / peaks.length,
    endRatio: endIndex / peaks.length,
    startIndex,
  };
}

export function waveformPointerRatio(localRatio: number, startRatio: number, endRatio: number): number {
  const local = Math.max(0, Math.min(1, localRatio));
  return Math.max(0, Math.min(1, startRatio + local * (endRatio - startRatio)));
}
