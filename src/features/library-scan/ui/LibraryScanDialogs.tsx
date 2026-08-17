import type { LibraryScanController } from "../model/useLibraryScan";
import { ScanDialog } from "./ScanDialog";
import { ScanReportDialog } from "./ScanReportDialog";

export function LibraryScanDialogs({ controller }: { controller: LibraryScanController }) {
  return <>
    {controller.preview && <ScanDialog
      preview={controller.preview}
      progress={controller.progress}
      busy={controller.scanning}
      onRefresh={(options) => void controller.refreshPreview(options)}
      onConfirm={() => void controller.confirmScan()}
      onPause={(jobId) => void controller.controlScan("pause", jobId)}
      onResume={(jobId) => void controller.controlScan("resume", jobId)}
      onCancel={(jobId) => void controller.controlScan("cancel", jobId)}
      onClose={controller.closePreview}
    />}
    {controller.report && <ScanReportDialog report={controller.report} onClose={controller.closeReport}/>} 
  </>;
}
