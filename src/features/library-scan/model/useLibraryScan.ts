import { useCallback, useEffect, useRef, useState } from "react";
import type { LibraryRootRemovalPreview, LibraryRootSettings, ScanOptions, ScanPreview, ScanProgress, ScanResult } from "../../../domain/models";
import type { LibraryScanPort } from "../../../shared/api/ports";
import { rootEnabledMessage, scanCompletedMessage } from "./scanMessages";

type NoticeReporter = (message: string | null) => void;

export function useLibraryScan(gateway: LibraryScanPort, reportNotice: NoticeReporter, reloadLibrary: () => Promise<unknown>) {
  const noticeRef = useRef(reportNotice);
  const reloadLibraryRef = useRef(reloadLibrary);
  const [roots, setRoots] = useState<LibraryRootSettings[]>([]);
  const [rootBusy, setRootBusy] = useState(false);
  const [rootRemoval, setRootRemoval] = useState<LibraryRootRemovalPreview | null>(null);
  const [scanning, setScanning] = useState(false);
  const [preview, setPreview] = useState<ScanPreview | null>(null);
  const [progress, setProgress] = useState<ScanProgress | null>(null);
  const [report, setReport] = useState<ScanResult | null>(null);

  useEffect(() => { noticeRef.current = reportNotice; }, [reportNotice]);
  useEffect(() => { reloadLibraryRef.current = reloadLibrary; }, [reloadLibrary]);
  const notify = useCallback((message: string | null) => noticeRef.current(message), []);
  const reloadRoots = useCallback(async () => setRoots(await gateway.listLibraryRoots()), [gateway]);

  useEffect(() => {
    void reloadRoots().catch((error) => notify(`Не удалось загрузить папки библиотеки: ${String(error)}`));
  }, [notify, reloadRoots]);

  useEffect(() => {
    if (!scanning || !preview) return;
    let active = true;
    const refresh = () => void gateway.getScanProgress()
      .then((nextProgress) => { if (active && nextProgress) setProgress(nextProgress); })
      .catch(() => undefined);
    refresh();
    const timer = window.setInterval(refresh, 250);
    return () => { active = false; window.clearInterval(timer); };
  }, [gateway, preview, scanning]);

  const refreshRootAvailability = async () => {
    setRootRemoval(null);
    setRootBusy(true);
    try {
      setRoots(await gateway.refreshLibraryRootAvailability());
    } catch (error) {
      notify(`Не удалось проверить доступность папок библиотеки: ${String(error)}`);
    } finally {
      setRootBusy(false);
    }
  };

  const addFolder = async () => {
    if (!gateway.isDesktop) {
      notify("Выбор локальной папки доступен в desktop-сборке SortTrack.");
      return;
    }
    setScanning(true);
    notify(null);
    try {
      const nextPreview = await gateway.pickAndPreviewScanFolder();
      if (nextPreview) {
        setReport(null);
        setPreview(nextPreview);
      }
    } catch (error) {
      notify(`Сканирование не выполнено: ${String(error)}`);
    } finally {
      setScanning(false);
    }
  };

  const refreshPreview = async (options: ScanOptions) => {
    if (!preview) return;
    setScanning(true);
    notify(null);
    try {
      setPreview(await gateway.previewScanFolder(preview.rootPath, options));
    } catch (error) {
      notify(`Не удалось обновить оценку папки: ${String(error)}`);
    } finally {
      setScanning(false);
    }
  };

  const previewSavedRoot = async (root: LibraryRootSettings, onPrepared?: () => void) => {
    if (root.status !== "active") {
      notify("Сначала подключите папку библиотеки.");
      return;
    }
    setScanning(true);
    notify(null);
    try {
      const nextPreview = await gateway.previewScanFolder(root.path, { recursive: root.recursive, exclusions: root.exclusions });
      setReport(null);
      setPreview(nextPreview);
      onPrepared?.();
    } catch (error) {
      notify(`Не удалось проверить сохранённую папку: ${String(error)}`);
    } finally {
      setScanning(false);
    }
  };

  const setRootEnabled = async (root: LibraryRootSettings, enabled: boolean) => {
    setRootBusy(true);
    notify(null);
    try {
      await gateway.setLibraryRootEnabled(root.id, root.status, enabled);
      await reloadRoots();
      notify(rootEnabledMessage(enabled));
    } catch (error) {
      await reloadRoots().catch(() => undefined);
      notify(`Не удалось изменить состояние папки: ${String(error)}`);
    } finally {
      setRootBusy(false);
    }
  };

  const previewRootRemoval = async (root: LibraryRootSettings) => {
    setRootBusy(true);
    notify(null);
    try {
      setRootRemoval(await gateway.previewLibraryRootRemoval(root.id));
    } catch (error) {
      await reloadRoots().catch(() => undefined);
      notify(`Не удалось подготовить удаление регистрации: ${String(error)}`);
    } finally {
      setRootBusy(false);
    }
  };

  const confirmRootRemoval = async () => {
    if (!rootRemoval) return;
    setRootBusy(true);
    notify(null);
    try {
      const removalReport = await gateway.removeLibraryRootRegistration(rootRemoval);
      setRootRemoval(null);
      await reloadRoots();
      notify(`Регистрация папки удалена; ${removalReport.detachedFiles} записей треков сохранены без изменений. MP3/FLAC не затронуты.`);
    } catch (error) {
      setRootRemoval(null);
      await reloadRoots().catch(() => undefined);
      notify(`Регистрация папки не удалена: ${String(error)}`);
    } finally {
      setRootBusy(false);
    }
  };

  const confirmScan = async () => {
    if (!preview) return;
    setScanning(true);
    notify(null);
    try {
      const result = await gateway.scanFolder(preview);
      setReport(result);
      setPreview(null);
      setProgress(null);
      await Promise.all([reloadLibraryRef.current(), reloadRoots()]);
      notify(scanCompletedMessage(result));
    } catch (error) {
      notify(`Сканирование не выполнено: ${String(error)}`);
    } finally {
      setScanning(false);
    }
  };

  const controlScan = async (action: "pause" | "resume" | "cancel", jobId: string) => {
    try {
      if (action === "pause") await gateway.pauseScan(jobId);
      else if (action === "resume") await gateway.resumeScan(jobId);
      else await gateway.cancelScan(jobId);
      const nextProgress = await gateway.getScanProgress();
      if (nextProgress) setProgress(nextProgress);
    } catch (error) {
      notify(`Не удалось изменить состояние индексирования: ${String(error)}`);
    }
  };

  const closePreview = () => {
    if (scanning) return;
    setPreview(null);
    setProgress(null);
  };

  return {
    roots,
    rootBusy,
    rootRemoval,
    scanning,
    preview,
    progress,
    report,
    refreshRootAvailability,
    addFolder,
    refreshPreview,
    previewSavedRoot,
    setRootEnabled,
    previewRootRemoval,
    cancelRootRemoval: () => setRootRemoval(null),
    confirmRootRemoval,
    confirmScan,
    controlScan,
    closePreview,
    closeReport: () => setReport(null),
  };
}

export type LibraryScanController = ReturnType<typeof useLibraryScan>;
