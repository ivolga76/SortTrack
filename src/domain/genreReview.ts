import type { GenreSuggestion } from "./models";

export function remainingGenreSuggestions(suggestions: GenreSuggestion[], skippedTrackIds: ReadonlySet<string>): GenreSuggestion[] {
  return suggestions.filter((item) => !skippedTrackIds.has(item.trackId));
}

export function genreReviewProgress(initialTotal: number, remaining: number): number {
  if (initialTotal <= 0) return 100;
  return Math.min(100, Math.max(0, (initialTotal - remaining) / initialTotal * 100));
}
