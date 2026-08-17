import type { ReactNode } from "react";

const paths: Record<string, ReactNode> = {
  inbox: <path d="M4 5h16l-2 11h-4l-2 3-2-3H6L4 5Z" />,
  library: <><path d="M5 4v16"/><path d="M10 4v16"/><path d="m15 5 4-1 2 15-4 1-2-15Z"/></>,
  genre: <><circle cx="12" cy="12" r="8"/><circle cx="12" cy="12" r="2"/><path d="M12 4v3"/></>,
  playlist: <><path d="M5 6h10M5 11h10M5 16h6"/><path d="M18 13v7M18 20l3-2M18 20l-3-2"/></>,
  duplicate: <><rect x="7" y="7" width="12" height="12" rx="2"/><path d="M16 7V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h2"/></>,
  settings: <><circle cx="12" cy="12" r="3"/><path d="M19 12a7 7 0 0 0-.1-1l2-1-2-4-2 1a7 7 0 0 0-2-1l-.3-2h-5L9 6a7 7 0 0 0-2 1L5 6 3 10l2 1a7 7 0 0 0 0 2l-2 1 2 4 2-1a7 7 0 0 0 2 1l.5 2h5l.5-2a7 7 0 0 0 2-1l2 1 2-4-2-1a7 7 0 0 0 0-1Z"/></>,
  search: <><circle cx="10" cy="10" r="6"/><path d="m15 15 5 5"/></>,
  folder: <path d="M3 6h7l2 2h9v11H3V6Z" />,
  play: <path d="m9 6 9 6-9 6V6Z" />,
  pause: <><path d="M9 6v12M15 6v12"/></>,
  previous: <><path d="M6 6v12"/><path d="m18 7-8 5 8 5V7Z"/></>,
  next: <><path d="M18 6v12"/><path d="m6 7 8 5-8 5V7Z"/></>,
  volume: <><path d="M5 10v4h3l4 3V7l-4 3H5Z"/><path d="M15 9a4 4 0 0 1 0 6M17 6a8 8 0 0 1 0 12"/></>,
  mute: <><path d="M5 10v4h3l4 3V7l-4 3H5Z"/><path d="m16 10 4 4M20 10l-4 4"/></>,
  repeat: <><path d="m17 2 3 3-3 3"/><path d="M3 11V9a4 4 0 0 1 4-4h13"/><path d="m7 22-3-3 3-3"/><path d="M21 13v2a4 4 0 0 1-4 4H4"/></>,
  shuffle: <><path d="M3 6h3c5 0 7 12 12 12h3"/><path d="m18 15 3 3-3 3"/><path d="M3 18h3c2.5 0 4.2-3 5.8-6"/><path d="M15 6h6M18 3l3 3-3 3"/></>,
  more: <><circle cx="5" cy="12" r="1"/><circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/></>,
  export: <><path d="M12 3v12"/><path d="m7 10 5 5 5-5"/><path d="M5 17v3h14v-3"/></>,
};

export function Icon({ name, size = 20 }: { name: string; size?: number }) {
  return (
    <svg aria-hidden="true" width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round">
      {paths[name]}
    </svg>
  );
}
