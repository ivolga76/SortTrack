import type { AppGateway } from "../../shared/api/ports";
import { DemoLibraryGateway } from "./DemoLibraryGateway";

export { DemoLibraryGateway } from "./DemoLibraryGateway";

export function createDemoGateway(): AppGateway {
  return new DemoLibraryGateway();
}
