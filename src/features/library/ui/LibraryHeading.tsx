export function LibraryHeading({ incomingOnly, total, reviewCount, genreBusy, onReviewGenres }: {
  incomingOnly: boolean;
  total: number;
  reviewCount: number;
  genreBusy: boolean;
  onReviewGenres: () => void;
}) {
  return <section className="page-heading">
    <div>
      <span className="eyebrow">{incomingOnly ? "Новые поступления" : "Локальная коллекция"}</span>
      <h1>{incomingOnly ? "Входящие" : "Библиотека"} <em>{total}</em></h1>
      <p>{incomingOnly ? "Прослушайте, проверьте метаданные и разберите новые треки." : "Поиск и сортировка выполняются в SQLite без загрузки всей коллекции."}</p>
    </div>
    <div className="status-card">
      <span className="status-dot"/>
      <div><strong>{reviewCount} требуют проверки</strong><small>Жанр предложен, решение за вами</small></div>
      <button onClick={onReviewGenres} disabled={genreBusy}>{genreBusy ? "Анализ…" : "Проверить жанры"}</button>
    </div>
  </section>;
}
