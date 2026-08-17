import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { ScanResult } from "../../../domain/models";
import { ScanReportDialog } from "./ScanReportDialog";

const report = (values: Partial<ScanResult> = {}): ScanResult => ({
  rootPath: "C:/music",
  discovered: 8,
  added: 4,
  updated: 1,
  moved: 0,
  restored: 0,
  missing: 1,
  skipped: 2,
  errors: [],
  cancelled: false,
  ...values,
});

describe("scan report dialog", () => {
  it("shows completion statistics and the read-only audio guarantee", () => {
    const html = renderToStaticMarkup(<ScanReportDialog report={report()} onClose={() => undefined}/>);
    expect(html).toContain("Индексирование завершено");
    expect(html).toContain("Музыкальные файлы не изменялись");
    expect(html).toContain("Ошибок чтения не зафиксировано");
  });

  it("explains recovery after a safe cancellation", () => {
    const html = renderToStaticMarkup(<ScanReportDialog report={report({ cancelled: true })} onClose={() => undefined}/>);
    expect(html).toContain("Индексирование отменено безопасно");
    expect(html).toContain("Повторный scan продолжит индексирование");
  });
});
