# Участие в разработке KeyCraft

Благодарим за интерес к KeyCraft. Перед отправкой Pull Request ознакомьтесь с ключевыми правилами проекта в `.agents/rules/01-architecture.md` и `.agents/rules/02-anti-slop-ui.md`.

## Главные принципы

- **0% ИИ и сети:** никаких нейросетей, LLM, облачных API, HTTP-клиентов или телеметрии. Только быстрые детерминированные алгоритмы (`< 2 мс`).
- **Чистые границы крейтов:** `crates/core-engine` не должен содержать платформенных зависимостей (`windows`, `objc2`, `x11rb`). Вся логика ОС живёт в `crates/platform`.
- **Anti-Slop UI:** чистый CSS на Design Tokens, монохромные иконки `1.5px`, ноль эмодзи, ноль декоративных градиентов, спокойный и лаконичный русский интерфейс.

## Локальная проверка перед PR

```powershell
# 1. Проверка фронтенда
cd ui
npm.cmd run check
npm.cmd run build
cd ..

# 2. Линтинг и тесты Rust
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Если вы обновляли словарные списки в `data/wordlists/`, пересоберите бинарные индексы:

```powershell
cargo run -p keycraft-dict-builder --release
```

## Формат коммитов

Используйте [Conventional Commits](https://www.conventionalcommits.org/):
- `feat(core): ...`
- `fix(windows): ...`
- `ui(overlay): ...`
- `docs: ...`
- `test(core): ...`

## Портирование на macOS и Linux

Точка входа для новых ОС — trait `PlatformBackend` в `crates/platform/src/lib.rs`:
- `crates/platform/src/macos/mod.rs` — реализация через `CGEventTap`, `AXUIElement`, `IsSecureEventInputEnabled`, `TISSelectInputSource`, `ScreenCaptureKit`.
- `crates/platform/src/linux/mod.rs` — реализация для X11 (`XRecord`/`XTest`/`XkbLockGroup`/AT-SPI2) и Wayland (`xdg-desktop-portal`).
