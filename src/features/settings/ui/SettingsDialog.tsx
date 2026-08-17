import { useEffect, useState } from "react";
import type { BackupReport, DiagnosticsExportReport, DiagnosticsPreview, LibraryRootSettings, RestorePreview, RuntimeProfile } from "../../../domain/models";
import type { LibraryRootRemovalPreview } from "../../../domain/models";

interface Props {
  desktop: boolean;
  busy: boolean;
  report: BackupReport | null;
  restorePreview: RestorePreview | null;
  runtimeProfile: RuntimeProfile | null;
  libraryRoots: LibraryRootSettings[];
  libraryRootBusy: boolean;
  libraryRootRemoval: LibraryRootRemovalPreview | null;
  diagnosticsPreview: DiagnosticsPreview | null;
  diagnosticsReport: DiagnosticsExportReport | null;
  diagnosticsBusy: boolean;
  onBackup: () => void;
  onPreviewRestore: () => void;
  onRestore: () => void;
  onPreviewLibraryRoot: (root: LibraryRootSettings) => void;
  onSetLibraryRootEnabled: (root: LibraryRootSettings, enabled: boolean) => void;
  onPreviewLibraryRootRemoval: (root: LibraryRootSettings) => void;
  onCancelLibraryRootRemoval: () => void;
  onConfirmLibraryRootRemoval: () => void;
  onPreviewDiagnostics: (redactPaths: boolean) => void;
  onExportDiagnostics: (redactPaths: boolean) => void;
  onOpenLogs: () => void;
  onClearDiagnostics: () => void;
  onClose: () => void;
}

function formatBytes(value: number): string {
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} КБ`;
  return `${(value / 1024 / 1024).toFixed(1)} МБ`;
}

export function SettingsDialog({ desktop, busy, report, restorePreview, runtimeProfile, libraryRoots, libraryRootBusy, libraryRootRemoval, diagnosticsPreview, diagnosticsReport, diagnosticsBusy, onBackup, onPreviewRestore, onRestore, onPreviewLibraryRoot, onSetLibraryRootEnabled, onPreviewLibraryRootRemoval, onCancelLibraryRootRemoval, onConfirmLibraryRootRemoval, onPreviewDiagnostics, onExportDiagnostics, onOpenLogs, onClearDiagnostics, onClose }: Props) {
  const [confirmation, setConfirmation] = useState("");
  const [clearConfirmation, setClearConfirmation] = useState("");
  const [redactPaths, setRedactPaths] = useState(true);
  const [removeRootConfirmation, setRemoveRootConfirmation] = useState("");
  useEffect(() => setConfirmation(""), [restorePreview?.manifestSha256]);
  useEffect(() => setRemoveRootConfirmation(""), [libraryRootRemoval?.rootId]);
  return <div className="copy-overlay settings-overlay" role="presentation"><section className="copy-dialog settings-dialog" role="dialog" aria-modal="true" aria-labelledby="settings-title">
    <header><div><span className="eyebrow">Локальные данные</span><h2 id="settings-title">Настройки и резервная копия</h2></div><button onClick={onClose} disabled={busy} aria-label="Закрыть настройки">×</button></header>
    {runtimeProfile && <section className="backup-card"><header><div><strong>{runtimeProfile.mode === "portable" ? "Portable-режим" : runtimeProfile.mode === "installed" ? "Установленная версия" : "Демонстрационный режим"}</strong><span>{runtimeProfile.mode === "portable" ? "Данные изолированы рядом с приложением" : runtimeProfile.mode === "installed" ? "Данные хранятся в системной папке приложения" : "Изменения существуют только до обновления страницы"}</span></div></header><code>{runtimeProfile.dataRoot}</code></section>}
    <section className="backup-card library-roots-card">
      <header><div><strong>Папки библиотеки</strong><span>{libraryRootBusy ? "Проверка доступности…" : libraryRoots.length ? `Сохранено: ${libraryRoots.length}` : "Пока нет сохранённых папок"}</span></div></header>
      {libraryRoots.length === 0
        ? <p>После первого подтверждённого сканирования папка и выбранные параметры появятся здесь.</p>
        : <div className="library-root-list">{libraryRoots.map((root) => <article className="library-root-item" key={root.id}>
          <div className="library-root-main"><strong title={root.path}>{root.path}</strong><span>{root.recursive ? "С подпапками" : "Только верхний уровень"} · исключений: {root.exclusions.length} · {root.lastScannedAt ? `проверена ${new Date(root.lastScannedAt).toLocaleString("ru-RU")}` : "ещё не проверялась"}</span>{root.exclusions.length > 0 && <small title={root.exclusions.join(", ")}>Исключения: {root.exclusions.join(", ")}</small>}</div>
          <span className={`library-root-status ${root.status}`}>{root.status === "active" ? "Доступна" : root.status === "unavailable" ? "Недоступна" : "Отключена"}</span>
          <div className="library-root-actions"><button disabled={busy || diagnosticsBusy || !desktop || root.status !== "active"} onClick={() => onPreviewLibraryRoot(root)}>Проверить снова</button><button className={root.status === "active" ? "root-disable" : "root-enable"} disabled={busy || diagnosticsBusy || !desktop} onClick={() => onSetLibraryRootEnabled(root, root.status !== "active")}>{root.status === "active" ? "Отключить" : "Подключить"}</button>{root.status === "disabled" && <button className="root-remove" disabled={busy || diagnosticsBusy || !desktop} onClick={() => onPreviewLibraryRootRemoval(root)}>Убрать из списка</button>}</div>
        </article>)}</div>}
      {libraryRootRemoval && <section className="library-root-removal"><strong>Удалить только регистрацию?</strong><code>{libraryRootRemoval.path}</code><p>{libraryRootRemoval.linkedFiles} записей файлов останутся в библиотеке со всей историей, но больше не будут связаны с этим корнем. MP3/FLAC не удаляются и не изменяются.</p><label><span>Для подтверждения введите <strong>УБРАТЬ ПАПКУ</strong></span><input value={removeRootConfirmation} onChange={(event) => setRemoveRootConfirmation(event.target.value)} disabled={busy} autoComplete="off"/></label><div><button disabled={busy} onClick={onCancelLibraryRootRemoval}>Отмена</button><button className="danger" disabled={busy || removeRootConfirmation !== "УБРАТЬ ПАПКУ"} onClick={onConfirmLibraryRootRemoval}>Удалить регистрацию</button></div></section>}
      <p>Отключение обратимо и меняет только статус в SortTrack. Записи треков и MP3/FLAC не удаляются; после подключения повторная проверка начинается с предварительного результата.</p>
    </section>
    <section className="backup-card"><header><div><strong>Резервная копия библиотеки</strong><span>SQLite snapshot и проверяемый manifest</span></div><button className="primary" disabled={busy || !desktop} onClick={onBackup}>{busy ? "Создание…" : "Создать backup"}</button></header>
      <p>Сохраняются плейлисты, история метаданных, подтверждённые жанры, validation manifests и результаты модели. Исходные MP3/FLAC и пересоздаваемый waveform-кэш не копируются.</p>
      <div className="backup-safety"><strong>Без перезаписи</strong><span>SortTrack создаёт новую папку, согласованный SQLite snapshot, проверяет integrity check и записывает SHA-256 базы и manifest.</span></div>
    </section>
    {report && <section className="backup-report"><header><strong>Резервная копия готова</strong><span>Схема БД v{report.databaseSchemaVersion} · {formatBytes(report.databaseSizeBytes)}</span></header><code>{report.backupPath}</code><div><span>Треков <strong>{report.counts.tracks}</strong></span><span>Файлов <strong>{report.counts.trackFiles}</strong></span><span>Плейлистов <strong>{report.counts.playlists}</strong></span><span>История жанров <strong>{report.counts.genreHistory}</strong></span></div><small>DB SHA-256 {report.databaseSha256}</small><small>Manifest SHA-256 {report.manifestSha256}</small></section>}
    <section className="restore-card"><header><div><strong>Восстановление из backup</strong><span>Сначала проверка, затем отдельное подтверждение</span></div><button disabled={busy || !desktop} onClick={onPreviewRestore}>Выбрать и проверить</button></header><p>Активная база будет заменена только после создания safety backup и перезапуска SortTrack. MP3/FLAC не изменяются.</p></section>
    {restorePreview && <section className="restore-preview"><header><strong>Backup проверен</strong><span>{new Date(restorePreview.createdAt).toLocaleString("ru-RU")} · SortTrack {restorePreview.applicationVersion}</span></header><code>{restorePreview.backupPath}</code><div><span>Схема <strong>v{restorePreview.databaseSchemaVersion}</strong></span><span>Треков <strong>{restorePreview.counts.tracks}</strong></span><span>Плейлистов <strong>{restorePreview.counts.playlists}</strong></span><span>{formatBytes(restorePreview.databaseSizeBytes)}</span></div>{restorePreview.requiresMigration && <div className="restore-warning">После восстановления база будет обновлена до схемы v{restorePreview.currentSchemaVersion}.</div>}<label className="restore-confirm"><span>Для подтверждения введите <strong>ВОССТАНОВИТЬ</strong></span><input value={confirmation} onChange={(event) => setConfirmation(event.target.value)} disabled={busy} autoComplete="off"/></label><button className="danger" disabled={busy || confirmation !== "ВОССТАНОВИТЬ"} onClick={onRestore}>{busy ? "Подготовка…" : "Создать safety backup и перезапустить"}</button><small>Manifest SHA-256 {restorePreview.manifestSha256}</small></section>}
    <section className="backup-card diagnostics-card">
      <header><div><strong>Локальная диагностика</strong><span>JSONL-логи и отчёты о сбоях — без музыкальных файлов и базы</span></div><button disabled={diagnosticsBusy || !desktop} onClick={() => onPreviewDiagnostics(redactPaths)}>Предпросмотр</button></header>
      <label className="diagnostics-redaction"><input type="checkbox" checked={redactPaths} onChange={(event) => setRedactPaths(event.target.checked)}/><span>Маскировать физические пути в экспортируемом пакете</span></label>
      <div className="diagnostics-actions"><button disabled={diagnosticsBusy || !desktop} onClick={onOpenLogs}>Открыть папку логов</button>{diagnosticsPreview && <button className="primary" disabled={diagnosticsBusy || diagnosticsPreview.redactPaths !== redactPaths} onClick={() => onExportDiagnostics(redactPaths)}>Сохранить пакет</button>}</div>
      {diagnosticsPreview && <div className="diagnostics-summary"><span>Логов <strong>{diagnosticsPreview.logFiles}</strong></span><span>Отчётов о сбоях <strong>{diagnosticsPreview.crashReports}</strong></span><span>{formatBytes(diagnosticsPreview.totalBytes)}</span>{diagnosticsPreview.warnings.map((warning) => <small key={warning}>{warning}</small>)}</div>}
      {diagnosticsReport && <div className="backup-report"><strong>Диагностический пакет сохранён</strong><code>{diagnosticsReport.path}</code><small>SHA-256 {diagnosticsReport.sha256}</small></div>}
      <div className="diagnostics-clear"><label><span>Для удаления локальных логов введите <strong>УДАЛИТЬ ЛОГИ</strong></span><input value={clearConfirmation} onChange={(event) => setClearConfirmation(event.target.value)} autoComplete="off"/></label><button className="danger" disabled={diagnosticsBusy || clearConfirmation !== "УДАЛИТЬ ЛОГИ"} onClick={() => { onClearDiagnostics(); setClearConfirmation(""); }}>Удалить логи</button></div>
      <p>SortTrack не отправляет диагностику автоматически. Перед сохранением показывается состав пакета; существующий файл никогда не перезаписывается.</p>
    </section>
    <footer><div className="copy-safety"><strong>Все данные остаются на компьютере</strong><span>Сетевые запросы и автоматическая отправка логов не выполняются.</span></div><button onClick={onClose} disabled={busy || diagnosticsBusy}>Закрыть</button></footer>
  </section></div>;
}
