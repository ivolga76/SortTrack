import { useMemo, useState } from "react";
import { AppContent, AppShell } from "./AppShell";
import { NoticeBanner } from "./NoticeBanner";
import { Sidebar } from "./Sidebar";
import type { Track } from "../domain/models";
import { LibraryFilters, LibraryHeading, LibraryPagination, LibrarySelectionActions, LibraryTable, LibraryTopbar, useLibraryBrowser, useTrackRatings } from "../features/library";
import { PlayerDock, usePlayerController } from "../features/player";
import { PlaylistDialogs, PlaylistDock, usePlaylistManager } from "../features/playlists";
import { DuplicatesView, useDuplicates } from "../features/duplicates";
import { LibraryScanDialogs, useLibraryScan } from "../features/library-scan";
import { MetadataDialogs, useMetadataManager } from "../features/metadata";
import { AnalysisDialogs, useAnalysisManager } from "../features/analysis";
import { SettingsFeature, useSettingsManager } from "../features/settings";
import { appGateway } from "../adapters/bootstrap";
import { GatewayProvider } from "../shared/api/GatewayProvider";

const gateway = appGateway;
const resolvePlayback = (trackId: string) => gateway.preparePlayback(trackId);
const resolveWaveform = async (trackId: string) => (await gateway.getTrackWaveform(trackId))?.peaks ?? null;
export function App() {
  const [notice, setNotice] = useState<string | null>(null);
  const {
    summary,
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
    activeAdvancedFilterCount,
    resetAdvancedFilters,
    reloadLibrary,
    reloadSummary,
    revealTrack,
    pageSize: trackPageSize,
  } = useLibraryBrowser(gateway, setNotice);
  const libraryScan = useLibraryScan(gateway, setNotice, reloadLibrary);
  const {
    currentTrack, setCurrentTrack,
    playing, setPlaying,
    shuffle, setShuffle,
    repeatMode, setRepeatMode,
    currentIndex,
    queueLength,
    canPrevious,
    canNext,
    playTrack,
    playAdjacent,
  } = usePlayerController(tracks);
  const playlistManager = usePlaylistManager(gateway, setNotice);
  const { playlists, reloadPlaylists } = playlistManager;
  const duplicates = useDuplicates(gateway, setNotice, reloadSummary);
  const [selected, setSelected] = useState<Track | null>(null);
  const [activeView, setActiveView] = useState<"inbox" | "duplicates">("inbox");
  const ratings = useTrackRatings(gateway, {
    reportNotice: setNotice,
    reloadLibrary,
    reloadPlaylists,
    updateVisibleTrack: (trackId, rating) => {
      setSelected((track) => track?.id === trackId ? { ...track, rating } : track);
      setCurrentTrack((track) => track?.id === trackId ? { ...track, rating } : track);
    },
  });
  const metadataManager = useMetadataManager(gateway, {
    reportNotice: setNotice,
    reloadLibrary,
    reloadPlaylists,
    selectTrack: setSelected,
    clearCheckedTracks: () => setCheckedTrackIds(new Set()),
    onRelinkCommitted: (trackId) => {
      setCurrentTrack((track) => track?.id === trackId ? null : track);
      setPlaying(false);
    },
  });
  const analysisManager = useAnalysisManager(gateway, { tracks, checkedIds: checkedTrackIds, selected }, {
    reportNotice: setNotice,
    reloadLibrary,
    uncheckTrack: (trackId) => setCheckedTrackIds((current) => {
      const next = new Set(current);
      next.delete(trackId);
      return next;
    }),
  });
  const settingsManager = useSettingsManager(gateway, {
    reportNotice: setNotice,
    refreshLibraryRoots: libraryScan.refreshRootAvailability,
    cancelLibraryRootRemoval: libraryScan.cancelRootRemoval,
  });

  const reviewCount = useMemo(() => tracks.filter((track) => track.analysisState === "needs-review").length, [tracks]);
  const revealCurrentTrack = async () => {
    if (!currentTrack) return;
    if (await revealTrack(currentTrack.id)) {
      setActiveView("inbox");
      setSelected(currentTrack);
    }
  };
  const openDuplicates = () => {
    setActiveView("duplicates");
    playlistManager.closeWorkspace();
    void duplicates.load();
  };
  const checkTrack = (trackId: string, checked: boolean) => {
    setCheckedTrackIds((current) => { const next = new Set(current); checked ? next.add(trackId) : next.delete(trackId); return next; });
  };
  return (
    <GatewayProvider gateway={gateway}>
    <AppShell>
      <Sidebar summary={summary} playlists={playlists} activeView={activeView} onOpenInbox={() => { setActiveView("inbox"); setIncomingOnly(true); }} onOpenDuplicates={openDuplicates} onOpenPlaylists={() => { setActiveView("inbox"); playlistManager.openWorkspace(); }} onOpenSettings={settingsManager.show} />
      <AppContent>
        <LibraryTopbar query={query} scanning={libraryScan.scanning} onQueryChange={setQuery} onAddFolder={() => void libraryScan.addFolder()}/>
        {notice && <NoticeBanner message={notice} onClose={() => setNotice(null)}/>} 
        {activeView === "duplicates" ? <DuplicatesView groups={duplicates.groups} lastScan={duplicates.lastScan} busy={duplicates.busy} onScan={() => void duplicates.scan()} /> : <>
        <LibraryHeading incomingOnly={incomingOnly} total={trackTotal} reviewCount={reviewCount || summary.needsReview} genreBusy={analysisManager.genreBusy} onReviewGenres={() => void analysisManager.openGenreAssistant()}/>
        <LibraryFilters
          genre={genre}
          advancedFiltersOpen={advancedFiltersOpen}
          activeAdvancedFilterCount={activeAdvancedFilterCount}
          minBpm={filterMinBpm}
          maxBpm={filterMaxBpm}
          minRating={filterMinRating}
          camelot={filterCamelot}
          format={filterFormat}
          fileStatus={filterFileStatus}
          selectionActions={<LibrarySelectionActions
            checkedCount={checkedTrackIds.size}
            visibleCount={tracks.length}
            total={trackTotal}
            sortBy={trackSort}
            descending={trackSortDescending}
            ratingTarget={ratings.bulkTarget}
            ratingBusy={ratings.bulkBusy}
            datasetBusy={analysisManager.datasetBusy}
            genreBusy={analysisManager.genreBusy}
            metadataLoading={metadataManager.loading}
            playlistCount={playlists.length}
            onSortByChange={setTrackSort}
            onDescendingChange={setTrackSortDescending}
            onRatingTargetChange={ratings.setBulkTarget}
            onApplyRating={() => void ratings.applyBulkRating(checkedTrackIds)}
            onOpenDataset={() => void analysisManager.openDatasetAudit()}
            onOpenGenres={() => void analysisManager.openGenreAssistant()}
            onOpenAnalysis={analysisManager.openQueue}
            onOpenMetadata={() => metadataManager.openSelection(checkedTrackIds, tracks, selected)}
            onOpenPlaylists={playlistManager.openWorkspace}
          />}
          onGenreChange={(nextGenre) => { setGenre(nextGenre); if (nextGenre) setIncomingOnly(false); }}
          onAdvancedFiltersOpenChange={setAdvancedFiltersOpen}
          onMinBpmChange={setFilterMinBpm}
          onMaxBpmChange={setFilterMaxBpm}
          onMinRatingChange={setFilterMinRating}
          onCamelotChange={setFilterCamelot}
          onFormatChange={setFilterFormat}
          onFileStatusChange={setFilterFileStatus}
          onReset={resetAdvancedFilters}
        />
        <div className={`library-workspace ${playlistManager.workspaceOpen ? "with-playlists" : ""}`}>
          <div className="library-pane">
            <LibraryTable tracks={tracks} layout={libraryTableLayout} selectedId={selected?.id ?? null} checkedIds={checkedTrackIds} playingId={currentTrack?.id ?? null} ratingBusyId={ratings.busyTrackId} onLayoutChange={setLibraryTableLayout} onSelect={setSelected} onCheck={checkTrack} onCheckAll={(checked) => setCheckedTrackIds(checked ? new Set(tracks.map((track) => track.id)) : new Set())} onPlay={(track) => { setSelected(track); playTrack(track); }} onEdit={(track) => void metadataManager.openTrack(track)} onRelink={(track) => void metadataManager.openRelink(track)} onRating={(track, rating) => void ratings.setRating(track, rating)} />
            <LibraryPagination incomingOnly={incomingOnly} offset={trackOffset} total={trackTotal} pageSize={trackPageSize} onIncomingOnlyChange={setIncomingOnly} onOffsetChange={setTrackOffset}/>
          </div>
          {playlistManager.workspaceOpen && <PlaylistDock manager={playlistManager} selectedTrack={selected} onPlay={(track) => { setSelected(track); playTrack(track); }}/>} 
        </div>
        </>}
      </AppContent>
      <PlayerDock
        track={currentTrack}
        playing={playing}
        canPrevious={canPrevious}
        canNext={canNext}
        onPlayingChange={setPlaying}
        onPrevious={() => { playAdjacent(-1, setSelected); }}
        onNext={() => { playAdjacent(1, setSelected); }}
        resolveSource={resolvePlayback}
        resolveWaveform={resolveWaveform}
        shuffle={shuffle}
        repeatMode={repeatMode}
        queuePosition={currentIndex + 1}
        queueLength={queueLength}
        onShuffleChange={setShuffle}
        onRepeatModeChange={setRepeatMode}
        onRevealCurrent={() => void revealCurrentTrack()}
      />
      <PlaylistDialogs manager={playlistManager}/>
      <MetadataDialogs manager={metadataManager}/>
      <AnalysisDialogs manager={analysisManager} desktop={gateway.isDesktop}/>
      <SettingsFeature manager={settingsManager} libraryScan={libraryScan} desktop={gateway.isDesktop}/>
      <LibraryScanDialogs controller={libraryScan}/>
    </AppShell>
    </GatewayProvider>
  );
}
