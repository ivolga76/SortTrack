import { useCallback, useMemo, useState } from "react";
import type { Track } from "../../../domain/models";
import { queuedTrackIndex } from "../../../domain/player";
import { currentQueueIndex, playableQueue } from "./playerQueue";

export type RepeatMode = "off" | "all" | "one";

export function usePlayerController(visibleTracks: Track[]) {
  const [currentTrack, setCurrentTrack] = useState<Track | null>(null);
  const [playing, setPlaying] = useState(false);
  const [shuffle, setShuffle] = useState(false);
  const [repeatMode, setRepeatMode] = useState<RepeatMode>("off");

  const playableTracks = useMemo(() => playableQueue(visibleTracks), [visibleTracks]);
  const playableIds = useMemo(() => playableTracks.map((track) => track.id), [playableTracks]);
  const currentIndex = currentQueueIndex(playableTracks, currentTrack?.id ?? null);

  const adjacentIndex = useCallback((direction: -1 | 1) => queuedTrackIndex(
    playableIds,
    currentTrack?.id ?? null,
    direction,
    { wrap: repeatMode === "all", shuffle: shuffle && direction === 1 },
  ), [currentTrack?.id, playableIds, repeatMode, shuffle]);

  const playTrack = useCallback((track: Track) => {
    if (track.analysisState === "missing") return;
    setCurrentTrack(track);
    setPlaying(true);
  }, []);

  const playAdjacent = useCallback((direction: -1 | 1, onTrackChange?: (track: Track) => void) => {
    const index = adjacentIndex(direction);
    if (index === null) {
      setPlaying(false);
      return null;
    }
    const track = playableTracks[index];
    setCurrentTrack(track);
    setPlaying(true);
    onTrackChange?.(track);
    return track;
  }, [adjacentIndex, playableTracks]);

  return {
    currentTrack,
    setCurrentTrack,
    playing,
    setPlaying,
    shuffle,
    setShuffle,
    repeatMode,
    setRepeatMode,
    currentIndex,
    queueLength: playableTracks.length,
    canPrevious: adjacentIndex(-1) !== null,
    canNext: adjacentIndex(1) !== null,
    playTrack,
    playAdjacent,
  };
}
