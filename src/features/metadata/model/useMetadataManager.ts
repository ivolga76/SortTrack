import { useEffect, useRef, useState } from "react";
import type {
  BulkMetadataCommitItem,
  BulkMetadataPreview,
  BulkMetadataReport,
  EditableMetadataField,
  MetadataPreview,
  MetadataValues,
  RelinkPreview,
  Track,
  TrackMetadataDetail,
} from "../../../domain/models";
import type { MetadataPort } from "../../../shared/api/ports";
import { resolveMetadataSelection } from "./metadataSelection";

type MetadataCallbacks = {
  reportNotice: (message: string | null) => void;
  reloadLibrary: () => Promise<unknown>;
  reloadPlaylists: () => Promise<unknown>;
  selectTrack: (track: Track) => void;
  clearCheckedTracks: () => void;
  onRelinkCommitted: (trackId: string) => void;
};

export function useMetadataManager(gateway: MetadataPort, callbacks: MetadataCallbacks) {
  const callbacksRef = useRef(callbacks);
  const [detail, setDetail] = useState<TrackMetadataDetail | null>(null);
  const [loading, setLoading] = useState(false);
  const [bulkTrackIds, setBulkTrackIds] = useState<string[] | null>(null);
  const [relinkPreview, setRelinkPreview] = useState<RelinkPreview | null>(null);
  const [relinkBusy, setRelinkBusy] = useState(false);

  useEffect(() => { callbacksRef.current = callbacks; }, [callbacks]);
  const notify = (message: string | null) => callbacksRef.current.reportNotice(message);

  const openTrack = async (track: Track | null) => {
    if (!track) {
      notify("Сначала выберите один трек в таблице.");
      return;
    }
    setLoading(true);
    notify(null);
    try {
      callbacksRef.current.selectTrack(track);
      setDetail(await gateway.getTrackMetadata(track.id));
    } catch (error) {
      notify(`Не удалось открыть метаданные: ${String(error)}`);
    } finally {
      setLoading(false);
    }
  };

  const openSelection = (checkedIds: ReadonlySet<string>, tracks: Track[], selected: Track | null) => {
    const selection = resolveMetadataSelection(checkedIds, tracks, selected);
    if (selection.mode === "bulk") setBulkTrackIds(selection.trackIds);
    else if (selection.mode === "single") void openTrack(selection.track);
    else void openTrack(null);
  };

  const previewUpdate = (values: MetadataValues): Promise<MetadataPreview> => {
    if (!detail) return Promise.reject(new Error("Metadata dialog is closed"));
    return gateway.previewMetadataUpdate(detail.trackId, values);
  };

  const commitUpdate = (preview: MetadataPreview): Promise<TrackMetadataDetail> => {
    if (!detail) return Promise.reject(new Error("Metadata dialog is closed"));
    return gateway.commitMetadataUpdate(detail.trackId, preview.expectedFileState, preview.after);
  };

  const metadataSaved = (updated: TrackMetadataDetail) => {
    setDetail(updated);
    notify("Метаданные безопасно записаны; предыдущая версия добавлена в историю.");
    void callbacksRef.current.reloadLibrary();
  };

  const previewBulkUpdate = (fields: EditableMetadataField[], values: MetadataValues): Promise<BulkMetadataPreview> => {
    if (!bulkTrackIds) return Promise.reject(new Error("Bulk metadata dialog is closed"));
    return gateway.previewBulkMetadataUpdate({ trackIds: bulkTrackIds, fields, values });
  };

  const commitBulkUpdate = (items: BulkMetadataCommitItem[]): Promise<BulkMetadataReport> => gateway.commitBulkMetadataUpdate(items);

  const bulkMetadataSaved = (report: BulkMetadataReport) => {
    callbacksRef.current.clearCheckedTracks();
    notify(`Массовая запись завершена: сохранено ${report.savedFiles}, ошибок ${report.failedFiles}.`);
    void callbacksRef.current.reloadLibrary();
  };

  const openRelink = async (track: Track) => {
    setRelinkBusy(true);
    notify(null);
    try {
      const preview = await gateway.pickAndPreviewRelink(track.id);
      if (preview) setRelinkPreview(preview);
      else if (!gateway.isDesktop) notify("Поиск файла доступен в desktop-приложении SortTrack.");
    } catch (error) {
      notify(`Не удалось проверить выбранный файл: ${String(error)}`);
    } finally {
      setRelinkBusy(false);
    }
  };

  const commitRelink = async () => {
    if (!relinkPreview) return;
    setRelinkBusy(true);
    notify(null);
    try {
      const report = await gateway.commitRelink(relinkPreview);
      setRelinkPreview(null);
      callbacksRef.current.onRelinkCommitted(report.trackId);
      await Promise.all([callbacksRef.current.reloadLibrary(), callbacksRef.current.reloadPlaylists()]);
      notify(report.availabilityStatus === "available"
        ? "Файл найден и безопасно перепривязан к сохранённому треку."
        : "Файл перепривязан, но отмечен как повреждённый: аудиоданные не читаются.");
    } catch (error) {
      notify(`Перепривязка остановлена: ${String(error)}`);
    } finally {
      setRelinkBusy(false);
    }
  };

  return {
    detail,
    loading,
    bulkTrackIds,
    relinkPreview,
    relinkBusy,
    openTrack,
    openSelection,
    closeMetadata: () => setDetail(null),
    previewUpdate,
    commitUpdate,
    metadataSaved,
    closeBulkMetadata: () => setBulkTrackIds(null),
    previewBulkUpdate,
    commitBulkUpdate,
    bulkMetadataSaved,
    openRelink,
    commitRelink,
    closeRelink: () => { if (!relinkBusy) setRelinkPreview(null); },
  };
}

export type MetadataManager = ReturnType<typeof useMetadataManager>;
