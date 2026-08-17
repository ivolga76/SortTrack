export function LibraryPagination({ incomingOnly, offset, total, pageSize, onIncomingOnlyChange, onOffsetChange }: {
  incomingOnly: boolean;
  offset: number;
  total: number;
  pageSize: number;
  onIncomingOnlyChange: (incomingOnly: boolean) => void;
  onOffsetChange: (offset: number) => void;
}) {
  return <div className="bottom-note">
    <button onClick={() => onIncomingOnlyChange(!incomingOnly)}>{incomingOnly ? "Открыть всю библиотеку" : "Вернуться во входящие"}</button>
    <div className="pagination">
      <button disabled={offset === 0} onClick={() => onOffsetChange(Math.max(0, offset - pageSize))}>←</button>
      <span>{total ? `${Math.floor(offset / pageSize) + 1} / ${Math.ceil(total / pageSize)}` : "0 / 0"}</span>
      <button disabled={offset + pageSize >= total} onClick={() => onOffsetChange(offset + pageSize)}>→</button>
    </div>
  </div>;
}
