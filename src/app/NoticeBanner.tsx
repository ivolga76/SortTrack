export function NoticeBanner({ message, onClose }: { message: string; onClose: () => void }) {
  return <div className="notice" role="status"><span>{message}</span><button onClick={onClose} aria-label="Закрыть уведомление">×</button></div>;
}
