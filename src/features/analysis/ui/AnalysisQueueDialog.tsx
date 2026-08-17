import type { AnalysisJob } from "../../../domain/models";

interface AnalysisQueueDialogProps {
  jobs: AnalysisJob[];
  selectionCount: number;
  busy: boolean;
  onStart: () => void;
  onResume: (jobId: string) => void;
  onPause: (jobId: string) => void;
  onCancel: (jobId: string) => void;
  onClose: () => void;
}

const stateLabels: Record<AnalysisJob["state"], string> = { queued: "В очереди", running: "Анализ", paused: "Пауза", completed: "Готово", failed: "Завершено с ошибками", cancelled: "Отменено", recovery_required: "Можно продолжить" };
const bytes = (value: number) => value < 1024 ** 2 ? `${Math.round(value / 1024)} КБ` : `${(value / 1024 ** 2).toFixed(1)} МБ`;

export function AnalysisQueueDialog({ jobs, selectionCount, busy, onStart, onResume, onPause, onCancel, onClose }: AnalysisQueueDialogProps) {
  return <div className="copy-overlay analysis-overlay" role="presentation"><section className="copy-dialog analysis-dialog" role="dialog" aria-modal="true" aria-labelledby="analysis-title">
    <header><div><span className="eyebrow">Локальная обработка</span><h2 id="analysis-title">Очередь BPM и тональности</h2></div><button onClick={onClose} aria-label="Закрыть очередь">×</button></header>
    <div className="recovery-message"><strong>Аудио остаётся на компьютере</strong><span>MP3/FLAC декодируются локально. Результат хранится отдельно от тегов и не записывается в файл автоматически.</span></div>
    <div className="analysis-start"><div><strong>{selectionCount} треков подготовлено</strong><span>До 240 секунд аудио на трек · один файл одновременно</span></div><button className="primary" onClick={onStart} disabled={busy || selectionCount === 0}>{busy ? "Добавление…" : "Добавить в очередь"}</button></div>
    <div className="analysis-jobs">{jobs.map((job) => {
      const progress = job.totalItems ? Math.min(100, Math.round(job.processedItems / job.totalItems * 100)) : 0;
      return <article className={`analysis-job ${job.state}`} key={job.id}>
        <header><span className="copy-status">{stateLabels[job.state]}</span><strong>{job.processedItems} / {job.totalItems}</strong><small>{bytes(job.processedBytes)} / {bytes(job.totalBytes)}</small></header>
        <div className="analysis-progress"><i style={{ width: `${progress}%` }}/></div>
        <footer><span>{job.failedItems ? `Ошибок: ${job.failedItems}` : job.state === "completed" ? "BPM, key и Camelot сохранены в локальной базе" : `Выполнено ${progress}%`}</span>
          {job.state === "running" && <button onClick={() => onPause(job.id)}>Пауза</button>}
          {(["queued", "paused", "recovery_required"] as AnalysisJob["state"][]).includes(job.state) && <button onClick={() => onResume(job.id)}>{job.state === "recovery_required" ? "Восстановить" : "Продолжить"}</button>}
          {(["queued", "running", "paused", "recovery_required"] as AnalysisJob["state"][]).includes(job.state) && <button className="danger-button" onClick={() => onCancel(job.id)}>Отменить</button>}
        </footer>
      </article>;
    })}{jobs.length === 0 && <div className="analysis-empty"><strong>Очередь пока пуста</strong><span>Выберите треки и запустите локальный анализ.</span></div>}</div>
    <footer><div className="copy-safety"><strong>Алгоритм sorttrack-dsp-1</strong><span>Низкий score не считается подтверждённым результатом и может быть исправлен вручную.</span></div><button className="primary" onClick={onClose}>Закрыть</button></footer>
  </section></div>;
}
