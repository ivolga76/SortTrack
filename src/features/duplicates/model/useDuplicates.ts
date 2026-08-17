import { useCallback, useEffect, useRef, useState } from "react";
import type { DuplicateGroup, DuplicateScanResult } from "../../../domain/models";
import type { DuplicatesPort } from "../../../shared/api/ports";

type NoticeReporter = (message: string | null) => void;

export function useDuplicates(gateway: DuplicatesPort, reportNotice: NoticeReporter, refreshSummary: () => Promise<void>) {
  const noticeRef = useRef(reportNotice);
  const refreshSummaryRef = useRef(refreshSummary);
  const [groups, setGroups] = useState<DuplicateGroup[]>([]);
  const [lastScan, setLastScan] = useState<DuplicateScanResult | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => { noticeRef.current = reportNotice; }, [reportNotice]);
  useEffect(() => { refreshSummaryRef.current = refreshSummary; }, [refreshSummary]);

  const load = useCallback(async () => {
    try {
      setGroups(await gateway.listDuplicateGroups());
    } catch (error) {
      noticeRef.current(`Не удалось открыть список дубликатов: ${String(error)}`);
    }
  }, [gateway]);

  const scan = useCallback(async () => {
    setBusy(true);
    noticeRef.current(null);
    try {
      const result = await gateway.scanExactDuplicates();
      setLastScan(result);
      setGroups(result.groups);
      await refreshSummaryRef.current();
    } catch (error) {
      noticeRef.current(`Поиск дубликатов не выполнен: ${String(error)}`);
    } finally {
      setBusy(false);
    }
  }, [gateway]);

  return { groups, lastScan, busy, load, scan };
}
