import { useState } from "react";
import type { SmartPlaylistRule } from "../../../domain/models";
import { GENRE_TAXONOMY } from "../../../domain/genre";

interface Props {
  genres: string[];
  busy: boolean;
  initialName?: string;
  initialRule?: SmartPlaylistRule;
  onCreate?: (name: string, rule: SmartPlaylistRule) => void;
  onUpdate?: (rule: SmartPlaylistRule) => void;
  onClose: () => void;
}

function optionalNumber(value: string): number | null {
  return value.trim() === "" ? null : Number(value);
}

export function SmartPlaylistDialog({ genres, busy, initialName, initialRule, onCreate, onUpdate, onClose }: Props) {
  const editing = initialRule !== undefined;
  const [name, setName] = useState(initialName ?? "Новый автоплейлист");
  const [genre, setGenre] = useState(initialRule?.genre ?? "");
  const [subgenre, setSubgenre] = useState(initialRule?.subgenre ?? "");
  const [minBpm, setMinBpm] = useState(initialRule?.minBpm?.toString() ?? "");
  const [maxBpm, setMaxBpm] = useState(initialRule?.maxBpm?.toString() ?? "");
  const [minRating, setMinRating] = useState(initialRule?.minRating?.toString() ?? "");
  const [camelot, setCamelot] = useState(initialRule?.camelot ?? "");
  const [limitCount, setLimitCount] = useState(initialRule?.limitCount ?? 200);
  const [sortBy, setSortBy] = useState<SmartPlaylistRule["sortBy"]>(initialRule?.sortBy ?? "added");
  const [descending, setDescending] = useState(initialRule?.descending ?? true);
  const minimum = optionalNumber(minBpm);
  const maximum = optionalNumber(maxBpm);
  const rating = optionalNumber(minRating);
  const subgenres = genre ? (GENRE_TAXONOMY[genre as keyof typeof GENRE_TAXONOMY] ?? []) : [];
  const camelotKeys = Array.from({ length: 12 }, (_, index) => [`${index + 1}A`, `${index + 1}B`]).flat();
  const bpmValid = [minimum, maximum].every((value) => value === null || (Number.isFinite(value) && value >= 20 && value <= 300)) && !(minimum !== null && maximum !== null && minimum > maximum);
  const ratingValid = rating === null || (Number.isInteger(rating) && rating >= 1 && rating <= 5);
  const canSubmit = name.trim().length > 0 && name.trim().length <= 100 && (genre !== "" || minimum !== null || maximum !== null || rating !== null || camelot !== "") && bpmValid && ratingValid && limitCount >= 1 && limitCount <= 1000;
  const submit = () => {
    const rule = { genre: genre || null, subgenre: subgenre || null, minBpm: minimum, maxBpm: maximum, minRating: rating, camelot: camelot || null, limitCount, sortBy, descending };
    if (editing) onUpdate?.(rule);
    else onCreate?.(name.trim(), rule);
  };

  return <div className="dialog-backdrop smart-playlist-overlay"><section className="metadata-dialog smart-playlist-dialog" role="dialog" aria-modal="true" aria-labelledby="smart-playlist-title">
    <header><div><span>Динамический список</span><h2 id="smart-playlist-title">{editing ? `Изменить «${initialName}»` : "Создать автоплейлист"}</h2><small>Состав вычисляется из актуальной библиотеки и не изменяет ручные плейлисты.</small></div><button className="dialog-close" onClick={onClose} disabled={busy}>×</button></header>
    <div className="smart-playlist-form">
      {!editing && <label className="smart-playlist-name"><span>Название</span><input value={name} maxLength={100} onChange={(event) => setName(event.target.value)}/></label>}
      <label><span>Жанр</span><select value={genre} onChange={(event) => { setGenre(event.target.value); setSubgenre(""); }}><option value="">Любой</option>{genres.map((value) => <option value={value} key={value}>{value}</option>)}</select></label>
      <label><span>Поджанр</span><select value={subgenre} disabled={subgenres.length === 0} onChange={(event) => setSubgenre(event.target.value)}><option value="">Любой</option>{subgenres.map((value) => <option value={value} key={value}>{value}</option>)}</select></label>
      <label><span>BPM от</span><input type="number" min={20} max={300} value={minBpm} onChange={(event) => setMinBpm(event.target.value)}/></label>
      <label><span>BPM до</span><input type="number" min={20} max={300} value={maxBpm} onChange={(event) => setMaxBpm(event.target.value)}/></label>
      <label><span>Рейтинг не ниже</span><select value={minRating} onChange={(event) => setMinRating(event.target.value)}><option value="">Любой</option>{[1, 2, 3, 4, 5].map((value) => <option value={value} key={value}>{value} ★</option>)}</select></label>
      <label><span>Camelot</span><select value={camelot} onChange={(event) => setCamelot(event.target.value)}><option value="">Любая</option>{camelotKeys.map((value) => <option value={value} key={value}>{value}</option>)}</select></label>
      <label><span>Максимум треков</span><input type="number" min={1} max={1000} value={limitCount} onChange={(event) => setLimitCount(Number(event.target.value))}/></label>
      <label><span>Сортировка</span><select value={sortBy} onChange={(event) => setSortBy(event.target.value as SmartPlaylistRule["sortBy"])}><option value="added">По добавлению</option><option value="title">По названию</option><option value="bpm">По BPM</option></select></label>
      <label className="smart-playlist-direction"><input type="checkbox" checked={descending} onChange={(event) => setDescending(event.target.checked)}/><span>По убыванию</span></label>
      {!bpmValid && <p className="metadata-error">BPM должен быть от 20 до 300; нижняя граница не может превышать верхнюю.</p>}
      {!ratingValid && <p className="metadata-error">Минимальный рейтинг должен быть от 1 до 5.</p>}
      {genre === "" && minimum === null && maximum === null && rating === null && camelot === "" && <p className="smart-playlist-hint">Выберите хотя бы один фильтр: жанр, BPM, рейтинг или Camelot.</p>}
    </div>
    <footer><span>Автоплейлист доступен только для чтения; результат обновляется сразу после сохранения.</span><button className="secondary" onClick={onClose} disabled={busy}>Отмена</button><button className="primary" disabled={busy || !canSubmit} onClick={submit}>{busy ? (editing ? "Сохранение…" : "Создание…") : (editing ? "Сохранить правило" : "Создать")}</button></footer>
  </section></div>;
}
