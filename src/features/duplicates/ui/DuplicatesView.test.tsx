import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { DuplicateGroup } from "../../../domain/models";
import { DuplicatesView } from "./DuplicatesView";

describe("duplicates feature UI", () => {
  it("explains the non-destructive empty state", () => {
    const html = renderToStaticMarkup(<DuplicatesView groups={[]} lastScan={null} busy={false} onScan={() => undefined}/>);
    expect(html).toContain("ничего не удаляет");
    expect(html).toContain("Начать анализ");
  });

  it("shows the recommended retained copy and scan counters", () => {
    const group: DuplicateGroup = {
      sha256: "a".repeat(64),
      sizeBytes: 1_048_576,
      recommendedFileId: "flac",
      recommendationReason: "FLAC и полные теги",
      files: [
        { fileId: "flac", trackId: "t1", title: "Track", artist: "Artist", format: "FLAC", path: "C:/music/track.flac", sizeBytes: 1_048_576, availabilityStatus: "available", metadataCompleteness: 100 },
        { fileId: "mp3", trackId: "t2", title: "Track", artist: "Artist", format: "MP3", path: "C:/copy/track.mp3", sizeBytes: 1_048_576, availabilityStatus: "available", metadataCompleteness: 80 },
      ],
    };
    const html = renderToStaticMarkup(<DuplicatesView groups={[group]} lastScan={{ hashedFiles: 2, cachedFiles: 1, failedFiles: 0, candidateFiles: 2, groups: [group] }} busy={false} onScan={() => undefined}/>);
    expect(html).toContain("Рекомендуется оставить");
    expect(html).toContain("FLAC и полные теги");
    expect(html).toContain("Из кэша: 1");
  });
});
