import { useEffect, useMemo, useRef, useState } from "react";
import { GENRE_TAXONOMY, genreLabel } from "../../../domain/genre";
import { genreReviewProgress, remainingGenreSuggestions } from "../../../domain/genreReview";
import type { GenreCandidate, GenreSuggestion } from "../../../domain/models";

interface Props {
  suggestions: GenreSuggestion[];
  busy: boolean;
  onConfirm: (trackId: string, genre: string, subgenre: string | null, locked: boolean) => void;
  onClose: () => void;
}

const choices = Object.entries(GENRE_TAXONOMY).flatMap(([genre, subgenres]) => [
  { value: `${genre}::`, genre, subgenre: null as string | null, label: genre },
  ...subgenres.map((subgenre) => ({ value: `${genre}::${subgenre}`, genre, subgenre, label: `${genre} → ${subgenre}` })),
]);

function Candidate({ candidate, disabled, locked, onConfirm }: { candidate: GenreCandidate; disabled: boolean; locked: boolean; onConfirm: () => void }) {
  return <article className="genre-candidate">
    <header><span>#{candidate.rank}</span><strong>{genreLabel(candidate.genre, candidate.subgenre)}</strong><em>score {candidate.score.toFixed(2)}</em></header>
    <ul>{candidate.signals.map((signal) => <li key={signal}>{signal}</li>)}</ul>
    <button className="primary" disabled={disabled} onClick={onConfirm}>{locked ? `Подтвердить и зафиксировать · ${candidate.rank}` : `Подтвердить · ${candidate.rank}`}</button>
  </article>;
}

export function GenreAssistantDialog({ suggestions, busy, onConfirm, onClose }: Props) {
  const initialTotal = useRef(suggestions.length);
  const [skipped, setSkipped] = useState<Set<string>>(() => new Set());
  const [lockResult, setLockResult] = useState(false);
  const [manual, setManual] = useState(choices[0].value);
  const remaining = useMemo(() => remainingGenreSuggestions(suggestions, skipped), [suggestions, skipped]);
  const suggestion = remaining[0] ?? null;
  const reviewed = Math.max(0, initialTotal.current - remaining.length);
  const selectedChoice = choices.find((choice) => choice.value === manual) ?? choices[0];

  const skipCurrent = () => {
    if (!suggestion || busy) return;
    setSkipped((current) => new Set(current).add(suggestion.trackId));
  };

  useEffect(() => {
    if (!suggestion || busy || suggestion.locked) return;
    const handleKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      if (event.repeat || target?.matches("input, select, textarea, button")) return;
      const rank = Number(event.key);
      if (rank >= 1 && rank <= 3) {
        const candidate = suggestion.candidates.find((item) => item.rank === rank);
        if (candidate) {
          event.preventDefault();
          onConfirm(suggestion.trackId, candidate.genre, candidate.subgenre, lockResult);
        }
      } else if (event.key.toLowerCase() === "s") {
        event.preventDefault();
        skipCurrent();
      } else if (event.key.toLowerCase() === "l") {
        event.preventDefault();
        setLockResult((current) => !current);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [busy, lockResult, onConfirm, suggestion]);

  return <div className="copy-overlay genre-overlay" role="presentation"><section className="copy-dialog genre-dialog" role="dialog" aria-modal="true" aria-labelledby="genre-title">
    <header><div><span className="eyebrow">Решение принимает пользователь</span><h2 id="genre-title">Очередь разметки жанров</h2></div><button onClick={onClose} aria-label="Закрыть рекомендации">×</button></header>
    <div className="genre-review-progress"><div><strong>{reviewed} / {initialTotal.current}</strong><span>обработано в этой очереди</span></div><i><b style={{ width: `${genreReviewProgress(initialTotal.current, remaining.length)}%` }}/></i><small>1/2/3 — кандидат · L — фиксация · S — пропустить</small></div>
    <div className="recovery-message"><strong>Кандидаты не изменяют аудиофайлы</strong><span>Каждое решение подтверждается отдельно. Пропущенный трек остаётся неклассифицированным. Score — сравнительная оценка сигнала, а не вероятность.</span></div>
    {suggestion ? <div className="genre-suggestions"><section className="genre-suggestion" key={suggestion.trackId}>
      <header><div><strong>{suggestion.title}</strong><span>{suggestion.artist || "Неизвестный исполнитель"}</span></div><small>таксономия v{suggestion.taxonomyVersion} · {suggestion.inferenceVersion}</small></header>
      {suggestion.locked && <div className="genre-locked"><span>Зафиксировано: {genreLabel(suggestion.confirmedGenre, suggestion.confirmedSubgenre)}</span><button disabled={busy || !suggestion.confirmedGenre} onClick={() => suggestion.confirmedGenre && onConfirm(suggestion.trackId, suggestion.confirmedGenre, suggestion.confirmedSubgenre, false)}>Снять фиксацию</button></div>}
      <div className="genre-candidate-grid">{suggestion.candidates.map((candidate) => <Candidate key={`${candidate.genre}-${candidate.subgenre}`} candidate={candidate} disabled={busy || suggestion.locked} locked={lockResult} onConfirm={() => onConfirm(suggestion.trackId, candidate.genre, candidate.subgenre, lockResult)} />)}</div>
      <footer><label><span>Другой вариант</span><select value={manual} disabled={busy || suggestion.locked} onChange={(event) => setManual(event.target.value)}>{choices.map((choice) => <option key={choice.value} value={choice.value}>{choice.label}</option>)}</select></label><label className="genre-lock"><input type="checkbox" checked={lockResult} disabled={busy || suggestion.locked} onChange={(event) => setLockResult(event.target.checked)}/> Зафиксировать для validation dataset</label><button disabled={busy || suggestion.locked} onClick={() => onConfirm(suggestion.trackId, selectedChoice.genre, selectedChoice.subgenre, lockResult)}>Назначить вручную</button><button disabled={busy} onClick={skipCurrent}>Пропустить · S</button></footer>
    </section></div> : <section className="genre-review-complete"><strong>Очередь обработана</strong><span>Подтверждённые решения сохранены. Пропущенные треки не изменены и смогут попасть в следующую очередь.</span></section>}
    <footer><div className="copy-safety"><strong>{remaining.length ? `Осталось: ${remaining.length}` : "Готово"}</strong><span>Для эталонной разметки включайте фиксацию только после уверенной ручной проверки жанра.</span></div><button className="primary" onClick={onClose}>Закрыть</button></footer>
  </section></div>;
}
