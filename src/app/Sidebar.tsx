import { Icon } from "../shared/ui/Icon";
import type { LibrarySummary, Playlist } from "../domain/models";

const items = [
  ["inbox", "Входящие", "incoming"],
  ["library", "Все треки", "tracks"],
  ["genre", "Жанры", null],
  ["playlist", "Плейлисты", null],
  ["duplicate", "Дубликаты", "duplicates"],
] as const;

export function Sidebar({ summary, playlists, onOpenPlaylists, activeView, onOpenInbox, onOpenDuplicates, onOpenSettings }: {
  summary: LibrarySummary;
  playlists: Playlist[];
  onOpenPlaylists: () => void;
  activeView: "inbox" | "duplicates";
  onOpenInbox: () => void;
  onOpenDuplicates: () => void;
  onOpenSettings: () => void;
}) {
  return (
    <aside className="sidebar">
      <div className="brand"><span className="brand-mark">S</span><span>SortTrack</span></div>
      <nav className="nav" aria-label="Основная навигация">
        <div className="nav-caption">Библиотека</div>
        {items.map(([icon, label, count]) => (
          <button className={`nav-item ${(icon === "inbox" && activeView === "inbox") || (icon === "duplicate" && activeView === "duplicates") ? "active" : ""}`} key={label} onClick={icon === "playlist" ? onOpenPlaylists : icon === "duplicate" ? onOpenDuplicates : icon === "inbox" ? onOpenInbox : undefined}>
            <Icon name={icon} />
            <span>{label}</span>
            {count && <span className="nav-count">{summary[count].toLocaleString("ru-RU")}</span>}
          </button>
        ))}
        <div className="nav-caption with-top">Рабочее пространство</div>
        {playlists.slice(0, 5).map((playlist) => <button className="nav-item" key={playlist.id} onClick={onOpenPlaylists}><Icon name="playlist"/><span>{playlist.name}</span><span className="nav-count">{playlist.items.length}</span></button>)}
        {playlists.length === 0 && <button className="nav-item" onClick={onOpenPlaylists}><Icon name="playlist"/><span>Создать плейлист</span></button>}
      </nav>
      <div className="sidebar-footer">
        <button className="nav-item" onClick={onOpenSettings}><Icon name="settings"/><span>Настройки</span></button>
        <div className="storage"><span>Локальная библиотека</span><strong>{summary.tracks.toLocaleString("ru-RU")} треков</strong><div><i /></div></div>
      </div>
    </aside>
  );
}
