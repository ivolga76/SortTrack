import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { LibrarySummary, LibraryTableLayout, Track, TrackQuery } from "../../../domain/models";
import type { LibraryBrowsePort } from "../../../shared/api/ports";
import { activeAdvancedFilterCount, buildTrackQuery, DEFAULT_LIBRARY_QUERY_STATE, DEFAULT_LIBRARY_TABLE_LAYOUT, requiresDefaultLibraryView, TRACK_PAGE_SIZE } from "./libraryQuery";

const EMPTY_SUMMARY: LibrarySummary = { tracks: 0, incoming: 0, needsReview: 0, duplicates: 0 };

export function useLibraryBrowser(gateway: LibraryBrowsePort, reportError: (message: string) => void) {
  const [summary, setSummary] = useState(EMPTY_SUMMARY);
  const [tracks, setTracks] = useState<Track[]>([]);
  const [trackTotal, setTrackTotal] = useState(0);
  const [trackOffset, setTrackOffset] = useState(0);
  const [query, setQuery] = useState(DEFAULT_LIBRARY_QUERY_STATE.text);
  const [genre, setGenre] = useState<string | null>(DEFAULT_LIBRARY_QUERY_STATE.genre);
  const [incomingOnly, setIncomingOnly] = useState(DEFAULT_LIBRARY_QUERY_STATE.onlyIncoming);
  const [trackSort, setTrackSort] = useState<TrackQuery["sortBy"]>(DEFAULT_LIBRARY_QUERY_STATE.sortBy);
  const [trackSortDescending, setTrackSortDescending] = useState(DEFAULT_LIBRARY_QUERY_STATE.descending);
  const [advancedFiltersOpen, setAdvancedFiltersOpen] = useState(false);
  const [filterMinBpm, setFilterMinBpm] = useState("");
  const [filterMaxBpm, setFilterMaxBpm] = useState("");
  const [filterMinRating, setFilterMinRating] = useState("");
  const [filterCamelot, setFilterCamelot] = useState("");
  const [filterFormat, setFilterFormat] = useState<"" | "mp3" | "flac">("");
  const [filterFileStatus, setFilterFileStatus] = useState<TrackQuery["fileStatus"]>("all");
  const [libraryTableLayout, setLibraryTableLayout] = useState<LibraryTableLayout>(() => structuredClone(DEFAULT_LIBRARY_TABLE_LAYOUT));
  const [libraryTableLayoutLoaded, setLibraryTableLayoutLoaded] = useState(false);
  const [checkedTrackIds, setCheckedTrackIds] = useState<Set<string>>(new Set());
  const revealOffsetRef = useRef<number | null>(null);
  const queryRequestRef = useRef(0);
  const reportErrorRef = useRef(reportError);

  useEffect(() => { reportErrorRef.current = reportError; }, [reportError]);

  const state = useMemo(() => ({
    text: query,
    genre,
    onlyIncoming: incomingOnly,
    minBpm: filterMinBpm,
    maxBpm: filterMaxBpm,
    minRating: filterMinRating,
    camelot: filterCamelot,
    format: filterFormat,
    fileStatus: filterFileStatus,
    sortBy: trackSort,
    descending: trackSortDescending,
  }), [query, genre, incomingOnly, filterMinBpm, filterMaxBpm, filterMinRating, filterCamelot, filterFormat, filterFileStatus, trackSort, trackSortDescending]);

  const advancedFilterCount = useMemo(() => activeAdvancedFilterCount(state), [state]);
  const trackQuery = useCallback((offset: number) => buildTrackQuery(state, offset), [state]);

  const reloadLibrary = useCallback(async () => {
    const [nextSummary, nextPage] = await Promise.all([
      gateway.getSummary(),
      gateway.searchTracks(trackQuery(trackOffset)),
    ]);
    setSummary(nextSummary);
    setTracks(nextPage.items);
    setTrackTotal(nextPage.total);
  }, [gateway, trackOffset, trackQuery]);

  const reloadSummary = useCallback(async () => setSummary(await gateway.getSummary()), [gateway]);

  const resetAdvancedFilters = useCallback(() => {
    setFilterMinBpm("");
    setFilterMaxBpm("");
    setFilterMinRating("");
    setFilterCamelot("");
    setFilterFormat("");
    setFilterFileStatus("all");
  }, []);

  const resetForReveal = useCallback((offset: number) => {
    revealOffsetRef.current = offset;
    setQuery("");
    setGenre(null);
    setIncomingOnly(false);
    resetAdvancedFilters();
    setTrackSort("added");
    setTrackSortDescending(true);
  }, [resetAdvancedFilters]);

  const revealTrack = useCallback(async (trackId: string): Promise<boolean> => {
    try {
      const offset = await gateway.locateTrackPage(trackId, TRACK_PAGE_SIZE);
      if (offset === null) {
        reportErrorRef.current("Текущий трек больше не найден в библиотеке.");
        return false;
      }
      if (requiresDefaultLibraryView(state)) resetForReveal(offset);
      else setTrackOffset(offset);
      return true;
    } catch (error) {
      reportErrorRef.current(`Не удалось показать текущий трек: ${String(error)}`);
      return false;
    }
  }, [gateway, resetForReveal, state]);

  useEffect(() => {
    void gateway.getSummary().then(setSummary).catch((error) => reportErrorRef.current(`Не удалось загрузить сводку библиотеки: ${String(error)}`));
    void gateway.getLibraryTableLayout()
      .then((layout) => { setLibraryTableLayout(layout); setLibraryTableLayoutLoaded(true); })
      .catch((error) => reportErrorRef.current(`Не удалось загрузить раскладку таблицы: ${String(error)}`));
  }, [gateway]);

  useEffect(() => {
    if (!libraryTableLayoutLoaded) return;
    const timer = window.setTimeout(() => {
      void gateway.saveLibraryTableLayout(libraryTableLayout).catch((error) => reportErrorRef.current(`Не удалось сохранить раскладку таблицы: ${String(error)}`));
    }, 250);
    return () => window.clearTimeout(timer);
  }, [gateway, libraryTableLayout, libraryTableLayoutLoaded]);

  useEffect(() => {
    if (revealOffsetRef.current !== null) {
      setTrackOffset(revealOffsetRef.current);
      revealOffsetRef.current = null;
    } else {
      setTrackOffset(0);
    }
  }, [state]);

  useEffect(() => {
    const requestId = queryRequestRef.current + 1;
    queryRequestRef.current = requestId;
    void gateway.searchTracks(trackQuery(trackOffset)).then((result) => {
      if (queryRequestRef.current === requestId) {
        setTracks(result.items);
        setTrackTotal(result.total);
      }
    }).catch((error) => {
      if (queryRequestRef.current === requestId) reportErrorRef.current(`Фильтр библиотеки отклонён: ${String(error)}`);
    });
  }, [gateway, trackOffset, trackQuery]);

  useEffect(() => {
    const visible = new Set(tracks.map((track) => track.id));
    setCheckedTrackIds((current) => new Set([...current].filter((id) => visible.has(id))));
  }, [tracks]);

  return {
    summary, setSummary,
    tracks, trackTotal,
    trackOffset, setTrackOffset,
    query, setQuery,
    genre, setGenre,
    incomingOnly, setIncomingOnly,
    trackSort, setTrackSort,
    trackSortDescending, setTrackSortDescending,
    advancedFiltersOpen, setAdvancedFiltersOpen,
    filterMinBpm, setFilterMinBpm,
    filterMaxBpm, setFilterMaxBpm,
    filterMinRating, setFilterMinRating,
    filterCamelot, setFilterCamelot,
    filterFormat, setFilterFormat,
    filterFileStatus, setFilterFileStatus,
    libraryTableLayout, setLibraryTableLayout,
    checkedTrackIds, setCheckedTrackIds,
    activeAdvancedFilterCount: advancedFilterCount,
    resetAdvancedFilters,
    reloadLibrary,
    reloadSummary,
    revealTrack,
    resetForReveal,
    requiresRevealReset: requiresDefaultLibraryView(state),
    pageSize: TRACK_PAGE_SIZE,
  };
}
