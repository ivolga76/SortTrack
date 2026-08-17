import type { LibraryScanController } from "../../library-scan/model/useLibraryScan";
import type { SettingsManager } from "../model/useSettingsManager";
import { SettingsDialog } from "./SettingsDialog";

export function SettingsFeature({ manager, libraryScan, desktop }: {
  manager: SettingsManager;
  libraryScan: LibraryScanController;
  desktop: boolean;
}) {
  if (!manager.open) return null;
  return <SettingsDialog
    desktop={desktop}
    busy={manager.backupBusy || libraryScan.scanning || libraryScan.rootBusy}
    report={manager.backupReport}
    restorePreview={manager.restorePreview}
    runtimeProfile={manager.runtimeProfile}
    libraryRoots={libraryScan.roots}
    libraryRootBusy={libraryScan.rootBusy}
    libraryRootRemoval={libraryScan.rootRemoval}
    diagnosticsPreview={manager.diagnosticsPreview}
    diagnosticsReport={manager.diagnosticsReport}
    diagnosticsBusy={manager.diagnosticsBusy}
    onBackup={() => void manager.createBackup()}
    onPreviewRestore={() => void manager.previewRestore()}
    onRestore={() => void manager.restoreAndRestart()}
    onPreviewLibraryRoot={(root) => void libraryScan.previewSavedRoot(root, manager.forceClose)}
    onSetLibraryRootEnabled={(root, enabled) => void libraryScan.setRootEnabled(root, enabled)}
    onPreviewLibraryRootRemoval={(root) => void libraryScan.previewRootRemoval(root)}
    onCancelLibraryRootRemoval={libraryScan.cancelRootRemoval}
    onConfirmLibraryRootRemoval={() => void libraryScan.confirmRootRemoval()}
    onPreviewDiagnostics={(redactPaths) => void manager.previewDiagnostics(redactPaths)}
    onExportDiagnostics={(redactPaths) => void manager.exportDiagnostics(redactPaths)}
    onOpenLogs={() => void manager.openLogs()}
    onClearDiagnostics={() => void manager.clearDiagnostics()}
    onClose={() => manager.close(libraryScan.scanning || libraryScan.rootBusy)}
  />;
}
