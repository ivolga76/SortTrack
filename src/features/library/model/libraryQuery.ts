import type { LibraryTableLayout, TrackQuery } from "../../../domain/models";

export const TRACK_PAGE_SIZE = 100;
export const CAMELOT_KEYS = Array.from({ length: 12 }, (_, index) => [`${index + 1}A`, `${index + 1}B`]).flat();

export const DEFAULT_LIBRARY_TABLE_LAYOUT: LibraryTableLayout = { columns: [
  { id: "track", visible: true, width: 320 },
  { id: "genre", visible: true, width: 170 },
  { id: "bpm", visible: true, width: 72 },
  { id: "key", visible: true, width: 130 },
  { id: "format", visible: true, width: 82 },
  { id: "duration", visible: true, width: 76 },
] };

export interface LibraryQueryState {
  text: string;
  genre: string | null;
  onlyIncoming: boolean;
  minBpm: string;
  maxBpm: string;
  minRating: string;
  camelot: string;
  format: "" | "mp3" | "flac";
  fileStatus: TrackQuery["fileStatus"];
  sortBy: TrackQuery["sortBy"];
  descending: boolean;
}

export const DEFAULT_LIBRARY_QUERY_STATE: LibraryQueryState = {
  text: "",
  genre: null,
  onlyIncoming: true,
  minBpm: "",
  maxBpm: "",
  minRating: "",
  camelot: "",
  format: "",
  fileStatus: "all",
  sortBy: "added",
  descending: true,
};

function optionalFilterNumber(value: string): number | null {
  return value.trim() === "" ? null : Number(value);
}

export function buildTrackQuery(state: LibraryQueryState, offset: number): TrackQuery {
  return {
    text: state.text,
    genre: state.genre,
    onlyIncoming: state.onlyIncoming,
    minBpm: optionalFilterNumber(state.minBpm),
    maxBpm: optionalFilterNumber(state.maxBpm),
    minRating: optionalFilterNumber(state.minRating),
    camelot: state.camelot || null,
    format: state.format || null,
    fileStatus: state.fileStatus,
    limit: TRACK_PAGE_SIZE,
    offset,
    sortBy: state.sortBy,
    descending: state.descending,
  };
}

export function activeAdvancedFilterCount(state: LibraryQueryState): number {
  return [
    state.minBpm,
    state.maxBpm,
    state.minRating,
    state.camelot,
    state.format,
    state.fileStatus === "all" ? "" : state.fileStatus,
  ].filter(Boolean).length;
}

export function requiresDefaultLibraryView(state: LibraryQueryState): boolean {
  return state.text !== ""
    || state.genre !== null
    || state.onlyIncoming
    || activeAdvancedFilterCount(state) > 0
    || state.sortBy !== "added"
    || !state.descending;
}
