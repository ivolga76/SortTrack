import type { ScanResult } from "../../../domain/models";

export function ScanReportDialog({ report, onClose }: { report: ScanResult; onClose: () => void }) {
  const stats = [
    ["Обработано", report.discovered], ["Добавлено", report.added], ["Обновлено", report.updated],
    ["Перемещено", report.moved], ["Вернулось", report.restored], ["Пропущено", report.skipped],
    ["Отсутствует", report.missing], ["Ошибки", report.errors.length],
  ] as const;
  return <div className="copy-overlay" role="presentation"><section className="copy-dialog scan-report-dialog" role="dialog" aria-modal="true" aria-labelledby="scan-report-title">
    <header><div><span className="eyebrow">Локальный отчёт</span><h2 id="scan-report-title">{report.cancelled ? "Индексирование отменено безопасно" : "Индексирование завершено"}</h2><small title={report.rootPath}>{report.rootPath}</small></div><button onClick={onClose} aria-label="Закрыть">×</button></header>
    <div className="scan-report-body">
      {report.cancelled && <div className="scan-warning">Уже обработанные записи сохранены. Повторный scan продолжит индексирование без создания дублей.</div>}
      <section className="scan-report-stats">{stats.map(([label, value]) => <div key={label}><strong>{value}</strong><span>{label}</span></div>)}</section>
      <section className="scan-report-errors"><header><strong>Проблемные файлы</strong><span>{report.errors.length}</span></header>{report.errors.length === 0 ? <p>Ошибок чтения не зафиксировано.</p> : <div>{report.errors.map((error, index) => <article key={`${error.path}-${index}`}><strong title={error.path}>{error.path || "Путь недоступен"}</strong><span>{error.reason}</span></article>)}</div>}</section>
      <div className="copy-safety"><strong>Музыкальные файлы не изменялись</strong><span>Отчёт сформирован по чтению MP3/FLAC и обновлению локального индекса SQLite.</span></div>
    </div>
    <footer><button className="primary" onClick={onClose}>Закрыть отчёт</button></footer>
  </section></div>;
}
