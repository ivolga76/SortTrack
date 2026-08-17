import { useEffect, useRef, useState } from "react";
import type { BackupReport, DiagnosticsExportReport, DiagnosticsPreview, RestorePreview, RuntimeProfile } from "../../../domain/models";
import type { SettingsPort } from "../../../shared/api/ports";

type SettingsCallbacks = {
  reportNotice: (message: string | null) => void;
  refreshLibraryRoots: () => Promise<unknown>;
  cancelLibraryRootRemoval: () => void;
};

export function useSettingsManager(gateway: SettingsPort, callbacks: SettingsCallbacks) {
  const callbacksRef = useRef(callbacks);
  const [open, setOpen] = useState(false);
  const [backupBusy, setBackupBusy] = useState(false);
  const [backupReport, setBackupReport] = useState<BackupReport | null>(null);
  const [restorePreview, setRestorePreview] = useState<RestorePreview | null>(null);
  const [runtimeProfile, setRuntimeProfile] = useState<RuntimeProfile | null>(null);
  const [diagnosticsPreview, setDiagnosticsPreview] = useState<DiagnosticsPreview | null>(null);
  const [diagnosticsReport, setDiagnosticsReport] = useState<DiagnosticsExportReport | null>(null);
  const [diagnosticsBusy, setDiagnosticsBusy] = useState(false);

  useEffect(() => { callbacksRef.current = callbacks; }, [callbacks]);
  useEffect(() => {
    void gateway.getRuntimeProfile()
      .then(setRuntimeProfile)
      .catch((error) => callbacksRef.current.reportNotice(`Не удалось определить режим хранения: ${String(error)}`));
  }, [gateway]);
  const notify = (message: string | null) => callbacksRef.current.reportNotice(message);

  const show = () => {
    setBackupReport(null);
    setRestorePreview(null);
    setDiagnosticsPreview(null);
    setDiagnosticsReport(null);
    callbacksRef.current.cancelLibraryRootRemoval();
    setOpen(true);
    void callbacksRef.current.refreshLibraryRoots();
  };

  const createBackup = async () => {
    setBackupBusy(true);
    notify(null);
    try {
      const report = await gateway.pickAndCreateBackup();
      if (report) {
        setBackupReport(report);
        notify(`Резервная копия создана: ${report.backupPath}`);
      }
    } catch (error) {
      notify(`Резервная копия не создана: ${String(error)}`);
    } finally {
      setBackupBusy(false);
    }
  };

  const previewRestore = async () => {
    setBackupBusy(true);
    setRestorePreview(null);
    notify(null);
    try {
      const preview = await gateway.pickAndPreviewRestore();
      if (preview) setRestorePreview(preview);
    } catch (error) {
      notify(`Backup не прошёл проверку: ${String(error)}`);
    } finally {
      setBackupBusy(false);
    }
  };

  const restoreAndRestart = async () => {
    if (!restorePreview) return;
    setBackupBusy(true);
    notify(null);
    try {
      await gateway.prepareRestoreAndRestart(restorePreview);
      notify("Восстановление подготовлено. SortTrack перезапускается…");
    } catch (error) {
      notify(`Восстановление остановлено: ${String(error)}`);
      setBackupBusy(false);
    }
  };

  const previewDiagnostics = async (redactPaths: boolean) => {
    setDiagnosticsBusy(true);
    setDiagnosticsReport(null);
    try {
      setDiagnosticsPreview(await gateway.previewDiagnostics(redactPaths));
    } catch (error) {
      notify(`Не удалось подготовить диагностику: ${String(error)}`);
    } finally {
      setDiagnosticsBusy(false);
    }
  };

  const exportDiagnostics = async (redactPaths: boolean) => {
    if (!diagnosticsPreview) return;
    setDiagnosticsBusy(true);
    try {
      const report = await gateway.pickAndExportDiagnostics(diagnosticsPreview, redactPaths);
      if (report) {
        setDiagnosticsReport(report);
        notify(`Диагностический пакет сохранён: ${report.path}`);
      }
    } catch (error) {
      setDiagnosticsPreview(null);
      notify(`Экспорт диагностики остановлен: ${String(error)}`);
    } finally {
      setDiagnosticsBusy(false);
    }
  };

  const openLogs = async () => {
    try {
      await gateway.openLogDirectory();
    } catch (error) {
      notify(`Не удалось открыть папку логов: ${String(error)}`);
    }
  };

  const clearDiagnostics = async () => {
    setDiagnosticsBusy(true);
    try {
      const removed = await gateway.clearDiagnostics();
      setDiagnosticsPreview(null);
      setDiagnosticsReport(null);
      notify(`Удалено локальных диагностических файлов: ${removed}.`);
    } catch (error) {
      notify(`Логи не удалены: ${String(error)}`);
    } finally {
      setDiagnosticsBusy(false);
    }
  };

  return {
    open,
    backupBusy,
    backupReport,
    restorePreview,
    runtimeProfile,
    diagnosticsPreview,
    diagnosticsReport,
    diagnosticsBusy,
    show,
    close: (externallyBusy = false) => { if (!backupBusy && !diagnosticsBusy && !externallyBusy) setOpen(false); },
    forceClose: () => setOpen(false),
    createBackup,
    previewRestore,
    restoreAndRestart,
    previewDiagnostics,
    exportDiagnostics,
    openLogs,
    clearDiagnostics,
  };
}

export type SettingsManager = ReturnType<typeof useSettingsManager>;
