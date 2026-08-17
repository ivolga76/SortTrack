import { useEffect, useState } from "react";
import type { MetadataPreview, MetadataValues, TrackMetadataDetail } from "../../../domain/models";

const textFields: Array<{ key: Exclude<keyof MetadataValues, "bpm" | "camelot">; label: string; wide?: boolean }> = [
  { key: "title", label: "Название" },
  { key: "artist", label: "Исполнитель" },
  { key: "album", label: "Альбом" },
  { key: "albumArtist", label: "Исполнитель альбома" },
  { key: "trackNumber", label: "Номер трека" },
  { key: "discNumber", label: "Номер диска" },
  { key: "genre", label: "Жанр в файле" },
  { key: "year", label: "Год" },
  { key: "composer", label: "Композитор" },
  { key: "musicalKey", label: "Тональность" },
  { key: "comment", label: "Комментарий", wide: true },
];

export function MetadataDialog({ detail, onClose, onPreview, onCommit, onSaved }: {
  detail: TrackMetadataDetail;
  onClose: () => void;
  onPreview: (values: MetadataValues) => Promise<MetadataPreview>;
  onCommit: (preview: MetadataPreview) => Promise<TrackMetadataDetail>;
  onSaved: (detail: TrackMetadataDetail) => void;
}) {
  const [values, setValues] = useState<MetadataValues>(() => structuredClone(detail.values));
  const [preview, setPreview] = useState<MetadataPreview | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setValues(structuredClone(detail.values));
    setPreview(null);
  }, [detail]);

  const setText = (key: Exclude<keyof MetadataValues, "bpm" | "camelot">, value: string) => {
    setValues((current) => ({ ...current, [key]: value || null }));
    setPreview(null);
  };
  const runPreview = async () => {
    setBusy(true);
    setError(null);
    try {
      const result = await onPreview(values);
      setValues(result.after);
      setPreview(result);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  };
  const commit = async () => {
    if (!preview) return;
    setBusy(true);
    setError(null);
    try {
      onSaved(await onCommit(preview));
    } catch (reason) {
      setError(String(reason));
      setPreview(null);
    } finally {
      setBusy(false);
    }
  };
  const restoreDraft = (previous: MetadataValues) => {
    setValues(structuredClone(previous));
    setPreview(null);
    setError(null);
  };

  return <div className="dialog-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget && !busy) onClose(); }}>
    <section className="metadata-dialog" role="dialog" aria-modal="true" aria-labelledby="metadata-title">
      <header>
        <div><span>Безопасная запись в файл</span><h2 id="metadata-title">Метаданные трека</h2><small title={detail.path}>{detail.format} · {detail.path}</small></div>
        <button className="dialog-close" onClick={onClose} disabled={busy} aria-label="Закрыть">×</button>
      </header>

      <div className="metadata-body">
        <div className="metadata-main">
          <div className="metadata-warning"><strong>Исходник защищён</strong><span>Сначала будет проверена отдельная копия. Запись начнётся только после просмотра изменений.</span></div>
          <div className="metadata-form">
            {textFields.map((field) => <label key={field.key} className={field.wide ? "wide" : ""}>
              <span>{field.label}</span>
              {field.wide
                ? <textarea value={values[field.key] ?? ""} onChange={(event) => setText(field.key, event.target.value)} />
                : <input value={values[field.key] ?? ""} onChange={(event) => setText(field.key, event.target.value)} />}
            </label>)}
            <label><span>BPM</span><input type="number" min="20" max="400" step="0.01" value={values.bpm ?? ""} onChange={(event) => { setValues((current) => ({ ...current, bpm: event.target.value ? Number(event.target.value) : null })); setPreview(null); }} /></label>
            <label><span>Camelot</span><input value={values.camelot ?? "Будет вычислен"} readOnly /></label>
          </div>

          {error && <div className="metadata-error" role="alert">{error}</div>}
          {preview && <div className="metadata-preview">
            <header><strong>Проверьте изменения</strong><span>{preview.changes.length} полей</span></header>
            {preview.changes.map((change) => <div className="metadata-change" key={change.field}>
              <strong>{change.label}</strong><span>{change.before || "—"}</span><b>→</b><span>{change.after || "—"}</span>
            </div>)}
          </div>}
        </div>

        <aside className="metadata-history">
          <div><strong>История тегов</strong><span>{detail.history.length} записей</span></div>
          {detail.history.length === 0 && <p>История появится после первой подтверждённой записи.</p>}
          {detail.history.map((entry) => <article key={entry.id}>
            <time>{new Date(entry.createdAt).toLocaleString("ru-RU")}</time>
            <strong>{entry.after.title || "Без названия"}</strong>
            <span>{entry.after.artist || "Исполнитель не указан"}</span>
            <button onClick={() => restoreDraft(entry.before)}>Подготовить восстановление</button>
          </article>)}
          <div className="rating-history-heading"><strong>История рейтинга</strong><span>{detail.ratingHistory.length} записей</span></div>
          {detail.ratingHistory.length === 0 && <p>Рейтинг этого трека ещё не изменялся.</p>}
          {detail.ratingHistory.map((entry) => <article className="rating-history-entry" key={entry.id}>
            <time>{new Date(entry.createdAt).toLocaleString("ru-RU")}</time>
            <strong>{entry.beforeRating || "Без рейтинга"} → {entry.afterRating || "Без рейтинга"}</strong>
          </article>)}
        </aside>
      </div>

      <footer>
        <span>{preview ? "Файл ещё не изменён" : "Подготовьте предварительный просмотр"}</span>
        <button className="secondary" onClick={onClose} disabled={busy}>Отмена</button>
        {!preview && <button className="primary" onClick={() => void runPreview()} disabled={busy}>{busy ? "Проверка…" : "Показать изменения"}</button>}
        {preview && <button className="primary danger-confirm" onClick={() => void commit()} disabled={busy}>{busy ? "Безопасная запись…" : "Подтвердить запись"}</button>}
      </footer>
    </section>
  </div>;
}
