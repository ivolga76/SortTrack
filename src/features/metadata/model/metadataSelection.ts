import type { Track } from "../../../domain/models";

export type MetadataSelection =
  | { mode: "none" }
  | { mode: "single"; track: Track }
  | { mode: "bulk"; trackIds: string[] };

export function resolveMetadataSelection(checkedIds: ReadonlySet<string>, tracks: Track[], selected: Track | null): MetadataSelection {
  const trackIds = [...checkedIds];
  if (trackIds.length > 1) return { mode: "bulk", trackIds };
  if (trackIds.length === 1) {
    const track = tracks.find((candidate) => candidate.id === trackIds[0]);
    return track ? { mode: "single", track } : { mode: "none" };
  }
  return selected ? { mode: "single", track: selected } : { mode: "none" };
}
