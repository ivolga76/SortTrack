import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { AnalysisJob } from "../../../domain/models";
import { AnalysisQueueDialog } from "./AnalysisQueueDialog";

const noOp = () => undefined;

describe("analysis queue dialog", () => {
  it("disables enqueue for an empty selection and explains local processing", () => {
    const html = renderToStaticMarkup(<AnalysisQueueDialog jobs={[]} selectionCount={0} busy={false} onStart={noOp} onResume={noOp} onPause={noOp} onCancel={noOp} onClose={noOp}/>);
    expect(html).toContain("Аудио остаётся на компьютере");
    expect(html).toContain("disabled");
    expect(html).toContain("Очередь пока пуста");
  });

  it("renders recovery controls for an interrupted job", () => {
    const job: AnalysisJob = { id: "job", state: "recovery_required", totalItems: 10, processedItems: 4, failedItems: 0, processedBytes: 400, totalBytes: 1000, createdAt: "2026-08-13T00:00:00Z", error: null };
    const html = renderToStaticMarkup(<AnalysisQueueDialog jobs={[job]} selectionCount={1} busy={false} onStart={noOp} onResume={noOp} onPause={noOp} onCancel={noOp} onClose={noOp}/>);
    expect(html).toContain("Можно продолжить");
    expect(html).toContain("Восстановить");
    expect(html).toContain("width:40%");
  });
});
