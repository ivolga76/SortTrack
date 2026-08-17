import type { ReactNode } from "react";
import type { TrackQuery } from "../../../domain/models";
import { GENRE_TAXONOMY } from "../../../domain/genre";
import { CAMELOT_KEYS } from "../model/libraryQuery";

export function LibraryFilters({
  genre,
  advancedFiltersOpen,
  activeAdvancedFilterCount,
  minBpm,
  maxBpm,
  minRating,
  camelot,
  format,
  fileStatus,
  selectionActions,
  onGenreChange,
  onAdvancedFiltersOpenChange,
  onMinBpmChange,
  onMaxBpmChange,
  onMinRatingChange,
  onCamelotChange,
  onFormatChange,
  onFileStatusChange,
  onReset,
}: {
  genre: string | null;
  advancedFiltersOpen: boolean;
  activeAdvancedFilterCount: number;
  minBpm: string;
  maxBpm: string;
  minRating: string;
  camelot: string;
  format: "" | "mp3" | "flac";
  fileStatus: TrackQuery["fileStatus"];
  selectionActions: ReactNode;
  onGenreChange: (genre: string | null) => void;
  onAdvancedFiltersOpenChange: (open: boolean) => void;
  onMinBpmChange: (value: string) => void;
  onMaxBpmChange: (value: string) => void;
  onMinRatingChange: (value: string) => void;
  onCamelotChange: (value: string) => void;
  onFormatChange: (value: "" | "mp3" | "flac") => void;
  onFileStatusChange: (value: TrackQuery["fileStatus"]) => void;
  onReset: () => void;
}) {
  return <>
    <section className="toolbar">
      <div className="genre-tabs">
        <button className={!genre ? "active" : ""} onClick={() => onGenreChange(null)}>Все</button>
        {Object.keys(GENRE_TAXONOMY).slice(0, 5).map((item) => <button key={item} className={genre === item ? "active" : ""} onClick={() => onGenreChange(item)}>{item}</button>)}
        <button className={advancedFiltersOpen || activeAdvancedFilterCount ? "advanced-filter-toggle active" : "advanced-filter-toggle"} onClick={() => onAdvancedFiltersOpenChange(!advancedFiltersOpen)}>
          Фильтры{activeAdvancedFilterCount ? ` · ${activeAdvancedFilterCount}` : ""}
        </button>
      </div>
      {selectionActions}
    </section>
    {advancedFiltersOpen && <section className="advanced-filters" aria-label="Расширенные фильтры библиотеки">
      <label><span>BPM от</span><input type="number" min="0" max="400" value={minBpm} onChange={(event) => onMinBpmChange(event.target.value)}/></label>
      <label><span>BPM до</span><input type="number" min="0" max="400" value={maxBpm} onChange={(event) => onMaxBpmChange(event.target.value)}/></label>
      <label><span>Рейтинг от</span><select value={minRating} onChange={(event) => onMinRatingChange(event.target.value)}><option value="">Любой</option>{[1, 2, 3, 4, 5].map((value) => <option value={value} key={value}>{value} ★</option>)}</select></label>
      <label><span>Camelot</span><select value={camelot} onChange={(event) => onCamelotChange(event.target.value)}><option value="">Любая</option>{CAMELOT_KEYS.map((value) => <option value={value} key={value}>{value}</option>)}</select></label>
      <label><span>Формат</span><select value={format} onChange={(event) => onFormatChange(event.target.value as "" | "mp3" | "flac")}><option value="">Любой</option><option value="flac">FLAC</option><option value="mp3">MP3</option></select></label>
      <label><span>Файл</span><select value={fileStatus} onChange={(event) => onFileStatusChange(event.target.value as TrackQuery["fileStatus"])}><option value="all">Любой статус</option><option value="available">Доступен</option><option value="problems">Повреждён / не читается</option><option value="missing">Отсутствует</option></select></label>
      <button onClick={onReset} disabled={activeAdvancedFilterCount === 0}>Сбросить фильтры</button>
    </section>}
  </>;
}
