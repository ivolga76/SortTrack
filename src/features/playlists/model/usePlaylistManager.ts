import { useCallback, useEffect, useRef, useState } from "react";
import type {
  CopyPlan,
  CopyRecoveryOperation,
  CopyRecoveryReport,
  CopyReport,
  CopyResolution,
  Playlist,
  PlaylistExportFormat,
  PlaylistExportPathMode,
  PlaylistExportPreview,
  PlaylistExportReport,
  PlaylistPlacement,
  PlaylistRemovalPreview,
  SmartPlaylistRule,
  TraktorTargetProfile,
} from "../../../domain/models";
import type { FileCopyPort, PlaylistExportPort, PlaylistPort } from "../../../shared/api/ports";
import { nextManualPlaylistName, smartPlaylistSnapshotName } from "./playlistNames";

type NoticeReporter = (message: string | null) => void;

export function usePlaylistManager(gateway: PlaylistPort & FileCopyPort & PlaylistExportPort, reportNotice: NoticeReporter) {
  const noticeRef = useRef(reportNotice);
  const [playlists, setPlaylists] = useState<Playlist[]>([]);
  const [workspaceOpen, setWorkspaceOpen] = useState(true);
  const [busy, setBusy] = useState(false);
  const [removalPreview, setRemovalPreview] = useState<PlaylistRemovalPreview | null>(null);
  const [smartCreatorOpen, setSmartCreatorOpen] = useState(false);
  const [smartPlaylistEdit, setSmartPlaylistEdit] = useState<Playlist | null>(null);
  const [copyPlan, setCopyPlan] = useState<CopyPlan | null>(null);
  const [copyReport, setCopyReport] = useState<CopyReport | null>(null);
  const [copyBusy, setCopyBusy] = useState(false);
  const [copyRecoveries, setCopyRecoveries] = useState<CopyRecoveryOperation[]>([]);
  const [recoveryReport, setRecoveryReport] = useState<CopyRecoveryReport | null>(null);
  const [recoveryBusy, setRecoveryBusy] = useState(false);
  const [exportPlaylistId, setExportPlaylistId] = useState<string | null>(null);
  const [exportPreview, setExportPreview] = useState<PlaylistExportPreview | null>(null);
  const [exportReport, setExportReport] = useState<PlaylistExportReport | null>(null);
  const [exportBusy, setExportBusy] = useState(false);

  useEffect(() => { noticeRef.current = reportNotice; }, [reportNotice]);
  const notify = useCallback((message: string | null) => noticeRef.current(message), []);
  const reloadPlaylists = useCallback(async () => setPlaylists(await gateway.listPlaylists()), [gateway]);

  useEffect(() => {
    void reloadPlaylists().catch((error) => notify(`Не удалось открыть плейлисты: ${String(error)}`));
    void gateway.listCopyRecoveries().then(setCopyRecoveries).catch((error) => notify(`Не удалось проверить незавершённые операции: ${String(error)}`));
  }, [gateway, notify, reloadPlaylists]);

  const runOperation = async (operation: () => Promise<void>) => {
    setBusy(true);
    try {
      await operation();
      await reloadPlaylists();
    } catch (error) {
      notify(`Операция с плейлистом не выполнена: ${String(error)}`);
    } finally {
      setBusy(false);
    }
  };

  const createPlaylist = () => {
    const name = nextManualPlaylistName(playlists);
    void runOperation(async () => { await gateway.createPlaylist(name); });
  };

  const createSmartPlaylist = async (name: string, rule: SmartPlaylistRule) => {
    setBusy(true);
    notify(null);
    try {
      await gateway.createSmartPlaylist(name, rule);
      await reloadPlaylists();
      setSmartCreatorOpen(false);
      notify(`Автоплейлист «${name}» создан и рассчитан по текущей библиотеке.`);
    } catch (error) {
      notify(`Автоплейлист не создан: ${String(error)}`);
    } finally {
      setBusy(false);
    }
  };

  const snapshotSmartPlaylist = async (playlistId: string) => {
    const source = playlists.find((playlist) => playlist.id === playlistId && playlist.kind === "smart");
    if (!source) return;
    const name = smartPlaylistSnapshotName(source.name);
    setBusy(true);
    notify(null);
    try {
      const report = await gateway.snapshotSmartPlaylist(playlistId, name);
      await reloadPlaylists();
      notify(`Создан обычный плейлист «${report.name}»: сохранено ${report.itemCount} треков. Автоплейлист не изменён.`);
    } catch (error) {
      notify(`Не удалось сохранить снимок автоплейлиста: ${String(error)}`);
    } finally {
      setBusy(false);
    }
  };

  const openSmartPlaylistEditor = (playlistId: string) => {
    const playlist = playlists.find((item) => item.id === playlistId && item.kind === "smart" && item.smartRule);
    if (playlist) setSmartPlaylistEdit(structuredClone(playlist));
  };

  const updateSmartPlaylistRule = async (nextRule: SmartPlaylistRule) => {
    if (!smartPlaylistEdit?.smartRule) return;
    setBusy(true);
    notify(null);
    try {
      await gateway.updateSmartPlaylistRule(smartPlaylistEdit.id, smartPlaylistEdit.smartRule, nextRule);
      await reloadPlaylists();
      setSmartPlaylistEdit(null);
      notify(`Правило автоплейлиста «${smartPlaylistEdit.name}» обновлено; состав пересчитан.`);
    } catch (error) {
      setSmartPlaylistEdit(null);
      await reloadPlaylists().catch(() => undefined);
      notify(`Правило не сохранено: ${String(error)}`);
    } finally {
      setBusy(false);
    }
  };

  const previewRemoval = async (playlistId: string) => {
    setBusy(true);
    notify(null);
    try {
      setRemovalPreview(await gateway.previewPlaylistRemoval(playlistId));
    } catch (error) {
      notify(`Не удалось подготовить удаление плейлиста: ${String(error)}`);
    } finally {
      setBusy(false);
    }
  };

  const confirmRemoval = async () => {
    if (!removalPreview) return;
    setBusy(true);
    notify(null);
    try {
      const report = await gateway.removePlaylist(removalPreview);
      if (exportPlaylistId === report.playlistId) setExportPlaylistId(null);
      setRemovalPreview(null);
      await reloadPlaylists();
      notify(`Плейлист «${report.name}» удалён; ${report.removedItems} связей убрано. Музыкальные файлы не изменены.`);
    } catch (error) {
      setRemovalPreview(null);
      await reloadPlaylists().catch(() => undefined);
      notify(`Плейлист не удалён: ${String(error)}`);
    } finally {
      setBusy(false);
    }
  };

  const placeTrack = (placement: PlaylistPlacement) => void runOperation(() => gateway.placeTrack(placement));
  const renamePlaylist = (playlistId: string, name: string) => void runOperation(() => gateway.renamePlaylist(playlistId, name));
  const removePlaylistItem = (playlistId: string, itemId: string) => void runOperation(() => gateway.removePlaylistItem(playlistId, itemId));

  const startCopy = async (playlistId: string) => {
    setCopyBusy(true);
    notify(null);
    try {
      const plan = await gateway.pickAndPlanPlaylistCopy(playlistId);
      if (plan) {
        setCopyReport(null);
        setCopyPlan(plan);
      } else if (!gateway.isDesktop) {
        notify("Безопасное копирование доступно в desktop-приложении SortTrack.");
      }
    } catch (error) {
      notify(`Не удалось подготовить копирование: ${String(error)}`);
    } finally {
      setCopyBusy(false);
    }
  };

  const confirmCopy = async (resolutions: CopyResolution[]) => {
    if (!copyPlan) return;
    setCopyBusy(true);
    try {
      setCopyReport(await gateway.executePlaylistCopy(copyPlan.operationId, resolutions));
    } catch (error) {
      notify(`Копирование не выполнено: ${String(error)}`);
    } finally {
      setCopyBusy(false);
    }
  };

  const closeCopy = () => {
    if (copyBusy) return;
    setCopyPlan(null);
    setCopyReport(null);
  };

  const recoverCopy = async (action: "resume" | "cancel_safe") => {
    const operation = copyRecoveries[0];
    if (!operation) return;
    setRecoveryBusy(true);
    notify(null);
    try {
      setRecoveryReport(await gateway.recoverCopyOperation(operation.operationId, action));
    } catch (error) {
      notify(`Восстановление не выполнено: ${String(error)}`);
    } finally {
      setRecoveryBusy(false);
    }
  };

  const closeRecovery = () => {
    if (recoveryBusy) return;
    setCopyRecoveries((current) => current.slice(1));
    setRecoveryReport(null);
  };

  const openExport = (playlistId: string) => {
    setExportPlaylistId(playlistId);
    setExportPreview(null);
    setExportReport(null);
  };

  const previewExport = async (format: PlaylistExportFormat, pathMode: PlaylistExportPathMode, targetProfile: TraktorTargetProfile | null) => {
    if (!exportPlaylistId) return;
    setExportBusy(true);
    notify(null);
    try {
      const preview = await gateway.pickAndPreviewPlaylistExport(exportPlaylistId, format, pathMode, targetProfile);
      if (preview) setExportPreview(preview);
    } catch (error) {
      notify(`Не удалось подготовить экспорт: ${String(error)}`);
    } finally {
      setExportBusy(false);
    }
  };

  const commitExport = async () => {
    if (!exportPreview) return;
    setExportBusy(true);
    notify(null);
    try {
      setExportReport(await gateway.commitPlaylistExport(exportPreview, exportPreview.expectedState));
    } catch (error) {
      notify(`Экспорт не выполнен: ${String(error)}`);
    } finally {
      setExportBusy(false);
    }
  };

  const closeExport = () => {
    if (exportBusy) return;
    setExportPlaylistId(null);
    setExportPreview(null);
    setExportReport(null);
  };

  return {
    playlists,
    workspaceOpen,
    busy,
    removalPreview,
    smartCreatorOpen,
    smartPlaylistEdit,
    copyPlan,
    copyReport,
    copyBusy,
    recovery: copyRecoveries[0] ?? null,
    recoveryReport,
    recoveryBusy,
    exportPlaylist: playlists.find((playlist) => playlist.id === exportPlaylistId) ?? null,
    exportPreview,
    exportReport,
    exportBusy,
    reloadPlaylists,
    openWorkspace: () => setWorkspaceOpen(true),
    closeWorkspace: () => setWorkspaceOpen(false),
    createPlaylist,
    openSmartCreator: () => setSmartCreatorOpen(true),
    closeSmartCreator: () => { if (!busy) setSmartCreatorOpen(false); },
    createSmartPlaylist,
    openSmartPlaylistEditor,
    closeSmartPlaylistEditor: () => { if (!busy) setSmartPlaylistEdit(null); },
    updateSmartPlaylistRule,
    snapshotSmartPlaylist,
    renamePlaylist,
    previewRemoval,
    cancelRemoval: () => setRemovalPreview(null),
    confirmRemoval,
    placeTrack,
    removePlaylistItem,
    startCopy,
    confirmCopy,
    closeCopy,
    recoverCopy,
    closeRecovery,
    openExport,
    previewExport,
    commitExport,
    resetExport: () => { setExportPreview(null); setExportReport(null); },
    closeExport,
  };
}

export type PlaylistManager = ReturnType<typeof usePlaylistManager>;
