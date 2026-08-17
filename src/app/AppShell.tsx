import type { ReactNode } from "react";

export function AppShell({ children }: { children: ReactNode }) {
  return <div className="app-shell">{children}</div>;
}

export function AppContent({ children }: { children: ReactNode }) {
  return <main className="content">{children}</main>;
}
