# Выпуск SortTrack для Windows

## Зафиксированное окружение

- Windows x64;
- Node.js 24.16.0 (`.nvmrc` и GitHub Actions);
- npm 11.x с обязательным `npm ci` и `package-lock.json`;
- Rust 1.97.1 в GitHub Actions; локально используется установленный Windows stable toolchain;
- Cargo работает с `Cargo.lock` через `--locked`.

## Локальная проверка

```text
npm ci
npm run typecheck
npm test -- --run
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --locked
npm run release:build
npm run release:verify
```

Результат находится в `src-tauri/target/release/bundle/release`. Валидатор требует NSIS и portable ZIP, полный текст основной лицензии AGPL-3.0, проверяет все SHA-256, release manifest, SPDX 2.3, полное покрытие лицензиями, отсутствие локальных абсолютных путей и обязательный portable-маркер. Portable ZIP обязан содержать `LICENSE`; ZIP с пользовательской базой отклоняется.

Для детерминированных метаданных SBOM используется `SOURCE_DATE_EPOCH`; workflow задаёт значение `0`. Сам Windows executable пока не объявляется byte-reproducible: Tauri/NSIS и Windows toolchain могут добавлять недетерминированные данные. Checksums всегда рассчитываются после окончательной упаковки.

## GitHub Actions

- `CI` запускается для pull request, push в `main` и вручную. Права токена ограничены `contents: read`.
- `Build release artifacts` запускается только вручную, повторяет тесты, собирает оба варианта, валидирует каталог и сохраняет его как workflow artifact на 14 дней.
- Workflow не создаёт GitHub Release и ничего не публикует автоматически. Публикация остаётся отдельным осознанным действием после проверки release notes.
- Внешние GitHub Actions закреплены полными commit SHA. Dependabot еженедельно предлагает обновления NPM, Cargo и Actions; они проходят обычный CI до принятия.

## Подпись и обновления

Windows code-signing сертификат пока не настроен. В release notes необходимо явно предупреждать о возможном SmartScreen. Автоматические обновления запрещено включать без отдельного подписанного update manifest: HTTPS и checksum не заменяют криптографическую подпись обновления.

Portable обновляется только заменой файлов приложения после закрытия SortTrack. Папка `data` не входит в архив обновления и не должна удаляться или заменяться.
