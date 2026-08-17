import type { ScanResult } from "../../../domain/models";

export function scanCompletedMessage(result: ScanResult): string {
  if (result.cancelled) {
    return `Индексирование безопасно отменено после ${result.discovered} файлов. Уже добавленные записи сохранены; повторный scan продолжит работу.`;
  }
  return `Найдено ${result.discovered}; добавлено ${result.added}; обновлено ${result.updated}; перемещено ${result.moved}; вернулось ${result.restored}; отсутствует ${result.missing}; ошибок ${result.errors.length}.`;
}

export function rootEnabledMessage(enabled: boolean): string {
  return enabled
    ? "Папка подключена. Перед индексированием используйте «Проверить снова»."
    : "Папка отключена. Треки и музыкальные файлы сохранены без изменений.";
}
