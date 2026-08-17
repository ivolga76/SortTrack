# ADR-001: стратегия типов и ошибок IPC

- Статус: принято
- Дата: 13 августа 2026 года
- Этап: R2

## Контекст

SortTrack использует TypeScript DTO во frontend и Rust DTO с `serde(rename_all = "camelCase")` в Tauri backend. Автоматическая генерация типов могла бы уменьшить ручное дублирование, но её немедленное внедрение связало бы текущий монолитный `models.rs` с frontend во время R2 и увеличило бы область изменений перед выделением backend application layer в R3.

Основные риски текущей схемы:

- незаметное расхождение camelCase/snake_case;
- смешивание предметных API в одном gateway;
- неструктурированные строковые ошибки;
- случайная передача `expectedState` от одной операции в другую.

## Решение

1. Frontend использует узкие предметные ports из `src/shared/api/ports.ts`. Полный `AppGateway` разрешён только в bootstrap/adapter composition.
2. Общие DTO пока остаются ручными, а совместимость Rust и TypeScript проверяется одним версионированным JSON fixture `ipc-contract-fixtures-v1.json`, который читают тесты обеих сторон.
3. Ошибка IPC имеет версионированную форму `sorttrack-error-1`:

   ```json
   {
     "schema": "sorttrack-error-1",
     "code": "STALE_STATE",
     "message": "...",
     "retryable": false
   }
   ```

4. Frontend adapter преобразует как новую структуру, так и старую строку в `GatewayError`. Пользовательский текст не меняется; feature может принимать решения по стабильному `code`.
5. Optimistic-concurrency значения остаются строками в JSON, но в TypeScript имеют несовместимые branded-типы по области: playlist removal, playlist export, metadata file, relink и diagnostics.
6. Имена IPC-команд по-прежнему защищены R0 inventory snapshot.

## Решение по генерации DTO

В R2 автоматическую генерацию не внедрять. Повторно оценить `specta`/`tauri-specta`, `ts-rs` либо JSON Schema после разделения `models.rs` в R3.

Условия перехода к генерации:

- Rust DTO находятся в interface/application boundary, а не в общем доменном файле;
- генератор воспроизводим и закреплён в lockfile;
- generated files проверяются CI на отсутствие diff;
- Tauri-specific типы не проникают в domain;
- fixtures для обратной совместимости сохраняются минимум на одну схему.

## Последствия

Положительные:

- features видят только необходимые операции;
- форма ошибок и optimistic state проверяется компилятором;
- расхождение ключевых DTO обнаруживается одновременно в Rust и TypeScript tests;
- R3 может менять внутреннюю backend-структуру без смены frontend contract.

Ограничения:

- часть DTO до R3 остаётся продублированной вручную;
- добавление DTO требует обновить fixture, если он относится к критическому сценарию.
