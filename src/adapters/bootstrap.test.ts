import { describe, expect, it } from "vitest";
import { DemoLibraryGateway } from "./demo";
import { TauriLibraryGateway } from "./tauri";
import { createAppGateway, hasTauriRuntime } from "./bootstrap";

describe("gateway bootstrap", () => {
  it("selects demo without relying on a browser global", () => {
    expect(hasTauriRuntime(undefined)).toBe(false);
    expect(createAppGateway(undefined)).toBeInstanceOf(DemoLibraryGateway);
  });

  it("selects the desktop adapter only for an explicit Tauri runtime", () => {
    const host = { __TAURI_INTERNALS__: {} };
    expect(hasTauriRuntime(host)).toBe(true);
    expect(createAppGateway(host)).toBeInstanceOf(TauriLibraryGateway);
  });
});
