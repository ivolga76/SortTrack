import type { Track } from "../../../domain/models";

export function playableQueue(tracks: Track[]): Track[] {
  return tracks.filter((track) => track.analysisState !== "missing");
}

export function currentQueueIndex(tracks: Track[], currentTrackId: string | null): number {
  return currentTrackId ? tracks.findIndex((track) => track.id === currentTrackId) : -1;
}
