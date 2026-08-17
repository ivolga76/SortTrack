import { useEffect, useMemo, useState } from "react";
import type { CopyConflictAction, CopyPlan, CopyReport, CopyResolution } from "../../../domain/models";

interface CopyDialogProps {
  plan: CopyPlan;
  report: CopyReport | null;
  busy: boolean;
  onConfirm: (resolutions: CopyResolution[]) => void;
  onClose: () => void;
}

function bytes(value: number): string {
  if (value < 1024) return `${value} Б`;
  if (value < 1024 ** 2) return `${(value / 1024).toFixed(1)} КБ`;
  if (value < 1024 ** 3) return `${(value / 1024 ** 2).toFixed(1)} МБ`;
  return `${(value / 1024 ** 3).toFixed(2)} ГБ`;
}

const statusLabel: Record<string, string> = {
  ready: "Копировать",
  identical: "Уже существует",
  conflict: "Конфликт",
  missing: "Источник отсутствует",
  invalid: "Недопустимое имя",
  permission_error: "Нет доступа",
};

export function CopyDialog({ plan, report, busy, onConfirm, onClose }: CopyDialogProps) {
  const [resolutions, setResolutions] = useState<Record<string, CopyConflictAction | "">>({});
  useEffect(() => setResolutions({}), [plan.operationId]);
  const conflicts = useMemo(() => plan.items.filter((item) => item.status === "conflict"), [plan]);
  const unresolved = conflicts.some((item) => !resolutions[item.id]);
  const insufficientSpace = plan.availableBytes > 0 && plan.requiredBytes > plan.availableBytes;
  const applyAll = (action: CopyConflictAction) => setResolutions(Object.fromEntries(conflicts.map((item) => [item.id, action])));

  return (
    <div className="copy-overlay" role="presentation">
      <section className="copy-dialog" role="dialog" aria-modal="true" aria-label="Безопасное копирование плейлиста">
        <header>
          <div><span className="eyebrow">Предварительный план</span><h2>{report ? "Отчёт о копировании" : "Копирование файлов плейлиста"}</h2></div>
          <button onClick={onClose} aria-label="Закрыть">×</button>
        </header>
        {!report ? <>
          <div className="copy-destination"><span>Папка назначения</span><strong title={plan.destinationRoot}>{plan.destinationRoot}</strong></div>
          <div className="copy-stats">
            <div><strong>{plan.totalFiles}</strong><span>файлов</span></div>
            <div><strong>{bytes(plan.totalBytes)}</strong><span>общий размер</span></div>
            <div className={insufficientSpace ? "danger" : ""}><strong>{bytes(plan.availableBytes)}</strong><span>доступно</span></div>
            <div><strong>{conflicts.length}</strong><span>конфликтов</span></div>
          </div>
          {conflicts.length > 0 && <div className="copy-bulk"><span>Применить ко всем конфликтам:</span><button onClick={() => applyAll("skip")}>Пропустить</button><button onClick={() => applyAll("rename")}>Создать копии с новым именем</button></div>}
          <div className="copy-items">
            {plan.items.map((item) => <article className={`copy-item ${item.status}`} key={item.id}>
              <span className="copy-status">{statusLabel[item.status] ?? item.status}</span>
              <span className="copy-track"><strong>{item.trackTitle}</strong><small title={item.destinationPath}>{item.destinationPath}</small><em>{item.reason}</em></span>
              <span className="copy-size">{bytes(item.sizeBytes)}</span>
              {item.status === "conflict" && <select aria-label={`Решение для ${item.trackTitle}`} value={resolutions[item.id] ?? ""} onChange={(event) => setResolutions((current) => ({ ...current, [item.id]: event.target.value as CopyConflictAction }))}>
                <option value="">Выберите решение</option>
                <option value="skip">Пропустить</option>
                <option value="rename">Копировать с новым именем</option>
              </select>}
            </article>)}
          </div>
          <footer><div className="copy-safety"><strong>Перезапись запрещена</strong><span>Сначала временный файл, затем проверка размера и SHA-256.</span></div><button className="secondary" onClick={onClose}>Отмена</button><button className="primary" disabled={busy || unresolved || insufficientSpace} onClick={() => onConfirm(Object.entries(resolutions).filter(([, action]) => action).map(([itemId, action]) => ({ itemId, action: action as CopyConflictAction })))}>{busy ? "Копирование…" : `Подтвердить · ${bytes(plan.requiredBytes)}`}</button></footer>
        </> : <>
          <div className="copy-stats report">
            <div><strong>{report.copied}</strong><span>скопировано</span></div>
            <div><strong>{report.usedExisting}</strong><span>уже было</span></div>
            <div><strong>{report.skipped}</strong><span>пропущено</span></div>
            <div className={report.failed ? "danger" : ""}><strong>{report.failed}</strong><span>ошибок</span></div>
          </div>
          <div className="copy-items report-items">{report.items.map((item, index) => <article className={`copy-item ${item.result}`} key={`${item.destinationPath}-${index}`}><span className="copy-status">{item.result === "copied" ? "Готово" : item.result === "used_existing" ? "Использован" : item.result === "skipped" ? "Пропущен" : "Ошибка"}</span><span className="copy-track"><strong>{item.trackTitle}</strong><small>{item.destinationPath}</small><em>{item.reason}</em></span></article>)}</div>
          <footer><div className="copy-safety"><strong>Скопировано {bytes(report.bytesCopied)}</strong><span>Исходные файлы не изменялись.</span></div><button className="primary" onClick={onClose}>Закрыть отчёт</button></footer>
        </>}
      </section>
    </div>
  );
}
