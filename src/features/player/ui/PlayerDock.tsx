import { Player } from "./Player";
import type { Track } from "../../../domain/models";
import type { RepeatMode } from "../model/usePlayerController";

export function PlayerDock({
  track,
  playing,
  canPrevious,
  canNext,
  shuffle,
  repeatMode,
  queuePosition,
  queueLength,
  resolveSource,
  resolveWaveform,
  onPlayingChange,
  onPrevious,
  onNext,
  onShuffleChange,
  onRepeatModeChange,
  onRevealCurrent,
}: {
  track: Track | null;
  playing: boolean;
  canPrevious: boolean;
  canNext: boolean;
  shuffle: boolean;
  repeatMode: RepeatMode;
  queuePosition: number;
  queueLength: number;
  resolveSource: (trackId: string) => Promise<string | null>;
  resolveWaveform: (trackId: string) => Promise<number[] | null>;
  onPlayingChange: (playing: boolean) => void;
  onPrevious: () => void;
  onNext: () => void;
  onShuffleChange: (shuffle: boolean) => void;
  onRepeatModeChange: (mode: RepeatMode) => void;
  onRevealCurrent: () => void;
}) {
  return <Player
    track={track}
    playing={playing}
    canPrevious={canPrevious}
    canNext={canNext}
    onPlayingChange={onPlayingChange}
    onPrevious={onPrevious}
    onNext={onNext}
    resolveSource={resolveSource}
    resolveWaveform={resolveWaveform}
    shuffle={shuffle}
    repeatMode={repeatMode}
    queuePosition={queuePosition}
    queueLength={queueLength}
    onShuffleChange={onShuffleChange}
    onRepeatModeChange={onRepeatModeChange}
    onRevealCurrent={onRevealCurrent}
  />;
}
