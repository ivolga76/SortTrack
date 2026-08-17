import type { TrackQuery } from "../../../domain/models";

export function LibrarySelectionActions({
  checkedCount,
  visibleCount,
  total,
  sortBy,
  descending,
  ratingTarget,
  ratingBusy,
  datasetBusy,
  genreBusy,
  metadataLoading,
  playlistCount,
  onSortByChange,
  onDescendingChange,
  onRatingTargetChange,
  onApplyRating,
  onOpenDataset,
  onOpenGenres,
  onOpenAnalysis,
  onOpenMetadata,
  onOpenPlaylists,
}: {
  checkedCount: number;
  visibleCount: number;
  total: number;
  sortBy: TrackQuery["sortBy"];
  descending: boolean;
  ratingTarget: string;
  ratingBusy: boolean;
  datasetBusy: boolean;
  genreBusy: boolean;
  metadataLoading: boolean;
  playlistCount: number;
  onSortByChange: (value: TrackQuery["sortBy"]) => void;
  onDescendingChange: (descending: boolean) => void;
  onRatingTargetChange: (value: string) => void;
  onApplyRating: () => void;
  onOpenDataset: () => void;
  onOpenGenres: () => void;
  onOpenAnalysis: () => void;
  onOpenMetadata: () => void;
  onOpenPlaylists: () => void;
}) {
  return <div className="selection-actions">
    <span>{checkedCount ? `${checkedCount} выбрано` : `${visibleCount} из ${total}`}</span>
    <select className="track-sort" value={sortBy} onChange={(event) => onSortByChange(event.target.value as TrackQuery["sortBy"])} aria-label="Сортировка треков">
      <option value="added">По добавлению</option><option value="title">По названию</option><option value="artist">По исполнителю</option><option value="bpm">По BPM</option>
    </select>
    <button title={descending ? "По убыванию" : "По возрастанию"} onClick={() => onDescendingChange(!descending)}>{descending ? "↓" : "↑"}</button>
    <select className="track-sort rating-select" value={ratingTarget} onChange={(event) => onRatingTargetChange(event.target.value)} aria-label="Массовый рейтинг">
      <option value="-1">Рейтинг…</option><option value="0">Сбросить</option>{[1, 2, 3, 4, 5].map((value) => <option value={value} key={value}>{value} ★</option>)}
    </select>
    <button onClick={onApplyRating} disabled={ratingBusy || checkedCount === 0 || ratingTarget === "-1"}>{ratingBusy ? "Рейтинг…" : "Применить"}</button>
    <button onClick={onOpenDataset} disabled={datasetBusy}>{datasetBusy ? "Аудит…" : "ML dataset"}</button>
    <button onClick={onOpenGenres} disabled={genreBusy}>{genreBusy ? "Анализ жанров…" : `Жанры${checkedCount ? ` · ${checkedCount}` : ""}`}</button>
    <button onClick={onOpenAnalysis}>Анализ BPM/Key{checkedCount ? ` · ${checkedCount}` : ""}</button>
    <button onClick={onOpenMetadata} disabled={metadataLoading}>{metadataLoading ? "Чтение тегов…" : checkedCount > 1 ? `Изменить теги · ${checkedCount}` : "Изменить теги"}</button>
    <button onClick={onOpenPlaylists}>Плейлисты {playlistCount ? `· ${playlistCount}` : ""}</button>
  </div>;
}
