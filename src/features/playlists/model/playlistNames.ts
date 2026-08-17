import type { Playlist } from "../../../domain/models";

export function nextManualPlaylistName(playlists: Pick<Playlist, "name">[]): string {
  const existing = new Set(playlists.map((playlist) => playlist.name));
  let index = playlists.length + 1;
  while (existing.has(`Новый плейлист ${index}`)) index += 1;
  return `Новый плейлист ${index}`;
}

export function smartPlaylistSnapshotName(name: string): string {
  const suffix = " — снимок";
  const maximumBaseLength = 100 - Array.from(suffix).length;
  return `${Array.from(name).slice(0, maximumBaseLength).join("")}${suffix}`;
}
