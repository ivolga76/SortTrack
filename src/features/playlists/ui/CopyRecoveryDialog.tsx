import type { CopyRecoveryOperation, CopyRecoveryReport } from "../../../domain/models";

interface CopyRecoveryDialogProps {
  operation: CopyRecoveryOperation;
  report: CopyRecoveryReport | null;
  busy: boolean;
  onAction: (action: "resume" | "cancel_safe") => void;
  onClose: () => void;
}

const stateLabels: Record<string, string> = {
  planned: "Ожидал запуска",
  copying_to_temp: "Копирование прервано",
  temp_copied: "Временная копия создана",
  verified: "Копия проверена",
  destination_committed: "Итоговый файл создан",
  recovery_required: "Нужна проверка",
};

export function CopyRecoveryDialog({ operation, report, busy, onAction, onClose }: CopyRecoveryDialogProps) {
  return (
    <div className="copy-overlay recovery-overlay" role="presentation">
      <section className="copy-dialog recovery-dialog" role="dialog" aria-modal="true" aria-labelledby="copy-recovery-title">
        <header>
          <div><span className="eyebrow">Защита после сбоя</span><h2 id="copy-recovery-title">{report ? "Отчёт о восстановлении" : "Незавершённое копирование"}</h2></div>
          <button onClick={onClose} disabled={busy} aria-label="Оставить восстановление на потом">×</button>
        </header>
        {!report ? <>
          <div className="recovery-message"><strong>Исходные файлы в безопасности</strong><span>SortTrack обнаружил прерванную операцию. Итоговые файлы будут приняты только после проверки SHA-256.</span></div>
          <div className="copy-destination"><span>Папка назначения</span><strong title={operation.destinationRoot}>{operation.destinationRoot}</strong></div>
          <div className="copy-stats">
            <div><strong>{operation.totalItems}</strong><span>всего</span></div>
            <div><strong>{operation.completedItems}</strong><span>уже завершено</span></div>
            <div className="danger"><strong>{operation.pendingItems}</strong><span>нужно проверить</span></div>
            <div><strong>{operation.createdAt}</strong><span>операция начата</span></div>
          </div>
          <div className="copy-items recovery-items">
            {operation.items.map((item) => <article className="copy-item recovery_required" key={item.id}>
              <span className="copy-status">{stateLabels[item.state] ?? item.state}</span>
              <span className="copy-track"><strong>{item.trackTitle}</strong><small title={item.destinationPath}>{item.destinationPath}</small><em>{item.recommendedAction}</em></span>
            </article>)}
          </div>
          <footer>
            <div className="copy-safety"><strong>Никакой перезаписи</strong><span>Удаляются только служебные файлы, принадлежащие этой операции.</span></div>
            <button className="secondary" onClick={onClose} disabled={busy}>Оставить на потом</button>
            <button className="secondary danger-button" onClick={() => onAction("cancel_safe")} disabled={busy}>{busy ? "Проверка…" : "Безопасно отменить"}</button>
            <button className="primary" onClick={() => onAction("resume")} disabled={busy}>{busy ? "Восстановление…" : "Проверить и продолжить"}</button>
          </footer>
        </> : <>
          <div className="copy-stats report">
            <div><strong>{report.recovered}</strong><span>восстановлено</span></div>
            <div><strong>{report.cancelled}</strong><span>отменено безопасно</span></div>
            <div className={report.failed ? "danger" : ""}><strong>{report.failed}</strong><span>требует внимания</span></div>
          </div>
          <div className="copy-items report-items">
            {report.items.map((item, index) => <article className={`copy-item ${item.result}`} key={`${item.destinationPath}-${index}`}>
              <span className="copy-status">{item.result === "recovered" ? "Готово" : item.result === "cancelled" ? "Отменено" : "Не решено"}</span>
              <span className="copy-track"><strong>{item.trackTitle}</strong><small title={item.destinationPath}>{item.destinationPath}</small><em>{item.reason}</em></span>
            </article>)}
          </div>
          <footer><div className="copy-safety"><strong>{report.failed ? "Некоторые файлы оставлены без изменений" : "Восстановление завершено"}</strong><span>Исходные файлы не изменялись.</span></div><button className="primary" onClick={onClose}>Закрыть отчёт</button></footer>
        </>}
      </section>
    </div>
  );
}
