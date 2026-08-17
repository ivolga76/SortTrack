import type { AppGateway } from "../shared/api/ports";
import { createDemoGateway } from "./demo";
import { createTauriGateway } from "./tauri";

type RuntimeHost = { __TAURI_INTERNALS__?: unknown };

export function hasTauriRuntime(host?: RuntimeHost): boolean {
  return Boolean(host?.__TAURI_INTERNALS__);
}

export function createAppGateway(host: RuntimeHost | undefined = typeof window !== "undefined" ? window : undefined): AppGateway {
  return hasTauriRuntime(host) ? createTauriGateway() : createDemoGateway();
}

export const appGateway = createAppGateway();
