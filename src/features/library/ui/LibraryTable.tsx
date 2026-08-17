import { useState } from "react";
import type { LibraryColumnId, LibraryTableLayout, Track } from "../../../domain/models";
import { genreLabel } from "../../../domain/genre";
import { PLAYLIST_DRAG_TYPE, encodePlaylistDrag } from "../../../domain/playlistDrag";
import { Icon } from "../../../shared/ui/Icon";
import { TrackCover } from "../../../entities/track/ui/TrackCover";
import { DEFAULT_LIBRARY_TABLE_LAYOUT } from "../model/libraryQuery";

const labels: Record<LibraryColumnId, string> = {
  track: "Трек", genre: "Жанр", bpm: "BPM", key: "Тональность", format: "Формат", duration: "Время",
};

const limits: Record<LibraryColumnId, [number, number]> = {
  track: [220, 520], genre: [100, 300], bpm: [56, 120], key: [90, 220], format: [68, 130], duration: [62, 130],
};

function duration(seconds: number) {
  const minutes = Math.floor(seconds / 60);
  return `${minutes}:${String(seconds % 60).padStart(2, "0")}`;
}

function RatingStars({ track, busy, onRating }: { track: Track; busy: boolean; onRating: (track: Track, rating: number) => void }) {
  return <span className="track-rating" aria-label={`Рейтинг ${track.title}: ${track.rating} из 5`}>
    {[1, 2, 3, 4, 5].map((rating) => <button key={rating} type="button" draggable={false} disabled={busy} className={rating <= track.rating ? "active" : ""} aria-label={`${rating} из 5`} title={rating === track.rating ? "Нажмите ещё раз, чтобы сбросить рейтинг" : `Поставить ${rating} из 5`} onClick={(event) => { event.stopPropagation(); onRating(track, rating === track.rating ? 0 : rating); }}>★</button>)}
  </span>;
}

function cell(column: LibraryColumnId, track: Track, playingId: string | null, ratingBusyId: string | null, onRating: (track: Track, rating: number) => void) {
  switch (column) {
    case "track": return <div className="track-cell"><TrackCover track={track} className="cover" disabled={track.analysisState === "missing"}>{playingId === track.id ? <Icon name="volume" size={16}/> : track.artist.at(0)}</TrackCover><span><strong>{track.title}</strong><small>{track.analysisState === "missing" ? "Файл отсутствует · запись сохранена" : `${track.artist} · ${track.album}`}</small><RatingStars track={track} busy={ratingBusyId === track.id} onRating={onRating}/></span></div>;
    case "genre": return <span className={track.genre ? "genre-chip" : "genre-chip muted"}>{genreLabel(track.genre, track.subgenre)}</span>;
    case "bpm": return <span className="mono">{track.bpm ?? "—"}</span>;
    case "key": return <span className="key-cell">{track.camelot ?? "—"}<small>{track.musicalKey ?? "Не определена"}</small></span>;
    case "format": return <span className={`format ${track.format.toLowerCase()}`}>{track.format}</span>;
    case "duration": return <span className="mono subtle">{duration(track.durationSeconds)}</span>;
  }
}

export function LibraryTable({ tracks, layout, selectedId, checkedIds, playingId, ratingBusyId, onLayoutChange, onSelect, onCheck, onCheckAll, onPlay, onEdit, onRelink, onRating }: {
  tracks: Track[];
  layout: LibraryTableLayout;
  selectedId: string | null;
  checkedIds: Set<string>;
  playingId: string | null;
  ratingBusyId: string | null;
  onLayoutChange: (layout: LibraryTableLayout) => void;
  onSelect: (track: Track) => void;
  onCheck: (trackId: string, checked: boolean) => void;
  onCheckAll: (checked: boolean) => void;
  onPlay: (track: Track) => void;
  onEdit: (track: Track) => void;
  onRelink: (track: Track) => void;
  onRating: (track: Track, rating: number) => void;
}) {
  const [settingsOpen, setSettingsOpen] = useState(false);
  const visible = layout.columns.filter((column) => column.visible);
  const update = (id: LibraryColumnId, change: { visible?: boolean; width?: number }) => onLayoutChange({ columns: layout.columns.map((column) => column.id === id ? { ...column, ...change } : column) });
  const move = (index: number, direction: -1 | 1) => {
    const target = index + direction;
    if (target < 0 || target >= layout.columns.length) return;
    const columns = [...layout.columns];
    [columns[index], columns[target]] = [columns[target], columns[index]];
    onLayoutChange({ columns });
  };

  return (
    <div className="table-shell configurable-table">
      <button className="column-settings-button" aria-expanded={settingsOpen} onClick={() => setSettingsOpen((value) => !value)}><Icon name="settings" size={13}/>Столбцы</button>
      {settingsOpen && <section className="column-settings" aria-label="Настройка столбцов">
        <header><strong>Столбцы библиотеки</strong><button onClick={() => onLayoutChange(structuredClone(DEFAULT_LIBRARY_TABLE_LAYOUT))}>Сбросить</button></header>
        {layout.columns.map((column, index) => <article key={column.id}>
          <label><input type="checkbox" checked={column.visible} disabled={column.id === "track"} onChange={(event) => update(column.id, { visible: event.target.checked })}/><span>{labels[column.id]}</span></label>
          <button aria-label={`Переместить ${labels[column.id]} выше`} disabled={index === 0} onClick={() => move(index, -1)}>↑</button>
          <button aria-label={`Переместить ${labels[column.id]} ниже`} disabled={index === layout.columns.length - 1} onClick={() => move(index, 1)}>↓</button>
          <input aria-label={`Ширина ${labels[column.id]}`} type="range" min={limits[column.id][0]} max={limits[column.id][1]} step="2" value={column.width} onChange={(event) => update(column.id, { width: Number(event.target.value) })}/>
          <output>{column.width}px</output>
        </article>)}
        <small>Раскладка сохраняется локально и входит в резервную копию.</small>
      </section>}
      <table style={{ minWidth: 80 + visible.reduce((sum, column) => sum + column.width, 0) }}>
        <colgroup><col style={{ width: 38 }}/>{visible.map((column) => <col key={column.id} style={{ width: column.width }}/>) }<col style={{ width: 42 }}/></colgroup>
        <thead><tr><th className="check"><input aria-label="Выбрать все" type="checkbox" checked={tracks.length > 0 && tracks.every((track) => checkedIds.has(track.id))} onChange={(event) => onCheckAll(event.target.checked)} /></th>{visible.map((column) => <th key={column.id}>{labels[column.id]}</th>)}<th /></tr></thead>
        <tbody>
          {tracks.map((track) => (
            <tr key={track.id} draggable className={[selectedId === track.id ? "selected" : "", playingId === track.id ? "playing" : "", track.analysisState === "missing" ? "missing" : ""].filter(Boolean).join(" ")} onDragStart={(event) => { event.dataTransfer.effectAllowed = "copy"; event.dataTransfer.setData(PLAYLIST_DRAG_TYPE, encodePlaylistDrag({ trackId: track.id })); }} onClick={() => onSelect(track)} onDoubleClick={() => { if (track.analysisState !== "missing") onPlay(track); }}>
              <td className="check"><input aria-label={`Выбрать ${track.title}`} type="checkbox" checked={checkedIds.has(track.id)} onChange={(event) => onCheck(track.id, event.target.checked)} onClick={(event) => event.stopPropagation()} /></td>
              {visible.map((column) => <td key={column.id}>{cell(column.id, track, playingId, ratingBusyId, onRating)}</td>)}
              <td>{track.analysisState === "missing" ? <button className="relink-button" onClick={(event) => { event.stopPropagation(); onRelink(track); }}>Найти файл</button> : <button className="icon-button" aria-label={`Изменить метаданные ${track.title}`} title="Изменить метаданные" onClick={(event) => { event.stopPropagation(); onEdit(track); }}><Icon name="more" /></button>}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {tracks.length === 0 && <div className="empty-state"><strong>Ничего не найдено</strong><span>Измените запрос или фильтр жанра.</span></div>}
    </div>
  );
}
