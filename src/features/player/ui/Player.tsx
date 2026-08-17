import { useEffect, useRef, useState, type MouseEvent } from "react";
import type { Track } from "../../../domain/models";
import { DEFAULT_WAVEFORM, formatPlaybackTime, waveformPointerRatio, waveformViewport } from "../../../domain/player";
import { Icon } from "../../../shared/ui/Icon";
import { TrackCover } from "../../../entities/track/ui/TrackCover";

interface PlayerProps {
  track: Track | null;
  playing: boolean;
  canPrevious: boolean;
  canNext: boolean;
  onPlayingChange: (playing: boolean) => void;
  onPrevious: () => void;
  onNext: () => void;
  resolveSource: (trackId: string) => Promise<string | null>;
  resolveWaveform: (trackId: string) => Promise<number[] | null>;
  shuffle: boolean;
  repeatMode: "off" | "all" | "one";
  queuePosition: number;
  queueLength: number;
  onShuffleChange: (enabled: boolean) => void;
  onRepeatModeChange: (mode: "off" | "all" | "one") => void;
  onRevealCurrent: () => void;
}

export function Player({
  track,
  playing,
  canPrevious,
  canNext,
  onPlayingChange,
  onPrevious,
  onNext,
  resolveSource,
  resolveWaveform,
  shuffle,
  repeatMode,
  queuePosition,
  queueLength,
  onShuffleChange,
  onRepeatModeChange,
  onRevealCurrent,
}: PlayerProps) {
  const audioRef = useRef<HTMLAudioElement>(null);
  const [source, setSource] = useState<string | null>(null);
  const [currentTime, setCurrentTime] = useState(0);
  const [duration, setDuration] = useState(0);
  const [volume, setVolume] = useState(0.72);
  const previousVolume = useRef(0.72);
  const [peaks, setPeaks] = useState(DEFAULT_WAVEFORM);
  const [waveformZoom, setWaveformZoom] = useState<1 | 2 | 4>(1);
  const [waveformFocus, setWaveformFocus] = useState(0);
  const [waveformHover, setWaveformHover] = useState<{ left: number; seconds: number } | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    const audio = audioRef.current;
    audio?.pause();
    setSource(null);
    setCurrentTime(0);
    setDuration(track?.durationSeconds ?? 0);
    setPeaks(DEFAULT_WAVEFORM);
    setWaveformZoom(1);
    setWaveformFocus(0);
    setWaveformHover(null);
    setError(null);
    if (!track) return () => { active = false; };

    setLoading(true);
    void resolveSource(track.id)
      .then((url) => {
        if (!active) return;
        if (!url) {
          setError("Прослушивание локальных файлов доступно в desktop-приложении");
          onPlayingChange(false);
          return;
        }
        setSource(url);
      })
      .catch((reason) => {
        if (!active) return;
        setError(`Трек недоступен: ${String(reason)}`);
        onPlayingChange(false);
      })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [track?.id, resolveSource, onPlayingChange]);

  useEffect(() => {
    const audio = audioRef.current;
    if (!audio || !source) return;
    if (playing) {
      void audio.play().catch((reason) => {
        setError(`Не удалось запустить воспроизведение: ${String(reason)}`);
        onPlayingChange(false);
      });
    } else {
      audio.pause();
    }
  }, [playing, source, onPlayingChange]);

  useEffect(() => {
    let active = true;
    if (!track) return () => { active = false; };
    void resolveWaveform(track.id)
      .then((nextPeaks) => {
        if (active && nextPeaks?.length) setPeaks(nextPeaks);
      })
      .catch((reason) => {
        if (active) console.warn("Waveform is unavailable", reason);
      });
    return () => { active = false; };
  }, [track?.id, resolveWaveform]);

  const progress = duration > 0 ? currentTime / duration : 0;
  const viewport = waveformViewport(peaks, waveformFocus, waveformZoom);
  const cycleZoom = () => {
    setWaveformFocus(progress);
    setWaveformZoom((current) => current === 1 ? 2 : current === 2 ? 4 : 1);
  };
  useEffect(() => {
    if (waveformZoom > 1 && (progress < viewport.startRatio || progress >= viewport.endRatio)) {
      setWaveformFocus(progress);
    }
  }, [progress, viewport.startRatio, viewport.endRatio, waveformZoom]);
  const seek = (event: MouseEvent<HTMLDivElement>) => {
    const audio = audioRef.current;
    if (!audio || !Number.isFinite(audio.duration)) return;
    const bounds = event.currentTarget.getBoundingClientRect();
    const localRatio = Math.max(0, Math.min(1, (event.clientX - bounds.left) / bounds.width));
    const ratio = waveformPointerRatio(localRatio, viewport.startRatio, viewport.endRatio);
    audio.currentTime = ratio * audio.duration;
    setCurrentTime(audio.currentTime);
  };
  const showHoverTime = (event: MouseEvent<HTMLDivElement>) => {
    const bounds = event.currentTarget.getBoundingClientRect();
    const localRatio = Math.max(0, Math.min(1, (event.clientX - bounds.left) / bounds.width));
    const ratio = waveformPointerRatio(localRatio, viewport.startRatio, viewport.endRatio);
    setWaveformHover({ left: localRatio * 100, seconds: ratio * (duration || track?.durationSeconds || 0) });
  };

  const previous = () => {
    const audio = audioRef.current;
    if (audio && audio.currentTime > 3) {
      audio.currentTime = 0;
      setCurrentTime(0);
    } else {
      onPrevious();
    }
  };

  const seekBy = (seconds: number) => {
    const audio = audioRef.current;
    if (!audio || !Number.isFinite(audio.duration)) return;
    audio.currentTime = Math.max(0, Math.min(audio.duration, audio.currentTime + seconds));
    setCurrentTime(audio.currentTime);
  };
  const toggleMute = () => {
    const next = volume > 0 ? 0 : previousVolume.current || 0.72;
    if (volume > 0) previousVolume.current = volume;
    setVolume(next);
    if (audioRef.current) audioRef.current.volume = next;
  };
  const cycleRepeat = () => onRepeatModeChange(repeatMode === "off" ? "all" : repeatMode === "all" ? "one" : "off");

  useEffect(() => {
    const keyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      if (event.repeat || target?.matches("input, select, textarea, button") || document.querySelector(".dialog-backdrop, .copy-overlay")) return;
      const key = event.key.toLowerCase();
      if (key === " " || key === "k") { event.preventDefault(); if (track && !loading && !error) onPlayingChange(!playing); }
      else if (key === "arrowleft" || key === "j") { event.preventDefault(); seekBy(event.shiftKey ? -5 : -10); }
      else if (key === "arrowright" || key === "l") { event.preventDefault(); seekBy(event.shiftKey ? 5 : 10); }
      else if (key === "m") toggleMute();
      else if (key === "s") onShuffleChange(!shuffle);
      else if (key === "r") cycleRepeat();
    };
    window.addEventListener("keydown", keyDown);
    return () => window.removeEventListener("keydown", keyDown);
  });

  return (
    <footer className="player">
      <audio
        ref={audioRef}
        src={source ?? undefined}
        loop={repeatMode === "one"}
        preload="metadata"
        onTimeUpdate={(event) => setCurrentTime(event.currentTarget.currentTime)}
        onDurationChange={(event) => setDuration(Number.isFinite(event.currentTarget.duration) ? event.currentTarget.duration : 0)}
        onEnded={onNext}
        onError={() => { setError("Файл не удалось декодировать"); onPlayingChange(false); }}
      />
      <div className="now-playing">
        <TrackCover track={track} className="player-cover">{track?.artist.at(0) || "S"}</TrackCover>
        <span title={error ?? undefined}><strong>{track?.title ?? "Выберите трек"}</strong><small className={error ? "player-error" : ""}>{error ?? track?.artist ?? "Двойной клик запускает воспроизведение"}</small></span>
      </div>
      <div className="transport">
        <div className="transport-buttons">
          <button className={`transport-mode ${shuffle ? "active" : ""}`} onClick={() => onShuffleChange(!shuffle)} aria-pressed={shuffle} title="Перемешивание · S"><Icon name="shuffle" size={15}/></button>
          <button className="transport-skip" onClick={previous} disabled={!track || (!canPrevious && currentTime <= 3)} aria-label="Предыдущий трек"><Icon name="previous" size={18}/></button>
          <button className="play-button" onClick={() => onPlayingChange(!playing)} disabled={!track || loading || !!error} aria-label={playing ? "Пауза" : "Воспроизвести"}><Icon name={playing ? "pause" : "play"} size={23}/></button>
          <button className="transport-skip" onClick={onNext} disabled={!canNext} aria-label="Следующий трек"><Icon name="next" size={18}/></button>
          <button className={`transport-mode ${repeatMode !== "off" ? "active" : ""}`} onClick={cycleRepeat} title="Повтор · R"><Icon name="repeat" size={15}/>{repeatMode === "one" && <b>1</b>}</button>
        </div>
        <div className="wave"><span className="time">{formatPlaybackTime(currentTime)}</span><div className="waveform" aria-label={`Вейвформа, масштаб ${waveformZoom} — нажмите для перемотки`} onClick={seek} onMouseMove={showHoverTime} onMouseLeave={() => setWaveformHover(null)}>{viewport.peaks.map((peak, index) => <i key={viewport.startIndex + index} className={(viewport.startIndex + index) / peaks.length <= progress ? "played" : ""} style={{ height: `${Math.round(peak * 100)}%` }}/>)}{waveformHover && <span className="waveform-hover" style={{ left: `${waveformHover.left}%` }}>{formatPlaybackTime(waveformHover.seconds)}</span>}</div><button className="waveform-zoom" onClick={cycleZoom} title="Масштаб waveform">×{waveformZoom}</button><span className="time">{formatPlaybackTime(duration || track?.durationSeconds || 0)}</span></div>
      </div>
      <div className="player-meta"><button className="queue-position" onClick={onRevealCurrent} disabled={!track} title="Показать текущий трек в библиотеке">{queuePosition > 0 ? `${queuePosition}/${queueLength}` : `↗ ${queueLength}`}</button><button className="volume-button" onClick={toggleMute} aria-label={volume > 0 ? "Выключить звук" : "Включить звук"} title="Mute · M"><Icon name={volume > 0 ? "volume" : "mute"} size={16}/></button><input className="volume" type="range" min="0" max="1" step="0.01" value={volume} aria-label="Громкость" onChange={(event) => { const next = Number(event.target.value); if (next > 0) previousVolume.current = next; setVolume(next); if (audioRef.current) audioRef.current.volume = next; }}/><span>{Math.round(volume * 100)}%</span></div>
    </footer>
  );
}
