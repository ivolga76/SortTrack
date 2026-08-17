import { useState } from "react";
import type { BulkMetadataCommitItem, BulkMetadataPreview, BulkMetadataReport, EditableMetadataField, MetadataValues } from "../../../domain/models";

const emptyValues: MetadataValues = { title: null, artist: null, album: null, albumArtist: null, trackNumber: null, discNumber: null, genre: null, year: null, composer: null, comment: null, bpm: null, musicalKey: null, camelot: null };
const fields: Array<{ key: EditableMetadataField; label: string; numeric?: boolean }> = [
  { key: "title", label: "Название" }, { key: "artist", label: "Исполнитель" },
  { key: "album", label: "Альбом" }, { key: "albumArtist", label: "Исполнитель альбома" },
  { key: "genre", label: "Жанр в файле" }, { key: "year", label: "Год" },
  { key: "trackNumber", label: "Номер трека" }, { key: "discNumber", label: "Номер диска" },
  { key: "composer", label: "Композитор" }, { key: "comment", label: "Комментарий" },
  { key: "bpm", label: "BPM", numeric: true }, { key: "musicalKey", label: "Тональность" },
];

export function BulkMetadataDialog({ trackIds, onClose, onPreview, onCommit, onSaved }: {
  trackIds: string[];
  onClose: () => void;
  onPreview: (fields: EditableMetadataField[], values: MetadataValues) => Promise<BulkMetadataPreview>;
  onCommit: (items: BulkMetadataCommitItem[]) => Promise<BulkMetadataReport>;
  onSaved: (report: BulkMetadataReport) => void;
}) {
  const [selectedFields, setSelectedFields] = useState<Set<EditableMetadataField>>(new Set());
  const [values, setValues] = useState<MetadataValues>(emptyValues);
  const [preview, setPreview] = useState<BulkMetadataPreview | null>(null);
  const [report, setReport] = useState<BulkMetadataReport | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const resetResult = () => { setPreview(null); setReport(null); setError(null); };
  const toggleField = (field: EditableMetadataField, checked: boolean) => {
    setSelectedFields((current) => { const next = new Set(current); checked ? next.add(field) : next.delete(field); return next; });
    resetResult();
  };
  const setValue = (field: EditableMetadataField, value: string) => {
    setValues((current) => ({ ...current, [field]: field === "bpm" ? (value ? Number(value) : null) : (value || null) }));
    resetResult();
  };
  const runPreview = async () => {
    setBusy(true); setError(null);
    try { setPreview(await onPreview([...selectedFields], values)); }
    catch (reason) { setError(String(reason)); }
    finally { setBusy(false); }
  };
  const commit = async () => {
    if (!preview) return;
    const items = preview.items.flatMap((item) => item.status === "ready" && item.expectedFileState && item.after
      ? [{ trackId: item.trackId, expectedFileState: item.expectedFileState, values: item.after }]
      : []);
    setBusy(true); setError(null);
    try { const result = await onCommit(items); setReport(result); onSaved(result); }
    catch (reason) { setError(String(reason)); }
    finally { setBusy(false); }
  };

  return <div className="dialog-backdrop" role="presentation">
    <section className="metadata-dialog bulk-metadata-dialog" role="dialog" aria-modal="true" aria-labelledby="bulk-metadata-title">
      <header><div><span>Пакетная операция</span><h2 id="bulk-metadata-title">Массовое редактирование</h2><small>Выбрано треков: {trackIds.length}. Предел одной операции — 1000 файлов.</small></div><button className="dialog-close" onClick={onClose} disabled={busy} aria-label="Закрыть">×</button></header>
      <div className="bulk-metadata-body">
        {!report && <section className="bulk-fields">
          <div className="metadata-warning"><strong>Только отмеченные поля</strong><span>Пустое отмеченное поле будет удалено из тегов. Неотмеченные поля останутся без изменений.</span></div>
          <div className="bulk-field-grid">
            {fields.map((field) => <label className={selectedFields.has(field.key) ? "enabled" : ""} key={field.key}>
              <input type="checkbox" checked={selectedFields.has(field.key)} onChange={(event) => toggleField(field.key, event.target.checked)} aria-label={`Изменять поле ${field.label}`} />
              <span>{field.label}</span>
              <input aria-label={`Новое значение: ${field.label}`} disabled={!selectedFields.has(field.key)} type={field.numeric ? "number" : "text"} min={field.numeric ? 20 : undefined} max={field.numeric ? 400 : undefined} step={field.numeric ? .01 : undefined} value={values[field.key] ?? ""} onChange={(event) => setValue(field.key, event.target.value)} placeholder="Очистить" />
            </label>)}
          </div>
        </section>}

        {(preview || report) && <section className="bulk-results">
          <div className="bulk-result-summary">
            {preview && <><div><strong>{preview.readyFiles}</strong><span>готово</span></div><div><strong>{preview.skippedFiles}</strong><span>без изменений</span></div><div className={preview.failedFiles ? "danger" : ""}><strong>{preview.failedFiles}</strong><span>ошибок</span></div></>}
            {report && <><div><strong>{report.savedFiles}</strong><span>сохранено</span></div><div className={report.failedFiles ? "danger" : ""}><strong>{report.failedFiles}</strong><span>ошибок</span></div></>}
          </div>
          <div className="bulk-result-items">
            {(report?.items ?? preview?.items ?? []).map((item) => <article className={item.status} key={item.trackId}>
              <span className="bulk-result-status">{item.status === "ready" ? "Готов" : item.status === "saved" ? "Сохранён" : item.status === "skipped" ? "Без изменений" : "Ошибка"}</span>
              <div><strong>{item.title}</strong><small title={item.path}>{item.path || item.trackId}</small><em>{item.reason}</em></div>
              {"changes" in item && <b>{item.changes.length} полей</b>}
            </article>)}
          </div>
        </section>}
        {error && <div className="metadata-error" role="alert">{error}</div>}
      </div>
      <footer><span>{report ? "Пакетная операция завершена" : preview ? "Файлы ещё не изменены" : "Сначала отметьте поля и проверьте preview"}</span><button className="secondary" onClick={onClose} disabled={busy}>{report ? "Закрыть" : "Отмена"}</button>{!preview && !report && <button className="primary" disabled={busy || selectedFields.size === 0} onClick={() => void runPreview()}>{busy ? "Проверка…" : "Показать preview"}</button>}{preview && !report && <button className="primary danger-confirm" disabled={busy || preview.readyFiles === 0} onClick={() => void commit()}>{busy ? "Запись…" : `Записать ${preview.readyFiles} файлов`}</button>}</footer>
    </section>
  </div>;
}
