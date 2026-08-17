import { CopyDialog } from "./CopyDialog";
import { CopyRecoveryDialog } from "./CopyRecoveryDialog";
import { PlaylistExportDialog } from "./PlaylistExportDialog";
import { SmartPlaylistDialog } from "./SmartPlaylistDialog";
import { GENRE_TAXONOMY } from "../../../domain/genre";
import type { PlaylistManager } from "../model/usePlaylistManager";

export function PlaylistDialogs({ manager }: { manager: PlaylistManager }) {
  return <>
    {manager.copyPlan && <CopyDialog
      plan={manager.copyPlan}
      report={manager.copyReport}
      busy={manager.copyBusy}
      onConfirm={(resolutions) => void manager.confirmCopy(resolutions)}
      onClose={manager.closeCopy}
    />}
    {manager.recovery && <CopyRecoveryDialog
      operation={manager.recovery}
      report={manager.recoveryReport}
      busy={manager.recoveryBusy}
      onAction={(action) => void manager.recoverCopy(action)}
      onClose={manager.closeRecovery}
    />}
    {manager.exportPlaylist && <PlaylistExportDialog
      playlist={manager.exportPlaylist}
      preview={manager.exportPreview}
      report={manager.exportReport}
      busy={manager.exportBusy}
      onPreview={(format, pathMode, targetProfile) => void manager.previewExport(format, pathMode, targetProfile)}
      onCommit={() => void manager.commitExport()}
      onReset={manager.resetExport}
      onClose={manager.closeExport}
    />}
    {manager.smartCreatorOpen && <SmartPlaylistDialog
      genres={Object.keys(GENRE_TAXONOMY)}
      busy={manager.busy}
      onCreate={(name, rule) => void manager.createSmartPlaylist(name, rule)}
      onClose={manager.closeSmartCreator}
    />}
    {manager.smartPlaylistEdit?.smartRule && <SmartPlaylistDialog
      genres={Object.keys(GENRE_TAXONOMY)}
      busy={manager.busy}
      initialName={manager.smartPlaylistEdit.name}
      initialRule={manager.smartPlaylistEdit.smartRule}
      onUpdate={(rule) => void manager.updateSmartPlaylistRule(rule)}
      onClose={manager.closeSmartPlaylistEditor}
    />}
  </>;
}
