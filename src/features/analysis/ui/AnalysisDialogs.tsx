import type { AnalysisManager } from "../model/useAnalysisManager";
import { AnalysisQueueDialog } from "./AnalysisQueueDialog";
import { DatasetAuditDialog } from "./DatasetAuditDialog";
import { GenreAssistantDialog } from "./GenreAssistantDialog";

export function AnalysisDialogs({ manager, desktop }: { manager: AnalysisManager; desktop: boolean }) {
  return <>
    {manager.queueOpen && <AnalysisQueueDialog
      jobs={manager.jobs}
      selectionCount={manager.analysisSelectionCount}
      busy={manager.analysisBusy}
      onStart={() => void manager.startAnalysis()}
      onResume={(jobId) => void manager.executeJob(jobId)}
      onPause={(jobId) => void manager.pauseJob(jobId)}
      onCancel={(jobId) => void manager.cancelJob(jobId)}
      onClose={manager.closeQueue}
    />}
    {manager.genreSuggestions && <GenreAssistantDialog
      suggestions={manager.genreSuggestions}
      busy={manager.genreBusy}
      onConfirm={(trackId, nextGenre, subgenre, locked) => void manager.confirmGenre(trackId, nextGenre, subgenre, locked)}
      onClose={manager.closeGenreAssistant}
    />}
    {manager.datasetAudit && <DatasetAuditDialog
      audit={manager.datasetAudit}
      report={manager.datasetReport}
      evaluation={manager.modelEvaluation}
      busy={manager.datasetBusy}
      desktop={desktop}
      onExport={() => void manager.exportDataset()}
      onEvaluate={() => void manager.evaluateGenreModel()}
      onExportModelReport={() => void manager.exportModelReport()}
      onClose={manager.closeDataset}
    />}
  </>;
}
