import { describe, expect, it } from "vitest";
import type { ScanResult } from "../../../domain/models";
import { rootEnabledMessage, scanCompletedMessage } from "./scanMessages";

function result(change: Partial<ScanResult> = {}): ScanResult {
  return {
    rootPath: "C:/music",
    discovered: 10,
    added: 4,
    updated: 2,
    moved: 1,
    restored: 1,
    skipped: 2,
    missing: 3,
    cancelled: false,
    errors: [{ path: "broken.mp3", reason: "invalid" }],
    ...change,
  };
}

describe("library scan messages", () => {
  it("reports every persistent scan outcome", () => {
    expect(scanCompletedMessage(result())).toBe("Найдено 10; добавлено 4; обновлено 2; перемещено 1; вернулось 1; отсутствует 3; ошибок 1.");
  });

  it("explains that a cancelled scan keeps committed index rows", () => {
    expect(scanCompletedMessage(result({ cancelled: true }))).toContain("Уже добавленные записи сохранены");
  });

  it("states the non-destructive root toggle guarantees", () => {
    expect(rootEnabledMessage(true)).toContain("Папка подключена");
    expect(rootEnabledMessage(false)).toContain("файлы сохранены без изменений");
  });
});
