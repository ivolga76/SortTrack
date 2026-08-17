import { useEffect, useState } from "react";
import type { ScanOptions, ScanPreview, ScanProgress } from "../../../domain/models";

function formatTime(seconds: number | null): string {
  if (seconds === null) return "оценка…";
  if (seconds < 60) return `${seconds} с`;
  const minutes = Math.floor(seconds / 60);
  return seconds < 3600 ? `${minutes} мин ${seconds % 60} с` : `${Math.floor(minutes / 60)} ч ${minutes % 60} мин`;
}

export function ScanDialog({ preview, progress, busy, onRefresh, onConfirm, onPause, onResume, onCancel, onClose }: {
  preview: ScanPreview;
  progress: ScanProgress | null;
  busy: boolean;
  onRefresh: (options: ScanOptions) => void;
  onConfirm: () => void;
  onPause: (jobId: string) => void;
  onResume: (jobId: string) => void;
  onCancel: (jobId: string) => void;
  onClose: () => void;
}) {
  const [recursive, setRecursive] = useState(preview.recursive);
  const [exclusions, setExclusions] = useState(preview.exclusions.join("\n"));
  const [dirty, setDirty] = useState(false);
  useEffect(() => {
    setRecursive(preview.recursive);
    setExclusions(preview.exclusions.join("\n"));
    setDirty(false);
  }, [preview]);
  const options = (): ScanOptions => ({ recursive, exclusions: exclusions.split(/\r?\n/).map((value) => value.trim()).filter(Boolean) });
  const percent = progress?.totalItems ? Math.min(100, Math.round(progress.processedItems / progress.totalItems * 100)) : 0;

  return <div className="copy-overlay" role="presentation"><section className="copy-dialog scan-dialog" role="dialog" aria-modal="true" aria-labelledby="scan-title">
    <header><div><span className="eyebrow">Без изменения файлов</span><h2 id="scan-title">Предпросмотр индексирования</h2><small title={preview.rootPath}>{preview.rootPath}</small></div><button onClick={onClose} disabled={busy} aria-label="Закрыть">×</button></header>
    <div className="scan-body">
      <section className="scan-count"><strong>{preview.discovered}</strong><span>MP3/FLAC найдено с текущими настройками</span></section>
      <label className="scan-recursive"><input type="checkbox" checked={recursive} onChange={(event) => { setRecursive(event.target.checked); setDirty(true); }}/><span><strong>Обходить подпапки</strong><small>Символические ссылки и reparse-переходы не используются.</small></span></label>
      <label className="scan-exclusions"><span><strong>Исключения</strong><small>По одному относительному пути на строку, например `archive/old`.</small></span><textarea value={exclusions} placeholder={"archive\nsets/old"} onChange={(event) => { setExclusions(event.target.value); setDirty(true); }}/></label>
      {dirty && <div className="scan-warning">Настройки изменены. Обновите оценку перед запуском.</div>}
      {preview.warnings.length > 0 && <section className="scan-warnings"><strong>Недоступные пути: {preview.warnings.length}</strong>{preview.warnings.slice(0, 8).map((warning) => <span key={`${warning.path}-${warning.reason}`} title={warning.reason}>{warning.path || warning.reason}</span>)}</section>}
      {progress && <section className="scan-progress"><header><strong>{progress.state === "paused" ? "Пауза" : progress.state === "cancelling" ? "Безопасная отмена…" : "Индексирование"}</strong><span>{progress.processedItems} / {progress.totalItems} · {percent}%</span></header><div><i style={{ width: `${percent}%` }}/></div><aside><span>{progress.itemsPerSecond ? `${progress.itemsPerSecond.toFixed(1)} файла/с` : "Расчёт скорости…"}</span><span>Осталось: {formatTime(progress.etaSeconds)}</span><span>Прошло: {formatTime(progress.elapsedSeconds)}</span></aside><footer>{progress.state === "running" && <button onClick={() => onPause(progress.id)}>Пауза</button>}{progress.state === "paused" && <button onClick={() => onResume(progress.id)}>Продолжить</button>}{(["running", "paused"] as ScanProgress["state"][]).includes(progress.state) && <button className="danger" onClick={() => onCancel(progress.id)}>Отменить безопасно</button>}</footer></section>}
      <div className="copy-safety"><strong>Что произойдёт после подтверждения</strong><span>SortTrack прочитает основные теги и параметры аудио и обновит локальный индекс. MP3/FLAC не перемещаются, не переименовываются и не изменяются.</span></div>
    </div>
    <footer><button onClick={onClose} disabled={busy}>Закрыть</button><button disabled={busy} onClick={() => onRefresh(options())}>{busy ? "Проверка…" : "Обновить оценку"}</button><button className="primary" disabled={busy || dirty} onClick={onConfirm}>{busy ? "Индексирование…" : `Начать · ${preview.discovered}`}</button></footer>
  </section></div>;
}
