import { useState } from "react";
import type { Playlist, PlaylistExportFormat, PlaylistExportPathMode, PlaylistExportPreview, PlaylistExportReport, TraktorTargetProfile } from "../../../domain/models";

interface PlaylistExportDialogProps {
  playlist: Playlist;
  preview: PlaylistExportPreview | null;
  report: PlaylistExportReport | null;
  busy: boolean;
  onPreview: (format: PlaylistExportFormat, pathMode: PlaylistExportPathMode, targetProfile: TraktorTargetProfile | null) => void;
  onCommit: () => void;
  onReset: () => void;
  onClose: () => void;
}

const bytes = (value: number) => value < 1024 ? `${value} Б` : `${(value / 1024).toFixed(1)} КБ`;

export function PlaylistExportDialog({ playlist, preview, report, busy, onPreview, onCommit, onReset, onClose }: PlaylistExportDialogProps) {
  const [format, setFormat] = useState<PlaylistExportFormat>("m3u8");
  const [pathMode, setPathMode] = useState<PlaylistExportPathMode>("absolute");
  const [targetProfile, setTargetProfile] = useState<TraktorTargetProfile>("traktor_pro_4");
  return <div className="copy-overlay export-overlay" role="presentation">
    <section className="copy-dialog export-dialog" role="dialog" aria-modal="true" aria-labelledby="playlist-export-title">
      <header><div><span className="eyebrow">Отдельный файл</span><h2 id="playlist-export-title">{report ? "Экспорт завершён" : `Экспорт · ${playlist.name}`}</h2></div><button onClick={onClose} disabled={busy} aria-label="Закрыть экспорт">×</button></header>
      {!preview && !report && <>
        <div className="export-config">
          <label><span>Формат</span><select value={format} onChange={(event) => setFormat(event.target.value as PlaylistExportFormat)}><option value="m3u8">M3U8 — универсальный</option><option value="traktor_nml">Traktor NML</option></select></label>
          {format === "m3u8" ? <label><span>Пути к трекам</span><select value={pathMode} onChange={(event) => setPathMode(event.target.value as PlaylistExportPathMode)}><option value="absolute">Абсолютные</option><option value="relative">Относительно файла плейлиста</option></select></label> : <label><span>Профиль совместимости</span><select value={targetProfile} onChange={(event) => setTargetProfile(event.target.value as TraktorTargetProfile)}><option value="traktor_pro_4">Traktor Pro 4 · NML 19</option><option value="traktor_pro_3">Traktor Pro 3 · NML 19</option></select></label>}
        </div>
        <div className="recovery-message"><strong>Существующая коллекция не изменяется</strong><span>SortTrack создаст новый файл. Перезапись уже существующего M3U8 или NML запрещена.</span></div>
        <footer><div className="copy-safety"><strong>Сначала preview</strong><span>Порядок и каждый итоговый путь будут показаны до записи.</span></div><button className="secondary" onClick={onClose}>Отмена</button><button className="primary" onClick={() => onPreview(format, format === "traktor_nml" ? "absolute" : pathMode, format === "traktor_nml" ? targetProfile : null)} disabled={busy}>{busy ? "Подготовка…" : "Выбрать файл и проверить"}</button></footer>
      </>}
      {preview && !report && <>
        <div className="copy-destination"><span>Итоговый файл</span><strong title={preview.destinationPath}>{preview.destinationPath}</strong></div>
        <div className="copy-stats"><div><strong>{preview.totalTracks}</strong><span>треков</span></div><div className={preview.missingTracks ? "danger" : ""}><strong>{preview.missingTracks}</strong><span>отсутствует</span></div><div><strong>{preview.format === "m3u8" ? "M3U8" : "NML 19"}</strong><span>формат</span></div><div className={preview.destinationExists ? "danger" : ""}><strong>{preview.destinationExists ? "Занят" : "Свободен"}</strong><span>итоговый путь</span></div></div>
        {preview.destinationExists && <div className="export-warning">Файл уже существует. Вернитесь назад и выберите новое имя — перезапись запрещена.</div>}
        <div className="copy-items export-items">{preview.items.map((item) => <article className={`copy-item ${item.status}`} key={`${item.position}-${item.sourcePath}`}><span className="copy-status">{item.status === "ready" ? "Готов" : "Нет файла"}</span><span className="copy-track"><strong>{item.position + 1}. {item.artist} — {item.title}</strong><small title={item.outputLocation}>{item.outputLocation}</small><em>{item.reason}</em></span></article>)}</div>
        <footer><div className="copy-safety"><strong>{preview.missingTracks ? `Предупреждение: отсутствует ${preview.missingTracks}` : "Все пути проверены"}</strong><span>Отсутствующие записи сохраняются в плейлисте и явно показаны в отчёте.</span></div><button className="secondary" onClick={onReset} disabled={busy}>Изменить</button><button className="primary" onClick={onCommit} disabled={busy || preview.destinationExists}>{busy ? "Запись…" : "Создать файл"}</button></footer>
      </>}
      {report && <>
        <div className="copy-stats report"><div><strong>{report.totalTracks}</strong><span>экспортировано</span></div><div className={report.missingTracks ? "danger" : ""}><strong>{report.missingTracks}</strong><span>отсутствует</span></div><div><strong>{bytes(report.bytesWritten)}</strong><span>размер файла</span></div></div>
        <div className="export-result"><strong>{report.outputPath}</strong><p>{report.instructions}</p></div>
        <footer><div className="copy-safety"><strong>Файл создан атомарно</strong><span>Исходные треки и коллекция Traktor не изменялись.</span></div><button className="primary" onClick={onClose}>Закрыть отчёт</button></footer>
      </>}
    </section>
  </div>;
}
