import { useEffect, useState, type DragEvent, type KeyboardEvent } from "react";
import type { Playlist, PlaylistPlacement, PlaylistRemovalPreview, SmartPlaylistRule, Track } from "../../../domain/models";
import { PLAYLIST_DRAG_TYPE, decodePlaylistDrag, encodePlaylistDrag, placementFromDrop } from "../../../domain/playlistDrag";
import { formatPlaybackTime } from "../../../domain/player";
import { Icon } from "../../../shared/ui/Icon";
import { TrackCover } from "../../../entities/track/ui/TrackCover";

interface PlaylistWorkspaceProps {
  playlists: Playlist[];
  selectedTrack: Track | null;
  busy: boolean;
  removalPreview: PlaylistRemovalPreview | null;
  onClose: () => void;
  onCreate: () => void;
  onCreateSmart: () => void;
  onEditSmart: (playlistId: string) => void;
  onSnapshotSmart: (playlistId: string) => void;
  onRename: (playlistId: string, name: string) => void;
  onPreviewDelete: (playlistId: string) => void;
  onCancelDelete: () => void;
  onConfirmDelete: () => void;
  onPlace: (placement: PlaylistPlacement) => void;
  onRemove: (playlistId: string, itemId: string) => void;
  onPlay: (track: Track) => void;
  onCopy: (playlistId: string) => void;
  onExport: (playlistId: string) => void;
}

function payloadFromEvent(event: DragEvent): ReturnType<typeof decodePlaylistDrag> {
  return decodePlaylistDrag(event.dataTransfer.getData(PLAYLIST_DRAG_TYPE));
}

function describeSmartRule(rule: SmartPlaylistRule): string {
  const parts = [];
  if (rule.genre) parts.push(rule.subgenre ?? rule.genre);
  if (rule.minBpm !== null || rule.maxBpm !== null) parts.push(`${rule.minBpm ?? "…"}–${rule.maxBpm ?? "…"} BPM`);
  if (rule.minRating !== null) parts.push(`от ${rule.minRating} ★`);
  if (rule.camelot) parts.push(rule.camelot);
  return parts.join(" · ");
}

export function PlaylistWorkspace({ playlists, selectedTrack, busy, removalPreview, onClose, onCreate, onCreateSmart, onEditSmart, onSnapshotSmart, onRename, onPreviewDelete, onCancelDelete, onConfirmDelete, onPlace, onRemove, onPlay, onCopy, onExport }: PlaylistWorkspaceProps) {
  const [deleteConfirmation, setDeleteConfirmation] = useState("");
  useEffect(() => setDeleteConfirmation(""), [removalPreview?.playlistId]);
  const drop = (event: DragEvent, playlistId: string, position: number) => {
    event.preventDefault();
    event.stopPropagation();
    const payload = payloadFromEvent(event);
    if (payload) onPlace(placementFromDrop(payload, playlistId, position));
  };
  const commitName = (playlistId: string, value: string) => {
    const name = value.trim();
    if (name) onRename(playlistId, name);
  };
  const blurOnEnter = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Enter") event.currentTarget.blur();
  };

  return (
    <aside className="playlist-workspace" aria-label="Рабочая область плейлистов">
      <header className="playlist-workspace-header">
        <div><span className="eyebrow">Ручная сортировка</span><strong>Плейлисты</strong></div>
        <span>{playlists.length} открыто</span>
        <button className="playlist-add" onClick={onCreate} disabled={busy}>+ Новый</button>
        <button className="playlist-add smart" onClick={onCreateSmart} disabled={busy}>+ Авто</button>
        <button className="playlist-close" onClick={onClose} aria-label="Закрыть плейлисты">×</button>
      </header>
      {removalPreview && <section className="playlist-removal"><strong>Удалить плейлист «{removalPreview.name}»?</strong><span>{removalPreview.itemCount} треков будут убраны только из списка. Музыкальные файлы и библиотека не изменяются.</span><label><span>Введите <strong>УДАЛИТЬ ПЛЕЙЛИСТ</strong></span><input value={deleteConfirmation} onChange={(event) => setDeleteConfirmation(event.target.value)} disabled={busy} autoComplete="off"/></label><div><button disabled={busy} onClick={onCancelDelete}>Отмена</button><button className="danger" disabled={busy || deleteConfirmation !== "УДАЛИТЬ ПЛЕЙЛИСТ"} onClick={onConfirmDelete}>Удалить плейлист</button></div></section>}
      <div className="playlist-board">
        {playlists.map((playlist) => (
          <section
            className="playlist-card"
            key={playlist.id}
            onDragOver={(event) => { if (playlist.kind === "manual") event.preventDefault(); }}
            onDrop={(event) => { if (playlist.kind === "manual") drop(event, playlist.id, playlist.items.length); }}
          >
            <header>
              <input defaultValue={playlist.name} key={`${playlist.id}-${playlist.name}`} maxLength={100} aria-label="Название плейлиста" onBlur={(event) => commitName(playlist.id, event.target.value)} onKeyDown={blurOnEnter}/>
              <span>{playlist.items.length}</span>
              <button className="playlist-copy playlist-export" onClick={() => onExport(playlist.id)} disabled={playlist.kind === "smart" || playlist.items.length === 0 || busy} title={playlist.kind === "smart" ? "Сначала сохраните динамический результат как ручной плейлист" : "Экспортировать плейлист"} aria-label={`Экспортировать ${playlist.name}`}><Icon name="export" size={14}/></button>
              <button className="playlist-copy" onClick={() => onCopy(playlist.id)} disabled={playlist.kind === "smart" || playlist.items.length === 0 || busy} title={playlist.kind === "smart" ? "Сначала сохраните динамический результат как ручной плейлист" : "Безопасно копировать файлы"} aria-label={`Копировать файлы ${playlist.name}`}><Icon name="folder" size={14}/></button>
              <button className="playlist-delete" onClick={() => onPreviewDelete(playlist.id)} disabled={busy} title="Удалить только плейлист" aria-label={`Удалить плейлист ${playlist.name}`}>×</button>
            </header>
            {playlist.kind === "smart" && playlist.smartRule && <div className="smart-playlist-rule"><strong>Авто</strong><span>{describeSmartRule(playlist.smartRule)}</span><button onClick={() => onEditSmart(playlist.id)} disabled={busy} title="Изменить фильтры, лимит и сортировку">Правило</button><button onClick={() => onSnapshotSmart(playlist.id)} disabled={busy || playlist.items.length === 0} title="Создать отдельный обычный плейлист из текущего результата">Снимок</button></div>}
            {selectedTrack && playlist.kind === "manual" && <button className="add-selected" onClick={() => onPlace({ targetPlaylistId: playlist.id, targetPosition: playlist.items.length, trackId: selectedTrack.id })}>+ Добавить выбранный</button>}
            <div className="playlist-items">
              {playlist.items.map((item, index) => (
                <article
                  className="playlist-track"
                  key={item.id}
                  draggable={playlist.kind === "manual"}
                  onDragStart={(event) => {
                    if (playlist.kind !== "manual") { event.preventDefault(); return; }
                    event.dataTransfer.effectAllowed = "move";
                    event.dataTransfer.setData(PLAYLIST_DRAG_TYPE, encodePlaylistDrag({ trackId: item.track.id, itemId: item.id, sourcePlaylistId: playlist.id }));
                  }}
                  onDragOver={(event) => event.preventDefault()}
                  onDrop={(event) => { if (playlist.kind === "manual") drop(event, playlist.id, index); }}
                  onDoubleClick={() => onPlay(item.track)}
                >
                  <span className="drag-handle">⠿</span>
                  <TrackCover track={item.track} className="mini-cover" disabled={item.track.analysisState === "missing"}>{item.track.artist.at(0)}</TrackCover>
                  <span className="playlist-track-title"><strong>{item.track.title}</strong><small>{item.track.artist}</small></span>
                  <span className="playlist-track-info"><small>{item.track.bpm ?? "—"} BPM</small><small>{formatPlaybackTime(item.track.durationSeconds)}</small></span>
                  {playlist.kind === "manual" && <button onClick={(event) => { event.stopPropagation(); onRemove(playlist.id, item.id); }} aria-label={`Убрать ${item.track.title}`}>×</button>}
                </article>
              ))}
              {playlist.items.length === 0 && <div className="playlist-empty"><Icon name="playlist"/><span>Перетащите сюда треки</span></div>}
            </div>
          </section>
        ))}
        {playlists.length === 0 && <div className="playlist-zero"><Icon name="playlist" size={28}/><strong>Создайте первый плейлист</strong><span>Затем перетаскивайте треки из библиотеки.</span><button onClick={onCreate}>Создать плейлист</button></div>}
      </div>
    </aside>
  );
}
