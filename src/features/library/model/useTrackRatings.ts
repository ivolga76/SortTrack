import { useEffect, useRef, useState } from "react";
import type { Track } from "../../../domain/models";
import type { TrackRatingPort } from "../../../shared/api/ports";

type RatingCallbacks = {
  reportNotice: (message: string | null) => void;
  updateVisibleTrack: (trackId: string, rating: number) => void;
  reloadLibrary: () => Promise<unknown>;
  reloadPlaylists: () => Promise<unknown>;
};

export function useTrackRatings(gateway: TrackRatingPort, callbacks: RatingCallbacks) {
  const callbacksRef = useRef(callbacks);
  const [busyTrackId, setBusyTrackId] = useState<string | null>(null);
  const [bulkTarget, setBulkTarget] = useState("-1");
  const [bulkBusy, setBulkBusy] = useState(false);

  useEffect(() => { callbacksRef.current = callbacks; }, [callbacks]);
  const notify = (message: string | null) => callbacksRef.current.reportNotice(message);
  const reloadAffectedViews = () => Promise.all([callbacksRef.current.reloadLibrary(), callbacksRef.current.reloadPlaylists()]);

  const setRating = async (track: Track, nextRating: number) => {
    setBusyTrackId(track.id);
    notify(null);
    try {
      await gateway.setTrackRating(track.id, track.rating, nextRating);
      callbacksRef.current.updateVisibleTrack(track.id, nextRating);
      await reloadAffectedViews();
      notify(nextRating === 0 ? `Рейтинг трека «${track.title}» сброшен.` : `Треку «${track.title}» присвоен рейтинг ${nextRating} из 5.`);
    } catch (error) {
      await Promise.all([callbacksRef.current.reloadLibrary().catch(() => undefined), callbacksRef.current.reloadPlaylists().catch(() => undefined)]);
      notify(`Рейтинг не изменён: ${String(error)}`);
    } finally {
      setBusyTrackId(null);
    }
  };

  const applyBulkRating = async (trackIds: ReadonlySet<string>) => {
    const targetRating = Number(bulkTarget);
    if (trackIds.size === 0 || !Number.isInteger(targetRating) || targetRating < 0 || targetRating > 5) return;
    setBulkBusy(true);
    notify(null);
    try {
      const report = await gateway.bulkSetTrackRating([...trackIds], targetRating);
      trackIds.forEach((trackId) => callbacksRef.current.updateVisibleTrack(trackId, targetRating));
      await reloadAffectedViews();
      notify(`Массовый рейтинг: сохранено ${report.savedTracks}, без изменений ${report.unchangedTracks}, ошибок ${report.failedTracks}.`);
    } catch (error) {
      await Promise.all([callbacksRef.current.reloadLibrary().catch(() => undefined), callbacksRef.current.reloadPlaylists().catch(() => undefined)]);
      notify(`Массовый рейтинг не применён: ${String(error)}`);
    } finally {
      setBulkBusy(false);
    }
  };

  return { busyTrackId, bulkTarget, setBulkTarget, bulkBusy, setRating, applyBulkRating };
}
