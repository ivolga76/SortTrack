import { createContext, useContext, type ReactNode } from "react";
import type { AppGateway, PlaybackPort } from "./ports";

const GatewayContext = createContext<AppGateway | null>(null);

export function GatewayProvider({ gateway, children }: { gateway: AppGateway; children: ReactNode }) {
  return <GatewayContext.Provider value={gateway}>{children}</GatewayContext.Provider>;
}

export function usePlaybackPort(): PlaybackPort | null {
  return useContext(GatewayContext);
}
