import type { DuplicateGroup, DuplicateScanResult } from "../../../domain/models";
import { Icon } from "../../../shared/ui/Icon";

function bytes(value: number): string {
  if (value < 1024 ** 2) return `${(value / 1024).toFixed(1)} КБ`;
  if (value < 1024 ** 3) return `${(value / 1024 ** 2).toFixed(1)} МБ`;
  return `${(value / 1024 ** 3).toFixed(2)} ГБ`;
}

export function DuplicatesView({ groups, lastScan, busy, onScan }: {
  groups: DuplicateGroup[];
  lastScan: DuplicateScanResult | null;
  busy: boolean;
  onScan: () => void;
}) {
  const redundantFiles = groups.reduce((sum, group) => sum + Math.max(0, group.files.length - 1), 0);
  const redundantBytes = groups.reduce((sum, group) => sum + group.sizeBytes * Math.max(0, group.files.length - 1), 0);
  return (
    <section className="duplicates-page">
      <header className="duplicates-heading">
        <div><span className="eyebrow">SHA-256 всего файла</span><h1>Точные дубликаты <em>{groups.length}</em></h1><p>Только полностью одинаковые файлы. Программа показывает рекомендации, но ничего не удаляет.</p></div>
        <button className="primary" onClick={onScan} disabled={busy}><Icon name="duplicate"/>{busy ? "Вычисление SHA-256…" : "Найти дубликаты"}</button>
      </header>
      <div className="duplicate-summary">
        <div><strong>{groups.length}</strong><span>групп</span></div>
        <div><strong>{redundantFiles}</strong><span>лишних копий</span></div>
        <div><strong>{bytes(redundantBytes)}</strong><span>занято копиями</span></div>
        <div><strong>{lastScan ? `${lastScan.cachedFiles} / ${lastScan.candidateFiles}` : "—"}</strong><span>хэшей из кэша</span></div>
      </div>
      {lastScan && <div className="duplicate-scan-note"><span>Вычислено: {lastScan.hashedFiles}</span><span>Из кэша: {lastScan.cachedFiles}</span><span className={lastScan.failedFiles ? "danger" : ""}>Ошибок чтения: {lastScan.failedFiles}</span></div>}
      <div className="duplicate-groups">
        {groups.map((group, groupIndex) => <article className="duplicate-group" key={group.sha256}>
          <header><div><span>Группа {groupIndex + 1}</span><strong>{group.files[0]?.artist || "Неизвестный исполнитель"} — {group.files[0]?.title || "Без названия"}</strong></div><span className="duplicate-size">{bytes(group.sizeBytes)} · {group.files.length} файла</span><code title={group.sha256}>SHA-256 {group.sha256.slice(0, 12)}…</code></header>
          <div className="duplicate-recommendation"><strong>Рекомендация</strong><span>{group.recommendationReason}</span><em>Удаление не выполняется</em></div>
          <div className="duplicate-files">{group.files.map((file) => {
            const recommended = file.fileId === group.recommendedFileId;
            return <div className={`duplicate-file ${recommended ? "recommended" : ""}`} key={file.fileId}>
              <span className={`format ${file.format.toLowerCase()}`}>{file.format}</span>
              <span className="duplicate-file-main"><strong>{file.title}</strong><small title={file.path}>{file.path}</small></span>
              <span className="duplicate-tags"><strong>{file.metadataCompleteness}%</strong><small>теги</small></span>
              <span className={`integrity ${file.availabilityStatus}`}>{file.availabilityStatus === "available" ? "Целостен" : "Есть проблема"}</span>
              {recommended ? <span className="recommended-badge">Рекомендуется оставить</span> : <span className="duplicate-passive">Точная копия</span>}
            </div>;
          })}</div>
        </article>)}
        {groups.length === 0 && <div className="duplicate-empty"><Icon name="duplicate" size={32}/><strong>Точные дубликаты пока не найдены</strong><span>Запустите анализ. Сначала будут выбраны только файлы одинакового размера, затем для них вычислится SHA-256.</span><button onClick={onScan} disabled={busy}>Начать анализ</button></div>}
      </div>
    </section>
  );
}
