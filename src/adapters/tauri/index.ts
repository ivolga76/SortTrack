import type { AppGateway } from "../../shared/api/ports";
import { TauriLibraryGateway } from "./TauriLibraryGateway";

export { TauriLibraryGateway } from "./TauriLibraryGateway";

export function createTauriGateway(): AppGateway {
  return new TauriLibraryGateway();
}
