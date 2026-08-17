# R1 — frontend shell и feature boundaries

Дата завершения: 13 августа 2026 года.

## Статус

Этап завершён без изменения IPC, схемы SQLite и файловых операций.

## Что вынесено из `App.tsx`

- `app/AppShell.tsx` и `app/NoticeBanner.tsx` — стабильная композиция оболочки и уведомлений;
- `features/library/model` — запросы, фильтры, пагинация, защита от устаревших ответов, выбор строк и сохранение раскладки;
- `features/library/ui` — заголовок, фильтры, массовые действия, таблица и пагинация;
- `features/player/model` — текущий трек, playable queue, переходы, shuffle и repeat state;
- `features/player/ui` — постоянный слот плеера под оболочкой приложения;
- `features/playlists/model` — ручные и smart playlists, удаление, Copy/recovery и экспорт;
- `features/playlists/ui` — workspace и все связанные плейлистные диалоги.
- `features/duplicates` — загрузка сохранённых групп, exact SHA-256 scan и экран результата;
- `features/library-scan` — выбор папки, preview, progress polling, pause/resume/cancel, отчёт и управление сохранёнными корнями;
- `features/metadata` — единичное и массовое редактирование метаданных, preview/commit и relink;
- `features/analysis` — очередь BPM/key, жанровый помощник, dataset audit и оценка модели;
- `features/settings` — runtime profile, backup/restore и полностью локальная диагностика;
- `entities/track/ui` — единое представление обложки трека;
- `shared/ui` — независимая UI-примитива `Icon`;
- `adapters/libraryGateway.ts` — на момент R1 фактическая Tauri/demo граница; временный `services` re-export позднее удалён при завершении R2.

Композиция перенесена в `app/App.tsx` и уменьшена с 948 до 199 строк. Корневой `App.tsx` оставлен двухстрочным transitional entry-point facade. Внешний вид, gateway API и имена IPC-команд сохранены.

## Добавленная страховка

- unit tests для стабильного преобразования фильтров в `TrackQuery`;
- проверка всех 24 Camelot keys;
- component tests основных состояний UI библиотеки;
- unit tests playable queue плеера;
- unit tests детерминированных имён плейлистов и снимков;
- unit tests выбора треков для metadata/audio/genre операций;
- unit tests сообщений безопасного scan и component tests completed/cancelled scan report;
- component tests empty/recovery analysis queue и non-destructive duplicates view;
- request sequence guard: запоздавший результат поиска не заменяет более новый.

## Проверка среза

```powershell
npm.cmd run typecheck
npm.cmd test -- --run
npm.cmd run build
npm.cmd run ipc:check
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

Финальный результат: 69/69 frontend-тестов и 71/71 Rust-тестов проходят; production build собирается; IPC inventory совпадает с R0 snapshot.

## Критерии завершения

- созданы `app`, `features`, `entities`, `shared`, `adapters`;
- library, playlists, player и dialog orchestration имеют собственные controllers/UI boundaries;
- композиция приложения меньше целевого ориентира 250 строк;
- прямой Tauri `invoke` находится только в adapter;
- IPC snapshot, Rust safety tests и frontend characterization tests зелёные;
- пользовательская музыка и рабочая база при проверках не открывались и не изменялись.

Разделение монолитного gateway на предметные ports и удаление transitional `services` facade завершены в R2.
