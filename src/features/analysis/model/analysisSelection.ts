import type { Track } from "../../../domain/models";

export function analysisTrackIds(tracks: Track[], checkedIds: ReadonlySet<string>, selected: Track | null): string[] {
  if (checkedIds.size) return [...checkedIds];
  if (selected) return [selected.id];
  return tracks.filter((track) => track.bpm === null || track.musicalKey === null).map((track) => track.id);
}

export function genreTrackIds(tracks: Track[], checkedIds: ReadonlySet<string>, selected: Track | null): string[] {
  if (checkedIds.size) return [...checkedIds];
  if (selected) return [selected.id];
  return tracks.filter((track) => track.genre === null).map((track) => track.id);
}
