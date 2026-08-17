import { useEffect, useMemo, useRef, useState } from "react";
import type { AnalysisJob, GenreDatasetAudit, GenreDatasetExportReport, GenreModelEvaluation, GenreSuggestion, Track } from "../../../domain/models";
import type { AnalysisPort } from "../../../shared/api/ports";
import { analysisTrackIds, genreTrackIds } from "./analysisSelection";

type SelectionContext = { tracks: Track[]; checkedIds: ReadonlySet<string>; selected: Track | null };
type AnalysisCallbacks = {
  reportNotice: (message: string | null) => void;
  reloadLibrary: () => Promise<unknown>;
  uncheckTrack: (trackId: string) => void;
};

export function useAnalysisManager(gateway: AnalysisPort, selection: SelectionContext, callbacks: AnalysisCallbacks) {
  const callbacksRef = useRef(callbacks);
  const [queueOpen, setQueueOpen] = useState(false);
  const [jobs, setJobs] = useState<AnalysisJob[]>([]);
  const [analysisBusy, setAnalysisBusy] = useState(false);
  const [genreSuggestions, setGenreSuggestions] = useState<GenreSuggestion[] | null>(null);
  const [genreBusy, setGenreBusy] = useState(false);
  const [datasetAudit, setDatasetAudit] = useState<GenreDatasetAudit | null>(null);
  const [datasetReport, setDatasetReport] = useState<GenreDatasetExportReport | null>(null);
  const [datasetBusy, setDatasetBusy] = useState(false);
  const [modelEvaluation, setModelEvaluation] = useState<GenreModelEvaluation | null>(null);

  useEffect(() => { callbacksRef.current = callbacks; }, [callbacks]);
  const selectedForAnalysis = useMemo(() => analysisTrackIds(selection.tracks, selection.checkedIds, selection.selected), [selection.checkedIds, selection.selected, selection.tracks]);
  const selectedForGenres = useMemo(() => genreTrackIds(selection.tracks, selection.checkedIds, selection.selected), [selection.checkedIds, selection.selected, selection.tracks]);
  const notify = (message: string | null) => callbacksRef.current.reportNotice(message);

  useEffect(() => {
    if (!queueOpen) return;
    let current = true;
    const refresh = () => void gateway.listAnalysisJobs().then((nextJobs) => { if (current) setJobs(nextJobs); });
    refresh();
    const timer = window.setInterval(refresh, 700);
    return () => { current = false; window.clearInterval(timer); };
  }, [gateway, queueOpen]);

  const executeJob = async (jobId: string) => {
    setJobs((current) => current.map((job) => job.id === jobId ? { ...job, state: "running" } : job));
    try {
      await gateway.runAnalysisJob(jobId);
      setJobs(await gateway.listAnalysisJobs());
      await callbacksRef.current.reloadLibrary();
    } catch (error) {
      notify(`Фоновый анализ остановлен: ${String(error)}`);
    }
  };

  const startAnalysis = async () => {
    setAnalysisBusy(true);
    notify(null);
    try {
      const job = await gateway.enqueueAudioAnalysis(selectedForAnalysis);
      setJobs((current) => [job, ...current]);
      void executeJob(job.id);
    } catch (error) {
      notify(`Не удалось создать задание анализа: ${String(error)}`);
    } finally {
      setAnalysisBusy(false);
    }
  };

  const changeJob = async (jobId: string, action: "pause" | "cancel") => {
    try {
      if (action === "pause") await gateway.pauseAnalysisJob(jobId);
      else await gateway.cancelAnalysisJob(jobId);
      setJobs(await gateway.listAnalysisJobs());
    } catch (error) {
      notify(`Не удалось изменить состояние анализа: ${String(error)}`);
    }
  };

  const openGenreAssistant = async () => {
    if (!selectedForGenres.length) {
      notify("Выберите хотя бы один трек для проверки жанра.");
      return;
    }
    setGenreBusy(true);
    notify(null);
    try {
      setGenreSuggestions(await gateway.suggestTrackGenres(selectedForGenres));
    } catch (error) {
      notify(`Жанровый помощник не запущен: ${String(error)}`);
    } finally {
      setGenreBusy(false);
    }
  };

  const confirmGenre = async (trackId: string, nextGenre: string, subgenre: string | null, locked: boolean) => {
    setGenreBusy(true);
    notify(null);
    try {
      await gateway.confirmTrackGenre(trackId, nextGenre, subgenre, locked);
      setGenreSuggestions((current) => {
        const remaining = current?.filter((item) => item.trackId !== trackId) ?? [];
        return remaining.length ? remaining : null;
      });
      callbacksRef.current.uncheckTrack(trackId);
      await callbacksRef.current.reloadLibrary();
      notify(`Жанр подтверждён${locked ? " и зафиксирован" : ""}. Тег аудиофайла не изменён.`);
    } catch (error) {
      notify(`Жанр не сохранён: ${String(error)}`);
    } finally {
      setGenreBusy(false);
    }
  };

  const openDatasetAudit = async () => {
    setDatasetBusy(true);
    setDatasetReport(null);
    setModelEvaluation(null);
    notify(null);
    try {
      const [audit, latestEvaluation] = await Promise.all([gateway.auditGenreDataset(), gateway.getLatestGenreModelEvaluation()]);
      setDatasetAudit(audit);
      setModelEvaluation(latestEvaluation);
    } catch (error) {
      notify(`Аудит разметки не выполнен: ${String(error)}`);
    } finally {
      setDatasetBusy(false);
    }
  };

  const exportDataset = async () => {
    setDatasetBusy(true);
    notify(null);
    try {
      const report = await gateway.pickAndExportGenreDataset();
      if (report) {
        setDatasetReport(report);
        setDatasetAudit(await gateway.auditGenreDataset());
      }
    } catch (error) {
      notify(`Manifest не создан: ${String(error)}`);
    } finally {
      setDatasetBusy(false);
    }
  };

  const evaluateGenreModel = async () => {
    setDatasetBusy(true);
    notify(null);
    try {
      setModelEvaluation(await gateway.evaluateLatestGenreModel());
    } catch (error) {
      notify(`Baseline не оценён: ${String(error)}`);
    } finally {
      setDatasetBusy(false);
    }
  };

  const exportModelReport = async () => {
    setDatasetBusy(true);
    notify(null);
    try {
      const outputPath = await gateway.pickAndExportLatestModelReport();
      if (outputPath) notify(`Отчёт проверки модели сохранён: ${outputPath}`);
    } catch (error) {
      notify(`Отчёт модели не сохранён: ${String(error)}`);
    } finally {
      setDatasetBusy(false);
    }
  };

  const closeDataset = () => {
    if (datasetBusy) return;
    setDatasetAudit(null);
    setDatasetReport(null);
    setModelEvaluation(null);
  };

  return {
    queueOpen,
    jobs,
    analysisBusy,
    analysisSelectionCount: selectedForAnalysis.length,
    genreSuggestions,
    genreBusy,
    datasetAudit,
    datasetReport,
    datasetBusy,
    modelEvaluation,
    openQueue: () => setQueueOpen(true),
    closeQueue: () => setQueueOpen(false),
    startAnalysis,
    executeJob,
    pauseJob: (jobId: string) => changeJob(jobId, "pause"),
    cancelJob: (jobId: string) => changeJob(jobId, "cancel"),
    openGenreAssistant,
    confirmGenre,
    closeGenreAssistant: () => { if (!genreBusy) setGenreSuggestions(null); },
    openDatasetAudit,
    exportDataset,
    evaluateGenreModel,
    exportModelReport,
    closeDataset,
  };
}

export type AnalysisManager = ReturnType<typeof useAnalysisManager>;
