import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ScanPreview, TrackPage, TrackQuery } from "../domain/models";
import { GatewayError } from "../shared/api/errors";
import { DemoLibraryGateway } from "./demo";
import { TauriLibraryGateway } from "./tauri";

const tauriMocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  convertFileSrc: vi.fn((path: string) => `asset://${path}`),
  open: vi.fn(),
  save: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: tauriMocks.invoke,
  convertFileSrc: tauriMocks.convertFileSrc,
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: tauriMocks.open,
  save: tauriMocks.save,
}));

const baseQuery: TrackQuery = {
  text: "",
  genre: null,
  onlyIncoming: false,
  minBpm: null,
  maxBpm: null,
  minRating: null,
  camelot: null,
  format: null,
  fileStatus: "all",
  limit: 100,
  offset: 0,
  sortBy: "added",
  descending: true,
};

describe("demo gateway characterization", () => {
  it("keeps filtering, sorting and pagination deterministic", async () => {
    const gateway = new DemoLibraryGateway();
    const page = await gateway.searchTracks({
      ...baseQuery,
      minBpm: 140,
      maxBpm: 180,
      minRating: 3,
      fileStatus: "all",
      sortBy: "bpm",
      descending: false,
    });

    expect(page).toMatchObject({ total: 2, limit: 100, offset: 0 });
    expect(page.items.map((track) => [track.id, track.bpm, track.format])).toEqual([
      ["trk-001", 142, "FLAC"],
      ["trk-002", 154, "MP3"],
    ]);
  });

  it("moves one logical playlist item without touching its track", async () => {
    const gateway = new DemoLibraryGateway();
    const before = await gateway.listPlaylists();
    const source = before[0];
    const target = before[1];
    const item = source.items[0];

    await gateway.placeTrack({
      targetPlaylistId: target.id,
      targetPosition: 0,
      trackId: item.track.id,
      itemId: item.id,
      sourcePlaylistId: source.id,
    });

    const after = await gateway.listPlaylists();
    expect(after[0].items.map((entry) => entry.track.id)).not.toContain(item.track.id);
    expect(after[1].items[0].track).toMatchObject({ id: item.track.id, path: item.track.path });
  });

  it("requires a current playlist preview before removal", async () => {
    const gateway = new DemoLibraryGateway();
    const playlist = (await gateway.listPlaylists())[0];
    const preview = await gateway.previewPlaylistRemoval(playlist.id);
    await gateway.renamePlaylist(playlist.id, "Changed after preview");

    await expect(gateway.removePlaylist(preview)).rejects.toThrow("Плейлист изменился");
    expect(await gateway.listPlaylists()).toHaveLength(2);
  });
});

describe("Tauri gateway IPC characterization", () => {
  beforeEach(() => {
    tauriMocks.invoke.mockReset();
    tauriMocks.convertFileSrc.mockClear();
    tauriMocks.open.mockReset();
    tauriMocks.save.mockReset();
  });

  it("maps the complete library query to the current command arguments", async () => {
    const gateway = new TauriLibraryGateway();
    const response: TrackPage = { items: [], total: 0, limit: 50, offset: 100 };
    tauriMocks.invoke.mockResolvedValue(response);
    const query: TrackQuery = {
      ...baseQuery,
      text: "acid",
      genre: "Techno",
      onlyIncoming: true,
      minBpm: 130,
      maxBpm: 150,
      minRating: 4,
      camelot: "4A",
      format: "flac",
      fileStatus: "available",
      limit: 50,
      offset: 100,
      sortBy: "artist",
      descending: false,
    };

    await expect(gateway.searchTracks(query)).resolves.toEqual(response);
    expect(tauriMocks.invoke).toHaveBeenCalledWith("search_tracks", {
      limit: 50,
      offset: 100,
      search: "acid",
      genre: "Techno",
      onlyIncoming: true,
      minBpm: 130,
      maxBpm: 150,
      minRating: 4,
      camelot: "4A",
      format: "flac",
      fileStatus: "available",
      sortBy: "artist",
      descending: false,
    });
  });

  it("keeps scan preview and commit arguments distinct", async () => {
    const gateway = new TauriLibraryGateway();
    const preview: ScanPreview = {
      rootPath: "D:\\Fixture",
      discovered: 12,
      recursive: true,
      exclusions: ["archive"],
      warnings: [],
    };
    tauriMocks.invoke.mockResolvedValueOnce(preview).mockResolvedValueOnce({ rootPath: preview.rootPath });

    await gateway.previewScanFolder(preview.rootPath, { recursive: true, exclusions: ["archive"] });
    await gateway.scanFolder(preview);

    expect(tauriMocks.invoke.mock.calls).toEqual([
      ["preview_music_folder", { path: "D:\\Fixture", options: { recursive: true, exclusions: ["archive"] } }],
      ["scan_music_folder", { path: "D:\\Fixture", options: { recursive: true, exclusions: ["archive"] }, expectedTotal: 12 }],
    ]);
  });

  it("preserves critical playlist, copy, export and metadata commit contracts", async () => {
    const gateway = new TauriLibraryGateway();
    tauriMocks.invoke.mockResolvedValue(undefined);

    await gateway.placeTrack({ targetPlaylistId: "target", targetPosition: 3, trackId: "track", itemId: "item", sourcePlaylistId: "source" });
    await gateway.executePlaylistCopy("copy-operation", [{ itemId: "copy-item", action: "rename" }]);
    await gateway.commitPlaylistExport({
      playlistId: "playlist",
      destinationPath: "D:\\Export\\playlist.nml",
      format: "traktor_nml",
      pathMode: "absolute",
      targetProfile: "traktor_pro_4",
    }, "playlist-state");
    await gateway.commitMetadataUpdate("track", "file-state", {
      title: "Title",
      artist: "Artist",
      album: null,
      albumArtist: null,
      trackNumber: null,
      discNumber: null,
      genre: "Techno",
      year: "2024",
      composer: null,
      comment: null,
      bpm: 145,
      musicalKey: "F minor",
      camelot: "4A",
    });

    expect(tauriMocks.invoke.mock.calls).toMatchInlineSnapshot(`
      [
        [
          "place_track_in_playlist",
          {
            "itemId": "item",
            "sourcePlaylistId": "source",
            "targetPlaylistId": "target",
            "targetPosition": 3,
            "trackId": "track",
          },
        ],
        [
          "execute_playlist_copy",
          {
            "operationId": "copy-operation",
            "resolutions": [
              {
                "action": "rename",
                "itemId": "copy-item",
              },
            ],
          },
        ],
        [
          "commit_playlist_export",
          {
            "expectedState": "playlist-state",
            "request": {
              "destinationPath": "D:\\Export\\playlist.nml",
              "format": "traktor_nml",
              "pathMode": "absolute",
              "playlistId": "playlist",
              "targetProfile": "traktor_pro_4",
            },
          },
        ],
        [
          "commit_metadata_update",
          {
            "expectedFileState": "file-state",
            "trackId": "track",
            "values": {
              "album": null,
              "albumArtist": null,
              "artist": "Artist",
              "bpm": 145,
              "camelot": "4A",
              "comment": null,
              "composer": null,
              "discNumber": null,
              "genre": "Techno",
              "musicalKey": "F minor",
              "title": "Title",
              "trackNumber": null,
              "year": "2024",
            },
          },
        ],
      ]
    `);
  });

  it("converts only a backend-approved playback path into an asset URL", async () => {
    const gateway = new TauriLibraryGateway();
    tauriMocks.invoke.mockResolvedValue("D:\\Fixture\\track.flac");

    await expect(gateway.preparePlayback("track-1")).resolves.toBe("asset://D:\\Fixture\\track.flac");
    expect(tauriMocks.invoke).toHaveBeenCalledWith("prepare_track_playback", { trackId: "track-1" });
    expect(tauriMocks.convertFileSrc).toHaveBeenCalledWith("D:\\Fixture\\track.flac");
  });

  it("normalizes the versioned Rust error and preserves its operation", async () => {
    const gateway = new TauriLibraryGateway();
    tauriMocks.invoke.mockRejectedValue({ schema: "sorttrack-error-1", code: "STALE_STATE", message: "playlist changed", retryable: false });

    const error = await gateway.removePlaylist({ playlistId: "playlist", name: "Fixture", itemCount: 1, expectedState: "state" as never }).catch((value) => value);

    expect(error).toBeInstanceOf(GatewayError);
    expect(error).toMatchObject({ code: "STALE_STATE", operation: "remove_playlist", message: "playlist changed" });
    expect(String(error)).toBe("playlist changed");
  });
});
