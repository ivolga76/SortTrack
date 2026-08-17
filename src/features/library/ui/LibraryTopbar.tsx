import { Icon } from "../../../shared/ui/Icon";

export function LibraryTopbar({ query, scanning, onQueryChange, onAddFolder }: {
  query: string;
  scanning: boolean;
  onQueryChange: (query: string) => void;
  onAddFolder: () => void;
}) {
  return <header className="topbar">
    <div className="search">
      <Icon name="search"/>
      <input value={query} onChange={(event) => onQueryChange(event.target.value)} placeholder="Искать трек, исполнителя или путь…" aria-label="Поиск"/>
      <kbd>Ctrl K</kbd>
    </div>
    <button className="primary" onClick={onAddFolder} disabled={scanning}>
      <Icon name="folder"/>{scanning ? "Сканирование…" : "Добавить папку"}
    </button>
  </header>;
}
