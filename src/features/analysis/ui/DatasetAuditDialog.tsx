import type { GenreDatasetAudit, GenreDatasetExportReport, GenreModelEvaluation } from "../../../domain/models";

interface Props {
  audit: GenreDatasetAudit;
  report: GenreDatasetExportReport | null;
  evaluation: GenreModelEvaluation | null;
  busy: boolean;
  desktop: boolean;
  onExport: () => void;
  onEvaluate: () => void;
  onExportModelReport: () => void;
  onClose: () => void;
}

export function DatasetAuditDialog({ audit, report, evaluation, busy, desktop, onExport, onEvaluate, onExportModelReport, onClose }: Props) {
  const progress = Math.min(100, Math.round(audit.eligibleTracks / audit.targetTracks * 100));
  const maxClass = Math.max(1, ...audit.classes.map((item) => item.tracks));
  const confusionGenres = evaluation ? [...new Set(evaluation.confusion.flatMap((cell) => [cell.actual, cell.predicted]))].sort() : [];
  const confusionCount = (actual: string, predicted: string) => evaluation?.confusion.find((cell) => cell.actual === actual && cell.predicted === predicted)?.count ?? 0;
  return <div className="copy-overlay dataset-overlay" role="presentation"><section className="copy-dialog dataset-dialog" role="dialog" aria-modal="true" aria-labelledby="dataset-title">
    <header><div><span className="eyebrow">Подготовка аудио-ML</span><h2 id="dataset-title">Validation dataset</h2></div><button onClick={onClose} aria-label="Закрыть аудит датасета">×</button></header>
    <div className="dataset-summary"><div><strong>{audit.eligibleTracks} / {audit.targetTracks}</strong><span>зафиксированных и доступных треков</span></div><div className="dataset-progress"><i style={{ width: `${progress}%` }}/></div><em>{progress}% целевого объёма</em></div>
    <div className="dataset-facts"><span>Подтверждено: <strong>{audit.confirmedTracks}</strong></span><span>Зафиксировано: <strong>{audit.lockedTracks}</strong></span><span>Недоступно: <strong>{audit.unavailableTracks}</strong></span><span className={audit.ready ? "ready" : "pending"}>{audit.ready ? "Готов к фиксации manifest" : "Требуется доработка разметки"}</span></div>
    <section className="dataset-classes"><header><strong>Баланс широких жанров</strong><small>Поджанры оцениваются отдельно</small></header>{audit.classes.map((item) => <div className="dataset-class" key={item.genre}><span>{item.genre}</span><div><i style={{ width: `${item.tracks / maxClass * 100}%` }}/></div><strong>{item.tracks}</strong></div>)}</section>
    {audit.missingGenres.length > 0 && <div className="dataset-missing"><strong>Нет размеченных примеров:</strong> {audit.missingGenres.join(", ")}</div>}
    <div className="dataset-warnings">{audit.warnings.map((warning) => <span key={warning}>{warning}</span>)}</div>
    {report && <section className="dataset-report"><header><strong>Manifest создан</strong><span>{report.totalEntries} уникальных групп записей</span></header><div><span>train <strong>{report.trainEntries}</strong></span><span>calibration <strong>{report.calibrationEntries}</strong></span><span>test <strong>{report.testEntries}</strong></span></div><small>{report.outputPath}</small><code>SHA-256 {report.manifestSha256}</code><p>Схлопнуто повторов: {report.collapsedRecordings}; конфликтов разметки исключено: {report.conflictingRecordings}.</p></section>}
    {evaluation && <section className={`model-evaluation ${evaluation.meetsReleaseThreshold ? "passed" : "failed"}`}><header><div><strong>Top-1 accuracy {(evaluation.top1Accuracy * 100).toFixed(1)}%</strong><span>{evaluation.meetsReleaseThreshold ? "Критерий выпуска выполнен" : "Baseline не готов к выпуску"}</span></div><small>{evaluation.modelVersion} · {evaluation.featureVersion}</small></header><div className="model-splits"><span>train <strong>{evaluation.trainEntries}</strong></span><span>calibration <strong>{evaluation.calibrationEntries}</strong></span><span>test <strong>{evaluation.testEntries}</strong></span><span>ошибок признаков <strong>{evaluation.failedFeatures}</strong></span></div><section className="recall-list"><strong>Per-class recall</strong>{evaluation.classMetrics.map((metric) => <div key={metric.genre}><span>{metric.genre}</span><i><b style={{ width: `${metric.recall * 100}%` }}/></i><em>{(metric.recall * 100).toFixed(1)}% · {metric.correct}/{metric.support}</em></div>)}</section><div className="confusion-scroll"><table className="confusion-table"><caption>Confusion matrix: строки — эталон, столбцы — прогноз</caption><thead><tr><th>actual \ predicted</th>{confusionGenres.map((genre) => <th key={genre}>{genre}</th>)}</tr></thead><tbody>{confusionGenres.map((actual) => <tr key={actual}><th>{actual}</th>{confusionGenres.map((predicted) => <td className={actual === predicted ? "diagonal" : ""} key={predicted}>{confusionCount(actual, predicted) || "·"}</td>)}</tr>)}</tbody></table></div><div className="dataset-warnings">{evaluation.warnings.map((warning) => <span key={warning}>{warning}</span>)}</div></section>}
    {evaluation && <div className="dataset-model-actions"><button disabled={busy || !desktop} onClick={onExportModelReport}>Экспорт отчёта модели</button></div>}
    <footer><div className="copy-safety"><strong>Музыка не копируется</strong><span>Признаки кэшируются по снимку файла. Test используется только для финальной метрики и не участвует в обучении.</span></div><button onClick={onClose}>Закрыть</button><button disabled={busy} onClick={onEvaluate}>{busy ? "Извлечение признаков…" : "Оценить baseline"}</button><button className="primary" disabled={busy || !desktop} onClick={onExport}>{busy ? "Обработка…" : report ? "Создать другой manifest" : "Создать manifest"}</button></footer>
  </section></div>;
}
