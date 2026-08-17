import type { RelinkPreview } from "../../../domain/models";

function bytes(value: number) {
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} КБ`;
  return `${(value / 1024 / 1024).toFixed(1)} МБ`;
}

export function RelinkDialog({ preview, busy, onCommit, onClose }: {
  preview: RelinkPreview;
  busy: boolean;
  onCommit: () => void;
  onClose: () => void;
}) {
  return (
    <div className="dialog-backdrop" role="presentation">
      <section className="metadata-dialog relink-dialog" role="dialog" aria-modal="true" aria-labelledby="relink-title">
        <header>
          <div><span>Безопасная перепривязка</span><h2 id="relink-title">{preview.title}</h2><small>{preview.artist || "Исполнитель не указан"}</small></div>
          <button className="dialog-close" onClick={onClose} disabled={busy} aria-label="Закрыть">×</button>
        </header>
        <div className="relink-body">
          <div className="relink-path old"><span>Старая ссылка</span><strong>{preview.oldPath}</strong><small>Файл отсутствует, запись трека сохранена.</small></div>
          <div className="relink-arrow">↓</div>
          <div className="relink-path next"><span>Выбранный файл · {preview.format} · {bytes(preview.sizeBytes)}</span><strong>{preview.newPath}</strong><small>Файл не перемещается и не изменяется.</small></div>
          {preview.willRegisterLibraryRoot && <div className="metadata-warning"><strong>Новая папка</strong><span>SortTrack добавит папку «{preview.newLibraryRoot}» в список библиотеки.</span></div>}
          {preview.warning && <div className="metadata-error">{preview.warning}</div>}
          <p className="relink-preserved">Будут сохранены плейлисты, подтверждённый жанр, заметки и история трека. Результаты аудиоанализа выбранного файла будут рассчитаны заново.</p>
        </div>
        <footer><span>Перед записью размер, дата, идентификатор и состояние файла будут проверены повторно.</span><button className="secondary" onClick={onClose} disabled={busy}>Отмена</button><button className="primary danger-confirm" onClick={onCommit} disabled={busy}>{busy ? "Проверка…" : "Перепривязать запись"}</button></footer>
      </section>
    </div>
  );
}
