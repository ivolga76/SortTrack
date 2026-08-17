import type { PlaylistPlacement } from "./models";

export const PLAYLIST_DRAG_TYPE = "application/x-sorttrack-playlist-item";

export interface PlaylistDragPayload {
  trackId: string;
  itemId?: string;
  sourcePlaylistId?: string;
}

export function encodePlaylistDrag(payload: PlaylistDragPayload): string {
  return JSON.stringify(payload);
}

export function decodePlaylistDrag(value: string): PlaylistDragPayload | null {
  try {
    const candidate = JSON.parse(value) as Partial<PlaylistDragPayload>;
    if (typeof candidate.trackId !== "string" || candidate.trackId.length === 0) return null;
    if (candidate.itemId !== undefined && typeof candidate.itemId !== "string") return null;
    if (candidate.sourcePlaylistId !== undefined && typeof candidate.sourcePlaylistId !== "string") return null;
    return candidate as PlaylistDragPayload;
  } catch {
    return null;
  }
}

export function placementFromDrop(
  payload: PlaylistDragPayload,
  targetPlaylistId: string,
  targetPosition: number,
): PlaylistPlacement {
  return { ...payload, targetPlaylistId, targetPosition };
}
