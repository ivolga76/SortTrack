# R2 — gateway boundaries и IPC contracts

Дата завершения: 13 августа 2026 года.

## Результат

Этап завершён без переименования IPC-команд и без изменения схемы SQLite или пользовательских файлов.

## Предметные ports

Монолитный `LibraryGateway` заменён узкими интерфейсами в `src/shared/api/ports.ts`:

- `LibraryBrowsePort`;
- `TrackRatingPort`;
- `LibraryScanPort`;
- `PlaybackPort`;
- `PlaylistPort`;
- `FileCopyPort`;
- `PlaylistExportPort`;
- `AnalysisPort`;
- `DuplicatesPort`;
- `MetadataPort`;
- `SettingsPort`.

`AppGateway` используется только для composition/bootstrap. Deprecated-имя `LibraryGateway` пока является type-only alias для внешней совместимости, но feature-модули его больше не импортируют.

## Runtime adapters

- `src/adapters/demo/DemoLibraryGateway.ts` — автономная демонстрационная реализация без Tauri imports;
- `src/adapters/tauri/TauriLibraryGateway.ts` — единственное место production-кода с `invoke`, `convertFileSrc` и desktop dialogs;
- `src/adapters/bootstrap.ts` — явный выбор runtime и SSR-safe fallback в demo;
- `GatewayProvider` передаёт playback/cover port в entity UI без импорта adapter singleton.

Старый `services/libraryGateway.ts` удалён. `adapters/libraryGateway.ts` оставлен только как type/class compatibility facade и не содержит реализации или bootstrap state.

## Ошибки и optimistic concurrency

Rust сериализует `AppError` как `sorttrack-error-1` с полями `code`, `message`, `retryable`. Tauri adapter принимает новый объект и прежнюю строковую форму, преобразуя их в `GatewayError`; отображаемый пользователю текст сохранён.

Стабильные коды: `STALE_STATE`, `INVALID_INPUT`, `NOT_FOUND`, `UNAVAILABLE`, `FILESYSTEM`, `DATABASE`, `SERIALIZATION`, `TASK_FAILED`, `UNKNOWN`.

Значения `expectedState` остаются непрозрачными строками на IPC wire, но TypeScript различает branded-типы playlist removal/export, metadata file, relink и diagnostics. Это не позволяет случайно передать token от другой операции.

## Contract fixtures и ADR

- список IPC-команд по-прежнему проверяет `ipc-contract-v1.json`;
- `ipc-contract-fixtures-v1.json` одновременно десериализуется Rust-тестом и проверяется TypeScript-тестом;
- форма fixture покрывает scan options, Traktor export request, metadata values и bulk metadata request;
- решение об отложенной генерации DTO принято в `ADR-001-IPC-TYPE-STRATEGY.md`.

## Проверка

```powershell
npm.cmd run typecheck
npm.cmd test -- --run
npm.cmd run build
npm.cmd run ipc:check
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

Результат: 79/79 frontend-тестов и 74/74 Rust-теста; production build и Clippy успешны; IPC inventory полностью совпадает с R0 snapshot.

## Следующая граница

R3 переносит application use cases из Tauri commands: команды должны валидировать DTO, вызвать один use case и преобразовать результат/ошибку. Ports и error contract R2 являются стабильной внешней границей для этого переноса.
