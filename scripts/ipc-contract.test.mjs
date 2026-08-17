import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
import { buildIpcInventory, currentIpcInventory, extractBackendCommands, extractFrontendCommands } from "./ipc-contract.mjs";

describe("IPC command inventory", () => {
  it("extracts commands from both adapter boundaries", () => {
    expect(extractFrontendCommands('invoke<void>("one"); invoke("two"); invoke<void>("one")')).toEqual(["one", "two"]);
    expect(extractBackendCommands(".invoke_handler(tauri::generate_handler![two, one])" )).toEqual(["one", "two"]);
  });

  it("reports asymmetric commands explicitly", () => {
    expect(buildIpcInventory('invoke<void>("shared"); invoke<void>("frontend_only")', ".invoke_handler(tauri::generate_handler![shared, backend_only])")).toMatchObject({
      frontendOnly: ["frontend_only"],
      backendOnly: ["backend_only"],
    });
  });

  it("matches the committed R0 command snapshot", () => {
    const snapshot = JSON.parse(fs.readFileSync(path.resolve("docs/refactoring/ipc-contract-v1.json"), "utf8"));
    expect(currentIpcInventory()).toEqual(snapshot);
    expect(snapshot.frontendOnly).toEqual([]);
    expect(snapshot.backendOnly).toEqual(["list_tracks"]);
  });
});
