import type { MetadataManager } from "../model/useMetadataManager";
import { BulkMetadataDialog } from "./BulkMetadataDialog";
import { MetadataDialog } from "./MetadataDialog";
import { RelinkDialog } from "./RelinkDialog";

export function MetadataDialogs({ manager }: { manager: MetadataManager }) {
  return <>
    {manager.detail && <MetadataDialog
      detail={manager.detail}
      onClose={manager.closeMetadata}
      onPreview={manager.previewUpdate}
      onCommit={manager.commitUpdate}
      onSaved={manager.metadataSaved}
    />}
    {manager.bulkTrackIds && <BulkMetadataDialog
      trackIds={manager.bulkTrackIds}
      onClose={manager.closeBulkMetadata}
      onPreview={manager.previewBulkUpdate}
      onCommit={manager.commitBulkUpdate}
      onSaved={manager.bulkMetadataSaved}
    />}
    {manager.relinkPreview && <RelinkDialog
      preview={manager.relinkPreview}
      busy={manager.relinkBusy}
      onCommit={() => void manager.commitRelink()}
      onClose={manager.closeRelink}
    />}
  </>;
}
