// Transitional compatibility facade. Implementations are physically isolated by runtime.
export { DemoLibraryGateway } from "./demo";
export { TauriLibraryGateway } from "./tauri";
export type { AppGateway, LibraryGateway } from "../shared/api/ports";
