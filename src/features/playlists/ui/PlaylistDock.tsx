import type { Track } from "../../../domain/models";
import { PlaylistWorkspace } from "./PlaylistWorkspace";
import type { PlaylistManager } from "../model/usePlaylistManager";

export function PlaylistDock({ manager, selectedTrack, onPlay }: {
  manager: PlaylistManager;
  selectedTrack: Track | null;
  onPlay: (track: Track) => void;
}) {
  return <PlaylistWorkspace
    playlists={manager.playlists}
    selectedTrack={selectedTrack}
    busy={manager.busy}
    removalPreview={manager.removalPreview}
    onClose={manager.closeWorkspace}
    onCreate={manager.createPlaylist}
    onCreateSmart={manager.openSmartCreator}
    onEditSmart={manager.openSmartPlaylistEditor}
    onSnapshotSmart={(playlistId) => void manager.snapshotSmartPlaylist(playlistId)}
    onRename={manager.renamePlaylist}
    onPreviewDelete={(playlistId) => void manager.previewRemoval(playlistId)}
    onCancelDelete={manager.cancelRemoval}
    onConfirmDelete={() => void manager.confirmRemoval()}
    onPlace={manager.placeTrack}
    onRemove={manager.removePlaylistItem}
    onPlay={onPlay}
    onCopy={(playlistId) => void manager.startCopy(playlistId)}
    onExport={manager.openExport}
  />;
}
