import { describe, expect, it } from "vitest";
import fixture from "../../../docs/refactoring/ipc-contract-fixtures-v1.json";
import type { BulkMetadataRequest, MetadataValues, PlaylistExportRequest, ScanOptions } from "../../domain/models";

describe("Rust ↔ TypeScript IPC DTO fixtures", () => {
  it("keeps the versioned fixture schema explicit", () => {
    expect(fixture.schema).toBe("sorttrack-ipc-dto-fixtures-1");
  });

  it("matches the TypeScript request contracts and camelCase keys", () => {
    const scanOptions = fixture.scanOptions as ScanOptions;
    const exportRequest = fixture.playlistExportRequest as PlaylistExportRequest;
    const metadata = fixture.metadataValues as MetadataValues;
    const bulkRequest = fixture.bulkMetadataRequest as BulkMetadataRequest;

    expect(scanOptions).toEqual({ recursive: true, exclusions: ["archive", "temp/sets"] });
    expect(exportRequest).toMatchObject({ playlistId: "playlist-fixture", targetProfile: "traktor_pro_4" });
    expect(metadata).toMatchObject({ musicalKey: "F minor", camelot: "4A" });
    expect(bulkRequest).toMatchObject({ trackIds: ["track-a", "track-b"], fields: ["genre", "bpm"] });
    expect(exportRequest).not.toHaveProperty("playlist_id");
    expect(metadata).not.toHaveProperty("musical_key");
  });
});
